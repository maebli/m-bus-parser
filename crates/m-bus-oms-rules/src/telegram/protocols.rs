use super::{RuleFinding, RuleStatus};
use alloc::vec::Vec;

macro_rules! sml_page {
    ($page:literal, $height:literal) => {
        concat!("https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR03109/TR-03109-1_Anlage_Feinspezifikation_Drahtgebundene_LMN-Schnittstelle_Teilb.pdf?__blob=publicationFile&v=1#page=", stringify!($page), "&view=FitH,", stringify!($height))
    };
}

#[derive(Clone, Copy, Default)]
pub struct ApplicationProtocolEvidence {
    pub sml_envelope: Option<bool>,
    pub sml_body: Option<bool>,
    pub sml_transport: Option<bool>,
    pub sml_file: Option<bool>,
    pub sml_file_groups: Option<bool>,
    pub sml_file_address: Option<bool>,
    pub sml_v2: Option<Result<SmlBlock, ()>>,
    pub association: Option<DlmsAssociation>,
    pub dlms_service: Option<crate::dlms::DlmsService>,
    pub selection_length: Option<usize>,
}

pub(crate) fn selected_application(parameters: &[u8]) -> Option<(u16, u64)> {
    if parameters.is_empty() || parameters.len() > 10 {
        return None;
    }
    let mut application = 0;
    let mut extended = true;
    let mut block = 0;
    for (position, byte) in parameters.iter().enumerate() {
        let selector = byte >> 4;
        if extended {
            application += u16::from(selector);
            extended = selector == 15;
        }
        block |= u64::from(byte & 15) << (4 * position);
    }
    Some((application, block))
}

#[derive(Clone, Copy)]
pub struct SmlBlock {
    pub control: u8,
    pub file: u8,
    pub negotiation: Option<(u16, u16)>,
    pub payload_len: usize,
    pub wire_len: usize,
}

impl SmlBlock {
    pub fn check_file(
        self,
        file: Option<ApplicationProtocolEvidence>,
        report: &mut super::TelegramReport,
    ) {
        for (name, result) in [
            (
                "Reassembled SML message checksums",
                file.and_then(|f| f.sml_envelope),
            ),
            (
                "Reassembled SML message bodies",
                file.and_then(|f| f.sml_body),
            ),
            (
                "Reassembled SML file open and close",
                file.and_then(|f| f.sml_file),
            ),
        ] {
            let mut finding = sml_transfer_finding(
                name,
                result == Some(true),
                "Checks the SML file assembled from the observed sequence of data blocks. Missing blocks or unsupported fields leave the result unknown.",
            );
            if result.is_none() {
                finding.status = RuleStatus::CouldNotCheck;
            }
            report.push(finding);
        }
        if let Some(evidence) = file {
            for (name, valid) in [
                ("Reassembled SML message groups", evidence.sml_file_groups),
                (
                    "Reassembled SML request addressing",
                    evidence.sml_file_address,
                ),
            ] {
                if let Some(valid) = valid {
                    report.push(sml_transfer_finding(name, valid, "Checks the group sequence or server addressing of the reconstructed SML file."));
                }
            }
        }
    }

    pub fn check_size(self, maximum: u16, report: &mut super::TelegramReport) {
        if self.negotiation.is_some() || self.control & 0x80 != 0 {
            return;
        }
        let mut finding = sml_transfer_finding(
            "SML negotiated block size",
            self.payload_len <= usize::from(maximum),
            "The transmitted block must fit the negotiated maximum. The reported size includes a separate transport envelope.",
        );
        if self.payload_len <= usize::from(maximum) && self.wire_len > usize::from(maximum) {
            finding.status = RuleStatus::CouldNotCheck;
            finding.detail = "The payload fits but its transport envelope exceeds the limit. The block-size accounting convention is needed.";
        }
        report.push(finding);
    }
    pub fn check_reply(self, reply: Self, rows: &mut super::TelegramReport) {
        if self.control & 0x80 != 0 || reply.control & 0x80 == 0 {
            return;
        }
        let acknowledged = self.control | 0x80 == reply.control;
        let block = self.control & 0x3f;
        let previous = block.saturating_sub(1);
        let rejected =
            block != 0 && (reply.control == 0x80 | previous || block == 1 && reply.control == 0xbf);
        let valid = self.file == reply.file && (acknowledged || rejected);
        rows.push(sml_transfer_finding("SML block acknowledgement", valid,
            "The reply must identify the same file and either acknowledge this block or reject it with the preceding block number."));
        if let (Some((timeout, size)), Some((chosen_timeout, chosen_size))) =
            (self.negotiation, reply.negotiation)
        {
            rows.push(sml_transfer_finding("SML negotiated transfer limits", chosen_timeout >= timeout && chosen_size <= size,
                "A receiver may increase the proposed timeout and decrease the proposed maximum block size."));
        }
    }
}

