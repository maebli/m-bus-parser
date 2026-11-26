#[cfg(feature = "decryption-mode-7")]
use super::OutputError;
use super::{DecodedOutput, OmsReport};
use m_bus_oms_rules::firmware::ImageTransfer;
use serde::Deserialize;
use serde_json::Value;
#[cfg(feature = "decryption-mode-7")]
mod capture;
#[cfg(feature = "decryption-mode-7")]
pub use capture::verify_captured_firmware;

pub(super) fn decode(data: &[u8]) -> Value {
    let mut left = data;
    let mut segments = Vec::new();
    while let Some((size, _)) = left.split_first_chunk::<2>() {
        let length = usize::from(u16::from_le_bytes(*size));
        if length < 4 {
            break;
        }
        let Some((segment, next)) = left.split_at_checked(length) else {
            break;
        };
        let [_, _, id, body @ ..] = segment else {
            break;
        };
        let Some(mut transfer) = ImageTransfer::parse(body) else {
            break;
        };
        transfer.segment_id = Some(*id);
        transfer.segment_length = Some(u16::from_le_bytes(*size));
        let mut fields = serde_json::json!(transfer);
        if let Some(bytes) = transfer
            .data_length
            .and_then(|n| body.get(body.len().checked_sub(n)?..))
        {
            if let Some(obj) = fields.as_object_mut() {
                obj.insert("image_block_hex".into(), super::hex_string(bytes).into());
            }
        }
        segments.push(fields);
        left = next;
    }
    let complete = left.is_empty() && !segments.is_empty();
    if segments.len() == 1 && complete {
        return segments.pop().unwrap_or_default();
    }
    serde_json::json!({"segments":segments,"segments_complete":complete})
}

fn segments(value: &Value) -> &[Value] {
    value["segments"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(value))
}

