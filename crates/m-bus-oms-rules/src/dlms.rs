use crate::telegram::{RuleFinding, RuleStatus};

#[derive(Clone, Copy, Default)]
pub struct DlmsService {
    valid: Option<bool>,
    uplink: Option<bool>,
    protected: bool,
    invoke: Option<u8>,
}

impl DlmsService {
    #[cfg(feature = "dlms")]
    pub fn inspect(payload: &[u8], padded: bool) -> Self {
        use dlms_cosem_kit::{
            codec::{Decode, ErrorKind, Reader},
            xdlms::{Apdu, ApduTag},
        };
        let [_, _, data @ ..] = payload else {
            return Self {
                valid: Some(false),
                ..Self::default()
            };
        };
        let Some(tag) = data.first().copied().and_then(ApduTag::from_u8) else {
            return Self::default();
        };
        let mut protected = false;
        let mut current = data;
        let mut allow_padding = padded;
        let mut depth = 0;
        let valid = loop {
            let mut reader = Reader::new(current);
            let apdu = match Apdu::decode(&mut reader) {
                Ok(apdu) => apdu,
                Err(err)
                    if matches!(
                        err.kind,
                        ErrorKind::Unsupported | ErrorKind::WorkExceeded | ErrorKind::DepthExceeded
                    ) =>
                {
                    break None;
                }
                Err(_) => break Some(false),
            };
            if !(reader.is_empty() || (allow_padding && reader.rest().iter().all(|b| *b == 0x2f))) {
                break Some(false);
            }
            protected |= apdu.is_protected();
            let inner = match apdu {
                Apdu::Gateway { payload, .. } => Some(payload),
                Apdu::Aarq(message) => message.user_information,
                Apdu::Aare(message) => message.user_information,
                Apdu::Rlrq(message) => message.user_information,
                Apdu::Rlre(message) => message.user_information,
                _ => None,
            };
            let Some(inner) = inner else { break Some(true) };
            if inner.len() >= current.len() {
                break Some(false);
            }
            depth += 1;
            if depth > 32 {
                break None;
            }
            current = inner;
            allow_padding = false;
        };
        let plain = tag.unprotect().map_or(tag, |(_, plain)| plain);
        let uplink = match plain {
            ApduTag::Aarq
            | ApduTag::ReleaseRequest
            | ApduTag::InitiateRequest
            | ApduTag::ReadRequest
            | ApduTag::WriteRequest
            | ApduTag::UnconfirmedWriteRequest
            | ApduTag::GetRequest
            | ApduTag::SetRequest
            | ApduTag::ActionRequest
            | ApduTag::AccessRequest => Some(false),
            ApduTag::Aare
            | ApduTag::ReleaseResponse
            | ApduTag::InitiateResponse
            | ApduTag::ReadResponse
            | ApduTag::WriteResponse
            | ApduTag::GetResponse
            | ApduTag::SetResponse
            | ApduTag::ActionResponse
            | ApduTag::AccessResponse
            | ApduTag::EventNotificationRequest
            | ApduTag::InformationReportRequest => Some(true),
            _ => None,
        };
        Self {
            valid,
            uplink,
            protected,
            invoke: if matches!(
                tag,
                ApduTag::GetRequest
                    | ApduTag::SetRequest
                    | ApduTag::ActionRequest
                    | ApduTag::GetResponse
                    | ApduTag::SetResponse
                    | ApduTag::ActionResponse
            ) {
                data.get(2).copied()
            } else {
                None
            },
        }
    }

    pub(super) fn findings(self, ci: u8, results: &mut alloc::vec::Vec<RuleFinding>) {
        let mut check = |rule, valid: Option<bool>, detail| {
            results.push(RuleFinding {
                rule,
                reference: "§8.5 / DLMS UA Green Book, clause 9",
                source_url: primary!(78, 122),
                status: match valid {
                    Some(true) => RuleStatus::Passed,
                    Some(false) => RuleStatus::Failed,
                    None => RuleStatus::CouldNotCheck,
                },
                detail,
                record_number: None,
            })
        };
        check(
            "DLMS service APDU encoding",
            self.valid,
            "Checks the recognized service fields, nested typed data and complete APDU length after the M-Bus SAPs.",
        );
        if let Some(uplink) = self.uplink {
            check(
                "DLMS service direction",
                Some(uplink == matches!(ci, 0x7c | 0x7d)),
                "The M-Bus CI must agree with the DLMS client request or server response.",
            );
        }
        if let Some(invoke) = self.invoke {
            check(
                "DLMS invoke ID reserved bits",
                Some(invoke & 0x30 == 0),
                "The two reserved bits in invoke-id-and-priority must be zero.",
            );
        }
        if self.protected {
            check(
                "DLMS protected service contents",
                None,
                "The service is protected by the DLMS association. Its inner data needs association-specific security credentials.",
            );
        }
    }
}
