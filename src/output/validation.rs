use super::{wireless, DecodedOutput, OmsReport, RecordAvailability};
use m_bus_oms_rules::telegram::RuleStatus;
use serde::Serialize;

#[derive(Serialize)]
struct Finding<'a> {
    group: &'static str,
    rule: &'a str,
    status: RuleStatus,
    detail: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    record_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<&'a str>,
}

impl<'a> Finding<'a> {
    fn new(group: &'static str, rule: &'a str, status: RuleStatus, detail: &'a str) -> Self {
        Self {
            group,
            rule,
            status,
            detail,
            offset_start: None,
            record_number: None,
            reference: None,
            source_url: None,
        }
    }
}

fn frame_checks(frame: &DecodedOutput, rows: &mut Vec<Finding<'_>>) {
    use RuleStatus::{CouldNotCheck, Failed, Passed};
    let kind = frame.frame.kind.as_str();
    if kind == "application_layer" || frame.protocol == "application" {
        return;
    }
    if !frame.raw.fragment_frames_hex.is_empty() {
        rows.push(Finding::new("M-Bus", "Capture framing", CouldNotCheck, "The report contains a reconstructed message. Its bytes do not establish each original frame checksum."));
        return;
    }
    let Ok(bytes) = super::decode_hex_bytes(&frame.raw.original_frame_hex) else {
        return;
    };
    match kind {
        "short" | "long" | "control" | "single_character" => {
            let valid = match bytes.as_slice() {
                [0xe5] => true,
                [0x10, _, _, _, 0x16] => true,
                [0x68, a, b, 0x68, .., 0x16] => a == b && bytes.len() == usize::from(*a) + 6,
                _ => false,
            };
            rows.push(Finding::new(
                "M-Bus",
                "Wired frame structure",
                if valid { Passed } else { Failed },
                "Checks the frame delimiters and the length for this frame type.",
            ));
            if kind != "single_character" {
                rows.push(Finding::new(
                    "M-Bus",
                    "Wired checksum",
                    Passed,
                    "The link decoder verified the frame checksum.",
                ));
            }
        }
        "wireless" => {
            let Some(&length) = bytes.first() else { return };
            let declared = usize::from(length) + 1;
            let format_a_len = declared + 2 + 2 * declared.saturating_sub(10).div_ceil(16);
            let standard_blocks = bytes.len() == format_a_len && declared >= 10;
            let length_matches = bytes.len() == declared || standard_blocks;
            rows.push(Finding::new("M-Bus", "Wireless frame length", if length_matches {Passed} else if frame.raw.format_a_crc_stripped {CouldNotCheck} else {Failed}, "Compares the captured length with the L-field, accounting for Format A CRC blocks."));
            let (status, detail) = if standard_blocks {
                if frame.raw.format_a_crc_stripped {
                    (
                        Passed,
                        "The link decoder verified the header and all Format A block CRCs.",
                    )
                } else {
                    (
                        Failed,
                        "A captured Format A block did not pass CRC verification.",
                    )
                }
            } else if frame.raw.format_a_crc_stripped {
                (
                    CouldNotCheck,
                    "The header CRC was verified. The capture does not establish every block CRC.",
                )
            } else if wireless::trailing_frame_crc_start(&bytes).is_some() {
                (
                    Passed,
                    "The final two bytes match the CRC calculated over the preceding bytes.",
                )
            } else {
                (
                    CouldNotCheck,
                    "No verifiable CRC was identified in the capture.",
                )
            };
            rows.push(Finding::new("M-Bus", "Wireless CRC", status, detail));
        }
        _ => {}
    }
}

pub(super) fn report(frame: &DecodedOutput, oms: &OmsReport, telegram: usize) -> serde_json::Value {
    use RuleStatus::{CouldNotCheck, Failed, NotApplicable, Passed};
    let mut findings = Vec::new();
    frame_checks(frame, &mut findings);
    let embedded_checked = oms
        .findings
        .iter()
        .any(|row| row.rule.starts_with("SML ") || row.rule.starts_with("DLMS "));
    for diagnostic in &frame.diagnostics {
        if diagnostic.layer != "application"
            || (embedded_checked && diagnostic.code == "application.external_protocol")
        {
            continue;
        }
        let status = match diagnostic.code.as_str() {
            "application.payload_invalid" | "application.record_partial" => Failed,
            _ => CouldNotCheck,
        };
        let mut row = Finding::new(
            "Payload",
            "Application decoding",
            status,
            &diagnostic.message,
        );
        row.offset_start = diagnostic.offset_start;
        findings.push(row);
    }
    if !frame.records.is_empty()
        && matches!(frame.record_availability, RecordAvailability::Complete)
    {
        findings.push(Finding::new(
            "Payload",
            "Data record structure",
            Passed,
            "The parser consumed the record headers and their declared data lengths.",
        ));
    } else if matches!(
        frame.record_availability,
        RecordAvailability::Encrypted | RecordAvailability::Fragmented
    ) {
        findings.push(Finding::new(
            "Payload",
            "Application decoding",
            CouldNotCheck,
            match frame.security.decryption_state.as_str() {
                "key_missing" => "Meter key required to decode the application records.",
                "session_required" => "TLS session required to decode the application records.",
                _ => "The complete plaintext payload is required.",
            },
        ));
    }
    let (status, detail) = match frame.security.decryption_state.as_str() {
        "decrypted_authenticated" => (
            Passed,
            "The supplied credentials verified the transport authentication tag.",
        ),
        "session_required" => (
            CouldNotCheck,
            "TLS session required to authenticate the protected records.",
        ),
        "key_missing" if frame.security.mode_code != Some(5) => (
            CouldNotCheck,
            "Key required to verify the protected message.",
        ),
        "decrypted_unverified" | "key_missing" if frame.security.mode_code == Some(5) => (
            NotApplicable,
            "Mode 5 encryption does not provide transport authentication.",
        ),
        "not_applicable" if frame.security.payload_protection == "cleartext" => {
            (NotApplicable, "The transport payload is unencrypted.")
        }
        _ => (
            CouldNotCheck,
            "Transport authentication could not be established.",
        ),
    };
    findings.push(Finding::new(
        "Security",
        "Transport authentication",
        status,
        detail,
    ));
    for finding in &oms.findings {
        let group = if finding.rule.starts_with("SML ") {
            "SML"
        } else if finding.rule.starts_with("DLMS ") {
            "DLMS/COSEM"
        } else {
            "OMS"
        };
        let mut row = Finding::new(group, finding.rule, finding.status, finding.detail);
        row.reference = Some(finding.reference);
        row.source_url = Some(finding.source_url);
        row.record_number = finding.record_number;
        findings.push(row);
    }
    let mut passed = 0;
    let mut failed = 0;
    let mut unchecked = 0;
    for row in &findings {
        match row.status {
            Passed => passed += 1,
            Failed => failed += 1,
            CouldNotCheck => unchecked += 1,
            NotApplicable => {}
        }
    }
    serde_json::json!({
        "telegram": telegram, "passed": passed, "assessed": passed + failed,
        "unchecked": unchecked, "version": oms.version, "specification": oms.specification,
        "security_mode": oms.security_mode, "findings": findings,
    })
}
