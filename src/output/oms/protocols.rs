use super::DecodedOutput;
use crate::output::{decode_hex_bytes, RecordAvailability};
use m_bus_oms_rules::telegram::{ApplicationProtocolEvidence, DlmsAssociation};

pub(in crate::output) fn application_protocol(
    frame: &DecodedOutput,
) -> ApplicationProtocolEvidence {
    let mut evidence = ApplicationProtocolEvidence::default();
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

#[derive(Default)]
pub(super) struct SmlTransfers {
    previous: Option<(Route, m_bus_oms_rules::telegram::SmlBlock)>,
    maximum: std::collections::BTreeMap<(Route, u8), u16>,
}

impl SmlTransfers {
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
        let Some(Ok(block)) = application_protocol(frame).sml_v2 else {
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