fn sml_transfer_finding(rule: &'static str, valid: bool, detail: &'static str) -> RuleFinding {
    RuleFinding {
        rule,
        reference: "§8.6 / SML 1.04, §8.2",
        source_url: sml_page!(69, 551),
        status: if valid {
            RuleStatus::Passed
        } else {
            RuleStatus::Failed
        },
        detail,
        record_number: None,
    }
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
        if matches!(ci, 0x50 | 0x53..=0x57 | 0x66..=0x68) {
            let query = matches!(ci, 0x54..=0x56);
            let response = matches!(ci, 0x66..=0x68);
            record(
                "Application selection parameters",
                "§8.10 / EN 13757-3, §7.2–7.3",
                primary!(80, 218),
                self.selection_length.map(|length| {
                    if query {
                        length == 0
                    } else if response {
                        (1..=10).contains(&length)
                    } else {
                        length <= 10
                    }
                }),
                "A query has no parameter. A selection or status response carries up to ten parameter bytes. An empty selection command resets the application.",
            );
            return;
        }
        if matches!(ci, 0x64 | 0x65 | 0x7e | 0x7f) {
            if let Some(block) = self.sml_v2 {
                rows.push(sml_transfer_finding("SML version 2 block", block.is_ok(), "Checks the reserved version byte, initial negotiation controls, ACK payload, escape padding and transport CRC."));
                return;
            }
            record(
                "SML message structure and checksum",
                "§8.6 / SML 1.04, §5",
                sml_page!(17, 618),
                self.sml_envelope,
                "Checks the six-field message envelope and the CRC of each readable SML binary message.",
            );
            record(
                "SML message body schema",
                "§8.6 / SML 1.04, §5.1",
                sml_page!(20, 228),
                self.sml_body,
                "Checks each recognized message schema, including its parameter trees, time choices, profile and value lists, and COSEM attribute descriptors.",
            );
            if let Some(valid) = self.sml_transport {
                record(
                    "SML version 1 transport",
                    "§8.6 / SML 1.04, §8.1",
                    sml_page!(67, 198),
                    Some(valid),
                    "The escaped frame must have valid padding, a complete end marker and a matching transport CRC.",
                );
            }
            if let Some(valid) = self.sml_file {
                record(
                    "SML file open and close",
                    "§8.6 / SML 1.04, §4.1",
                    sml_page!(16, 690),
                    Some(valid),
                    "Validates request, response and combined file boundaries. A response to a request may use an attention message at either end.",
                );
            }
            if let Some(valid) = self.sml_file_groups {
                record(
                    "SML file message groups",
                    "§8.6 / SML 1.04, §5",
                    sml_page!(17, 501),
                    Some(valid),
                    "Each group occupies a contiguous range within its request or response file.",
                );
            }
            if let Some(valid) = self.sml_file_address {
                record(
                    "SML file request addressing",
                    "§8.6 / SML 1.04, §5.1.1",
                    sml_page!(21, 426),
                    Some(valid),
                    "Every request with a server ID field follows the addressed or broadcast mode of its open request.",
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
        } else if self.dlms_service.is_none() {
            record(
                "DLMS application payload",
                "§8.5",
                primary!(78, 122),
                None,
                "An association APDU is needed to assess its fields. Other COSEM services require a service-specific decoder.",
            );
        }
        if let Some(service) = self.dlms_service {
            service.findings(ci, rows);
        }
    }
}
