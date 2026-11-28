use crate::telegram::{RuleFinding, RuleStatus};

#[derive(Clone, Copy, Debug, Default)]
pub struct DlmsService {
    valid: Option<bool>,
    uplink: Option<bool>,
    protected: bool,
    invoke: Option<u8>,
}

impl DlmsService {
    #[cfg(feature = "dlms")]
    pub fn inspect(payload: &[u8], padded: bool) -> Self {
        m_bus_application_layer::dlms::DlmsPayload::decode(payload, padded).into()
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

#[cfg(feature = "dlms")]
impl From<m_bus_application_layer::dlms::DlmsPayload<'_>> for DlmsService {
    fn from(parsed: m_bus_application_layer::dlms::DlmsPayload<'_>) -> Self {
        Self {
            valid: parsed.valid,
            uplink: parsed.uplink,
            protected: parsed.protected,
            invoke: parsed.invoke,
        }
    }
}