pub(super) fn assess(frame: &DecodedOutput, report: &mut OmsReport) {
    let Some(app) = &frame.application else {
        return;
    };
    let Some(decoded) = &app.decoded else {
        return;
    };
    let wrapped = app.protocol == "security_information_transfer";
    let command = matches!(app.control_information.as_str(), "C0" | "C7" | "C3" | "C6");
    let blocks = decoded["blocks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(decoded));
    for block in blocks {
        let Some(value) = block.get("image_transfer") else {
            continue;
        };
        m_bus_oms_rules::firmware::check_segments(
            value["segments_complete"].as_bool().unwrap_or(true),
            report,
        );
        for segment in segments(value) {
            let Ok(firmware) = ImageTransfer::deserialize(segment) else {
                continue;
            };
            let wrapper = (|| {
                let expected_time = if firmware.function & 0x7f == 6 {
                    "0000008030"
                } else {
                    "0000000030"
                };
                Some((
                    block.get("block_control")?.as_u64()? as u8,
                    block.get("key_id")?.as_u64()? as u8,
                    block.get("data_selector")?.as_u64()? as u8,
                    block.get("target_time_hex")?.as_str()? == expected_time,
                ))
            })();
            firmware.assess(wrapped, wrapper, command, report);
        }
    }
}

#[cfg(feature = "decryption-mode-7")]
impl DecodedOutput {
    pub fn verify_firmware_image(
        &mut self,
        image: &[u8],
        image_key: &[u8; 16],
    ) -> Result<(), OutputError> {
        let Some(decoded) = self
            .application
            .as_mut()
            .and_then(|app| app.decoded.as_mut())
        else {
            return Ok(());
        };
        let verify_segment = |fields: &mut Value| {
            if fields["function"] != 0
                || fields["mac_algorithm"] != 4
                || fields["mac_key_id"] != 0x18
            {
                return;
            }
            if let Ok(tag) = serde_json::from_value::<[u8; 16]>(fields["mac"].clone()) {
                fields["firmware_cmac_matches"] =
                    m_bus_core::decryption::verify_firmware_cmac(image, &tag, image_key).into();
            }
        };
        let verify = |block: &mut Value| {
            let Some(fields) = block.get_mut("image_transfer") else {
                return;
            };
            if let Some(segments) = fields.get_mut("segments").and_then(Value::as_array_mut) {
                segments.iter_mut().for_each(verify_segment);
            } else {
                verify_segment(fields);
            }
        };
        if let Some(blocks) = decoded.get_mut("blocks").and_then(Value::as_array_mut) {
            blocks.iter_mut().for_each(verify);
        } else {
            verify(decoded);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MeterAddress {
    Secondary([u8; 8]),
    Wired(u8),
}

fn meter_address(
    frame: &DecodedOutput,
    app: &super::ApplicationOutput,
    command: bool,
) -> Option<MeterAddress> {
    frame
        .application_meter(app, command)
        .map(MeterAddress::Secondary)
        .or_else(|| match super::oms::wired_observation(frame)? {
            m_bus_oms_rules::wired::WiredObservation::Frame { address, .. } if address <= 250 => {
                Some(MeterAddress::Wired(address))
            }
            _ => None,
        })
}

#[derive(PartialEq, Eq)]
struct PendingSegment {
    id: u8,
    function: u8,
    image: Option<[u8; 8]>,
}

#[derive(Default)]
pub(super) struct History {
    sizes: std::collections::BTreeMap<(MeterAddress, [u8; 8]), (u16, u16)>,
    last_request: std::collections::BTreeMap<MeterAddress, Vec<PendingSegment>>,
}

impl History {
    pub(super) fn observe(
        &mut self,
        frame: &DecodedOutput,
        report: &mut OmsReport,
        consecutive: bool,
    ) {
        let Some(app) = &frame.application else {
            return;
        };
        let command = matches!(app.control_information.as_str(), "C0" | "C7" | "C3" | "C6");
        if !command && !matches!(app.control_information.as_str(), "C1" | "C2" | "C4" | "C5") {
            return;
        }
        let Some(decoded) = &app.decoded else {
            return;
        };
        let address = meter_address(frame, app, command);
        let previous = address.and_then(|peer| self.last_request.remove(&peer));
        let mut requests = Vec::new();
        let blocks = decoded["blocks"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| core::slice::from_ref(decoded));
        for block in blocks {
            let Some(value) = block.get("image_transfer") else {
                continue;
            };
            for segment in segments(value) {
                let Ok(transfer) = ImageTransfer::deserialize(segment) else {
                    continue;
                };
                if let Some(id) = transfer.segment_id {
                    let pending = PendingSegment {
                        id,
                        function: transfer.function & 0x7f,
                        image: transfer.image_identifier,
                    };
                    if command {
                        requests.push(pending);
                    } else {
                        let matches = previous.as_ref().filter(|_| consecutive).map(|requests| {
                            transfer.function >= 0x80 && requests.contains(&pending)
                        });
                        m_bus_oms_rules::firmware::check_reply(matches, report);
                    }
                }
                if !command {
                    continue;
                }
                let peer = address.zip(transfer.image_identifier);
                if transfer.function == 0 {
                    let Some(peer) = peer else {
                        continue;
                    };
                    self.sizes.remove(&peer);
                    if let Some((count, size)) = transfer
                        .total_blocks
                        .zip(transfer.block_size)
                        .filter(|(count, size)| {
                            transfer.structure_complete && *count > 0 && *size > 0
                        })
                    {
                        self.sizes.insert(peer, (count, size));
                    }
                } else if transfer.function == 2 {
                    let size = peer
                        .as_ref()
                        .and_then(|k| self.sizes.get(k))
                        .copied()
                        .filter(|_| consecutive);
                    transfer.check_block_dimensions(size, report);
                }
            }
        }
        if let Some(peer) = address.filter(|_| !requests.is_empty()) {
            self.last_request.insert(peer, requests);
        }
    }
}
