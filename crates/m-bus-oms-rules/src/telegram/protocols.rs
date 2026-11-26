use super::{RuleFinding, RuleStatus};
use alloc::vec::Vec;

#[derive(Clone, Copy, Default)]
pub struct ApplicationProtocolEvidence {
    pub sml_envelope: Option<bool>,
    pub sml_body: Option<bool>,
    pub sml_transport: Option<bool>,
    pub sml_file: Option<bool>,
    pub association: Option<DlmsAssociation>,
}

#[derive(Clone, Copy)]
pub struct DlmsAssociation {
    response: bool,
    length: Option<bool>,
    mandatory_fields: Option<bool>,
}

enum BerError {
    Malformed,
    Unsupported,
}

fn element(data: &[u8]) -> Result<(u8, &[u8], &[u8]), BerError> {
    let [tag, length, rest @ ..] = data else {
        return Err(BerError::Malformed);
    };
    if tag & 0x1f == 0x1f || *length == 0x80 {
        return Err(BerError::Unsupported);
    }
    let (len, rest) = if length & 0x80 == 0 {
        (usize::from(*length), rest)
    } else {
        if *length == 0xff {
            return Err(BerError::Malformed);
        }
        let (octets, body) = rest
            .split_at_checked(usize::from(length & 0x7f))
            .ok_or(BerError::Malformed)?;
        let n = octets
            .iter()
            .try_fold(0usize, |n, byte| {
                n.checked_mul(256)?.checked_add(usize::from(*byte))
            })
            .ok_or(BerError::Malformed)?;
        (n, body)
    };
    let (value, tail) = rest.split_at_checked(len).ok_or(BerError::Malformed)?;
    Ok((*tag, value, tail))
}

impl DlmsAssociation {
    pub fn from_mbus_payload(payload: &[u8]) -> Option<Self> {
        let [_, _, tag @ (0x60 | 0x61), ..] = payload else {
            return None;
        };
        let response = *tag == 0x61;
        let mut association = Self {
            response,
            length: None,
            mandatory_fields: None,
        };
        match element(&payload[2..]) {
            Ok((_, mut fields, trailing)) => {
                association.length = Some(trailing.iter().all(|b| *b == 0x2f));
                let mut context = false;
                let mut result = false;
                let mut diagnostic = false;
                while !fields.is_empty() {
                    match element(fields) {
                        Ok((tag, value, rest)) => {
                            match tag {
                                0xa1 => {
                                    context = matches!(element(value), Ok((6, oid, [])) if !oid.is_empty())
                                }
                                0xa2 => result = true,
                                0xa3 => diagnostic = true,
                                _ => {}
                            }
                            fields = rest;
                        }
                        Err(BerError::Malformed) => {
                            association.mandatory_fields = Some(false);
                            return Some(association);
                        }
                        Err(BerError::Unsupported) => return Some(association),
                    }
                }
                association.mandatory_fields = Some(context && (!response || result && diagnostic));
            }
            Err(BerError::Malformed) => association.length = Some(false),
            Err(BerError::Unsupported) => {}
        }
        Some(association)
    }
}

impl ApplicationProtocolEvidence {
    pub(super) fn check(self, ci: u8, rows: &mut Vec<RuleFinding>) {
        let mut record = |rule, reference, url, valid, detail| {
            rows.push(RuleFinding {
                rule,
                reference,
                source_url: url,
                status: match valid {
                    Some(true) => RuleStatus::Passed,
                    Some(false) => RuleStatus::Failed,
                    None => RuleStatus::CouldNotCheck,
                },
                detail,
                record_number: None,
            })
        };
        if matches!(ci, 0x64 | 0x65 | 0x7e | 0x7f) {
            record(
                "SML message structure and checksum",
                "§8.6 / SML specification",
                primary!(78, 212),
                self.sml_envelope,
                "Checks the six-field message envelope and the CRC of each readable SML binary message.",
            );
            record(
                "SML message body schema",
                "§8.6 / SML 1.04, §5.1",
                primary!(78, 212),
                self.sml_body,
                "Checks the field count and top-level field types of the recognized SML request and response bodies.",
            );
            if let Some(valid) = self.sml_transport {
                record(
                    "SML version 1 transport",
                    "§8.6 / SML 1.04, §8.1",
                    primary!(78, 212),
                    Some(valid),
                    "The escaped frame must have valid padding, a complete end marker and a matching transport CRC.",
                );
            }
            if let Some(valid) = self.sml_file {
                record(
                    "SML file open and close",
                    "§8.6 / SML 1.04, §5.1.1–5.1.4",
                    primary!(78, 212),
                    Some(valid),
                    "A complete SML request or response file starts and ends with the corresponding public open and close messages.",
                );
            }
        } else if let Some(association) = self.association {
            record(
                "DLMS association direction",
                "§8.5 / Table 1 / ITU-T X.227",
                primary!(78, 122),
                Some(association.response == matches!(ci, 0x7c | 0x7d)),
                "The association request travels to the meter and the association response travels to the client.",
            );
            record(
                "DLMS association BER length",
                "§8.5 / ITU-T X.690",
                primary!(78, 122),
                association.length,
                "The definite BER length must match the association APDU inside the M-Bus wrapper.",
            );
            record(
                "DLMS association fields",
                "§8.5 / ITU-T X.227",
                primary!(78, 122),
                association.mandatory_fields,
                "An association carries an application-context OID. The response also carries its result and diagnostic.",
            );
        } else {
            record(
                "DLMS application payload",
                "§8.5",
                primary!(78, 122),
                None,
                "An association APDU is needed to assess its fields. Other COSEM services require a service-specific decoder.",
            );
        }
    }
}
