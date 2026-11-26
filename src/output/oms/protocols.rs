use super::DecodedOutput;
use crate::output::{decode_hex_bytes, RecordAvailability};
use m_bus_oms_rules::telegram::{ApplicationProtocolEvidence, DlmsAssociation};

pub(in crate::output) fn application_protocol(
    frame: &DecodedOutput,
) -> ApplicationProtocolEvidence {
    let mut evidence = ApplicationProtocolEvidence::default();
    let ci = frame.transport.application_ci.or(frame.transport.outer_ci);
    if let Some(ci @ (0x50 | 0x53..=0x57 | 0x66..=0x68)) = ci {
        if matches!(
            frame.record_availability,
            RecordAvailability::Encrypted | RecordAvailability::Fragmented
        ) || frame.security.mode_code.is_some_and(|mode| mode != 0)
        {
            return evidence;
        }
        evidence.selection_length = frame
            .application
            .as_ref()
            .filter(|app| {
                matches!(
                    app.protocol.as_str(),
                    "request_selected_application"
                        | "selected_application"
                        | "application_reset_or_select"
                )
            })
            .map(|app| app.payload_hex.len() / 2)
            .or_else(|| {
                let wire = frame.raw.payload_hex.as_deref()?;
                let header = match ci {
                    0x50 | 0x54 | 0x66 => 0,
                    0x56 | 0x57 | 0x67 => 4,
                    _ => 12,
                };
                (frame.transport.outer_ci == Some(ci))
                    .then(|| (wire.len() / 2).checked_sub(1 + header))?
            });
        return evidence;
    }
    let Some(application) = frame
        .application
        .as_ref()
        .filter(|app| matches!(app.protocol.as_str(), "sml" | "dlms_cosem"))
    else {
        return evidence;
    };
    let clear = frame.security.decrypted_payload_hex.as_deref().or_else(|| {
        (!matches!(
            frame.record_availability,
            RecordAvailability::Encrypted | RecordAvailability::Fragmented
        ))
        .then_some(application.payload_hex.as_str())
    });
    let Some(bytes) = clear.and_then(|hex| decode_hex_bytes(hex).ok()) else {
        return evidence;
    };
    if application.protocol == "dlms_cosem" {
        let bytes = if frame.security.decrypted_payload_hex.is_some()
            && frame.security.mode_code == Some(5)
        {
            bytes.strip_prefix(&[0x2f, 0x2f]).unwrap_or(&bytes)
        } else {
            &bytes
        };
        evidence.association = DlmsAssociation::from_mbus_payload(bytes);
    } else {
        #[cfg(feature = "oms-gen3")]
        {
            crate::output::sml::inspect(&bytes, &mut evidence);
        }
    }
    evidence
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Route {
    Wireless([u8; 8], [u8; 8]),
    Wired(u8, bool),
}

impl Route {
    fn reversed(self) -> Self {
        match self {
            Self::Wireless(from, to) => Self::Wireless(to, from),
            Self::Wired(address, master) => Self::Wired(address, !master),
        }
    }
}

#[cfg(feature = "oms-gen3")]
#[derive(Default)]
struct FileBlocks {
    next: u8,
    last: Option<(u8, usize)>,
    data: Vec<u8>,
}

#[derive(Default)]
pub(super) struct SmlTransfers {
    #[cfg(feature = "oms-gen3")]
    files: std::collections::BTreeMap<(Route, u8), FileBlocks>,
    previous: Option<(Route, m_bus_oms_rules::telegram::SmlBlock)>,
    maximum: std::collections::BTreeMap<(Route, u8), u16>,
}

impl SmlTransfers {
    #[cfg(feature = "oms-gen3")]
    fn assemble(
        &mut self,
        route: Route,
        block: m_bus_oms_rules::telegram::SmlBlock,
        data: &[u8],
        report: &mut super::OmsReport,
    ) {
        let key = (route, block.file);
        if block.control == 0 {
            self.files.retain(|(active, _), _| *active != route);
            self.files.insert(
                key,
                FileBlocks {
                    next: 1,
                    ..Default::default()
                },
            );
            return;
        }
        if block.control & 0x80 != 0 {
            return;
        }
        let number = block.control & 0x3f;
        let mut collected = self.files.remove(&key);
        if let Some(file) = &mut collected {
            if let Some((_, start)) = file.last.filter(|(last, _)| *last == number) {
                file.data.truncate(start);
            } else if number != file.next {
                collected = None;
            }
        }
        if let Some(file) = &mut collected {
            file.last = Some((number, file.data.len()));
            file.next = if number == 63 { 1 } else { number + 1 };
            file.data.extend_from_slice(data);
        }
        if block.control & 0x40 == 0 {
            if let Some(file) = collected {
                self.files.insert(key, file);
            }
        } else {
            let evidence = collected.map(|file| {
                let mut evidence = ApplicationProtocolEvidence::default();
                crate::output::sml::inspect_messages(&file.data, true, &mut evidence);
                evidence
            });
            block.check_file(evidence, report);
        }
    }

    pub(super) fn observe(
        &mut self,
        frame: &DecodedOutput,
        wireless: Option<m_bus_oms_rules::sequence::ExchangeFrame>,
        wired: Option<m_bus_oms_rules::wired::WiredObservation>,
        report: &mut super::OmsReport,
    ) {
        let previous = self.previous.take();
        if !frame
            .application
            .as_ref()
            .is_some_and(|app| app.protocol == "sml")
        {
            return;
        }
        let route = match (wireless, wired) {
            (Some(link), _) => match link.receiver {
                Some(to) => Route::Wireless(link.sender, to),
                None => match previous {
                    Some((Route::Wireless(from, to), _)) if link.sender == to => {
                        Route::Wireless(to, from)
                    }
                    _ => return,
                },
            },
            (
                _,
                Some(m_bus_oms_rules::wired::WiredObservation::Frame {
                    control,
                    address: address @ 0..=250,
                }),
            ) => Route::Wired(address, control & 0x40 != 0),
            _ => return,
        };
        #[cfg(feature = "oms-gen3")]
        let block = {
            let clear = frame.security.decrypted_payload_hex.as_deref().or_else(|| {
                if matches!(
                    frame.record_availability,
                    RecordAvailability::Encrypted | RecordAvailability::Fragmented
                ) {
                    return None;
                }
                Some(frame.application.as_ref()?.payload_hex.as_str())
            });
            let Some(bytes) = clear.and_then(|hex| decode_hex_bytes(hex).ok()) else {
                return;
            };
            let start = bytes.iter().position(|b| *b != 0x2f).unwrap_or(bytes.len());
            let Some((block, data)) = bytes
                .get(start..)
                .and_then(crate::output::sml::version_two_block)
            else {
                return;
            };
            self.assemble(route, block, &data, report);
            block
        };
        #[cfg(not(feature = "oms-gen3"))]
        let Some(Ok(block)) = application_protocol(frame).sml_v2
        else {
            return;
        };
        if block.control == 0 {
            self.maximum.remove(&(route, block.file));
        }
        if let Some((last_route, last)) = previous {
            if route == last_route.reversed() {
                last.check_reply(block, report);
                if last.control == 0 && block.control == 0x80 && last.file == block.file {
                    if let Some((_, limit)) = block.negotiation {
                        self.maximum.insert((last_route, block.file), limit);
                    }
                }
            }
        }
        if let Some(maximum) = self.maximum.get(&(route, block.file)) {
            block.check_size(*maximum, report);
        }
        self.previous = Some((route, block));
    }
}
