use super::{
    meter_address, segments, DecodedOutput, ImageTransfer, MeterAddress, OutputError, Value,
};
use m_bus_oms_rules::firmware::CapturedImage;
use serde::Deserialize;
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(PartialEq, Eq)]
struct Declaration {
    blocks: u16,
    block_size: u16,
    tag: [u8; 16],
    key_version: Option<u8>,
}

impl Declaration {
    fn read(transfer: &ImageTransfer) -> Option<Self> {
        if transfer.function != 0
            || !transfer.structure_complete
            || transfer.mac_algorithm != Some(4)
            || transfer.mac_key_id != Some(0x18)
        {
            return None;
        }
        Some(Self {
            blocks: transfer.total_blocks?,
            block_size: transfer.block_size?,
            tag: transfer.mac?,
            key_version: transfer.mac_key_version,
        })
    }
}

#[derive(Default)]
struct Transfer {
    prepare: Option<Declaration>,
    blocks: BTreeMap<u16, Vec<u8>>,
    conflicting: bool,
}

impl Transfer {
    fn observe(&mut self, transfer: &ImageTransfer, fields: &Value) -> Result<(), OutputError> {
        if let Some(declaration) = Declaration::read(transfer) {
            if let Some(previous) = &self.prepare {
                self.conflicting |= *previous != declaration;
            } else {
                self.prepare = Some(declaration);
            }
        } else if transfer.function == 2 && transfer.structure_complete {
            if let (Some(index), Some(hex)) =
                (transfer.block_number, fields["image_block_hex"].as_str())
            {
                let bytes = crate::output::decode_hex_bytes(hex)?;
                match self.blocks.entry(index) {
                    Entry::Vacant(entry) => {
                        entry.insert(bytes);
                    }
                    Entry::Occupied(entry) => {
                        self.conflicting |= *entry.get() != bytes;
                    }
                }
            }
        }
        Ok(())
    }
    fn verify(&self, key: &[u8; 16]) -> CapturedImage {
        if self.conflicting {
            return CapturedImage::Conflicting;
        }
        let Some(prepare) = &self.prepare else {
            return CapturedImage::Missing;
        };
        if prepare.blocks == 0
            || prepare.block_size == 0
            || self.blocks.iter().any(|(&i, data)| {
                i >= prepare.blocks || data.len() != usize::from(prepare.block_size)
            })
        {
            return CapturedImage::Dimensions;
        }
        if self.blocks.len() != usize::from(prepare.blocks) {
            return CapturedImage::Missing;
        }
        if m_bus_core::decryption::verify_firmware_blocks(
            self.blocks.values().map(Vec::as_slice),
            &prepare.tag,
            key,
        ) {
            CapturedImage::Verified
        } else {
            CapturedImage::Mismatch
        }
    }
}

fn visit_mut(decoded: &mut Value, mut visit: impl FnMut(&mut Value)) {
    let mut block = |value: &mut Value| {
        let Some(value) = value.get_mut("image_transfer") else {
            return;
        };
        if let Some(list) = value.get_mut("segments").and_then(Value::as_array_mut) {
            list.iter_mut().for_each(&mut visit);
        } else {
            visit(value);
        }
    };
    if let Some(list) = decoded.get_mut("blocks").and_then(Value::as_array_mut) {
        list.iter_mut().for_each(&mut block);
    } else {
        block(decoded);
    }
}

pub fn verify_captured_firmware(
    frames: &mut [DecodedOutput],
    key: &[u8; 16],
) -> Result<(), OutputError> {
    let mut transfers: BTreeMap<(MeterAddress, [u8; 8]), Transfer> = BTreeMap::new();
    for frame in frames.iter() {
        let Some(app) = &frame.application else {
            continue;
        };
        if !matches!(app.control_information.as_str(), "C0" | "C7" | "C3" | "C6") {
            continue;
        }
        let Some(peer) = meter_address(frame, app, true) else {
            continue;
        };
        let Some(decoded) = &app.decoded else {
            continue;
        };
        let blocks = decoded["blocks"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| std::slice::from_ref(decoded));
        for image in blocks
            .iter()
            .filter_map(|block| block.get("image_transfer"))
        {
            if image["segments_complete"] == false {
                continue;
            }
            for fields in segments(image) {
                let Ok(transfer) = ImageTransfer::deserialize(fields) else {
                    continue;
                };
                if let Some(id) = transfer.image_identifier {
                    transfers
                        .entry((peer, id))
                        .or_default()
                        .observe(&transfer, fields)?;
                }
            }
        }
    }
    let outcomes: BTreeMap<_, _> = transfers
        .into_iter()
        .map(|(id, transfer)| {
            let outcome = transfer.verify(key);
            (id, (outcome, transfer.prepare))
        })
        .collect();
    for frame in frames {
        let Some(app) = &frame.application else {
            continue;
        };
        if !matches!(app.control_information.as_str(), "C0" | "C7" | "C3" | "C6") {
            continue;
        }
        let peer = meter_address(frame, app, true);
        let Some(decoded) = frame
            .application
            .as_mut()
            .and_then(|app| app.decoded.as_mut())
        else {
            continue;
        };
        visit_mut(decoded, |fields| {
            let Ok(transfer) = ImageTransfer::deserialize(&*fields) else {
                return;
            };
            if transfer.function != 0
                || transfer.mac_algorithm != Some(4)
                || transfer.mac_key_id != Some(0x18)
            {
                return;
            }
            let outcome = match peer {
                None => CapturedImage::UnknownMeter,
                Some(peer) => match transfer
                    .image_identifier
                    .and_then(|id| outcomes.get(&(peer, id)))
                {
                    Some((CapturedImage::Conflicting, _)) => CapturedImage::Conflicting,
                    Some((outcome, Some(declaration)))
                        if Declaration::read(&transfer).as_ref() == Some(declaration) =>
                    {
                        *outcome
                    }
                    _ => CapturedImage::Missing,
                },
            };
            if let Some(obj) = fields.as_object_mut() {
                obj.insert("captured_image".into(), serde_json::json!(outcome));
            }
        });
    }
    Ok(())
}
