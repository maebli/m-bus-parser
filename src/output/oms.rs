use super::{ConfigurationField, DecodedOutput, Link, OmsReport, OutputError};
use crate::rscada_xml::xml_encode;
use m_bus_oms_rules::sequence::{AccessObservation, ExchangeFrame};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmsFormat {
    Json,
    Xml,
    Csv,
}

impl OmsFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "oms-json",
            Self::Xml => "oms-xml",
            Self::Csv => "oms-csv",
        }
    }
}

pub(super) fn encrypted_record_prefix(
    frame: &super::FrameOutput,
    block: Option<&crate::user_data::UserDataBlock<'_>>,
    security: &super::SecurityContext,
) -> Option<usize> {
    if !security.encrypted {
        return (frame.kind == "wireless" && !security.ell_encrypted).then_some(0);
    }
    let plain_len = security.decrypted_payload.as_ref()?.len();
    let (header, data) = block?.transport_payload()?;
    let cf = header.configuration_field;
    let lower = cf.mode_specific_lower();
    match cf.security_mode().to_bits() {
        5 | 7 if lower >> 4 == 15 => Some(plain_len),
        5 => Some(usize::from(lower >> 4) * 16),
        7 => {
            let extension = 1 + usize::from(data.first()? & 0x40 != 0);
            let clear_tail = data
                .len()
                .checked_sub(extension + usize::from(lower >> 4) * 16)?;
            plain_len.checked_sub(clear_tail)
        }
        10 if lower != 0xff => Some(usize::from(lower)),
        10 | 13 => Some(plain_len),
        _ => None,
    }
}

pub(super) fn access_observation(
    frame: &DecodedOutput,
    link: Link,
    configuration: Option<ConfigurationField>,
) -> Option<AccessObservation> {
    if !matches!(link, Link::Wireless(0x44 | 0x46..=0x48)) {
        return None;
    }
    let address = frame.raw.normalized_frame_hex.get(4..20)?;
    let sender = super::decode_hex_bytes(address).ok()?.try_into().ok()?;
    let (synchronous, repeated, number) = if let Some(ell) = &frame.transport.ell {
        let flags = u8::from_str_radix(&ell.communication_control, 16).ok()?;
        (
            Some(flags & 0x20 != 0),
            flags & 0x12 != 0,
            Some(ell.access_number),
        )
    } else {
        let cf = configuration.filter(|cf| matches!(cf.security_mode().to_bits(), 0 | 5));
        let tpl = frame
            .transport
            .access_numbers
            .iter()
            .find(|field| matches!(field.source.as_str(), "tpl" | "tpl.long" | "tpl.short"));
        (
            cf.map(|cf| cf.raw() & 0x2000 != 0),
            cf.is_some_and(|cf| cf.raw() & 3 != 0),
            tpl.map(|field| field.value),
        )
    };
    Some(AccessObservation {
        sender,
        synchronous,
        repeated,
        number,
    })
}

fn exchange_frame(frame: &DecodedOutput) -> Option<ExchangeFrame> {
    if frame.frame.kind != "wireless" {
        return None;
    }
    let bytes = &frame.raw.normalized_frame_hex;
    let sender = super::decode_hex_bytes(bytes.get(4..20)?)
        .ok()?
        .try_into()
        .ok()?;
    let function = u8::from_str_radix(frame.frame.control_field.as_deref()?, 16).ok()?;
    let ell = frame.transport.ell.as_ref();
    let receiver = ell.and_then(|ell| {
        let manufacturer = u16::from_str_radix(ell.receiver_manufacturer.as_deref()?, 16)
            .ok()?
            .to_le_bytes();
        let id = super::decode_hex_bytes(ell.receiver_address.as_deref()?).ok()?;
        let [a, b, c, d, version, medium] = *id.as_slice() else {
            return None;
        };
        let address = [
            manufacturer[0],
            manufacturer[1],
            a,
            b,
            c,
            d,
            version,
            medium,
        ];
        Some(address)
    });
    Some(ExchangeFrame {
        sender,
        receiver,
        function,
        link_access: ell.map(|ell| ell.access_number),
        transport_access: frame
            .transport
            .access_numbers
            .iter()
            .find(|field| matches!(field.source.as_str(), "tpl" | "tpl.short" | "tpl.long"))
            .map(|field| field.value),
        transport_status: frame
            .transport
            .statuses
            .iter()
            .find(|field| matches!(field.source.as_str(), "tpl" | "tpl.short" | "tpl.long"))
            .and_then(|s| u8::from_str_radix(&s.raw, 16).ok()),
        fragmented: matches!(
            frame.record_availability,
            super::RecordAvailability::Fragmented
        ),
    })
}

pub fn render_oms_reports(
    frames: &[DecodedOutput],
    format: OmsFormat,
) -> Result<String, OutputError> {
    render_oms_capture(frames, format, None, false)
}

pub fn render_oms_capture(
    frames: &[DecodedOutput],
    format: OmsFormat,
    selected: Option<usize>,
    consecutive: bool,
) -> Result<String, OutputError> {
    if selected.is_some_and(|index| index >= frames.len()) {
        return Err(OutputError::InvalidOption {
            option: "telegram",
            message: "Telegram index outside the capture.".into(),
        });
    }
    let mut history = Default::default();
    let mut reports = Vec::new();
    let mut previous: Option<ExchangeFrame> = None;
    for (index, frame) in frames.iter().enumerate() {
        let mut report = frame.assess_oms_with_history(&mut history, consecutive);
        let current = exchange_frame(frame);
        if let (Some(request), Some(reply)) = (previous, current) {
            request.check_reply(reply, consecutive, &mut report);
        }
        previous = current;
        if selected.is_none_or(|selected| selected == index) {
            reports.push(report);
        }
        if selected == Some(index) {
            break;
        }
    }
    match format {
        OmsFormat::Json => match reports.as_slice() {
            [report] => serde_json::to_string_pretty(report),
            _ => serde_json::to_string_pretty(&reports),
        }
        .map_err(|error| OutputError::Serialization {
            format: "oms-json",
            message: error.to_string(),
        }),
        OmsFormat::Xml => Ok(xml(&reports, selected.unwrap_or(0))),
        OmsFormat::Csv => {
            csv(&reports, selected.unwrap_or(0)).map_err(|error| OutputError::Serialization {
                format: "oms-csv",
                message: error.to_string(),
            })
        }
    }
}

fn xml(reports: &[OmsReport], offset: usize) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<oms_reports>\n");
    for (index, report) in reports.iter().enumerate() {
        let _ = writeln!(out, "  <report telegram=\"{}\" version=\"{}\" passed=\"{}\" assessed=\"{}\" unchecked=\"{}\">", index + offset + 1, report.version, report.passed, report.assessed, report.unchecked);
        let _ = writeln!(
            out,
            "    <specification>{}</specification>",
            xml_encode(report.specification)
        );
        let _ = writeln!(
            out,
            "    <security_mode result=\"{}\" candidate_profile=\"{}\"/>",
            xml_encode(report.security_mode.result),
            xml_encode(report.security_mode.candidate_profile.unwrap_or_default())
        );
        for finding in &report.findings {
            let _ = write!(out, "    <finding status=\"{}\"", finding.status.as_str());
            if let Some(number) = finding.record_number {
                let _ = write!(out, " record_number=\"{number}\"");
            }
            let _ = writeln!(
                out,
                "><rule>{}</rule><reference>{}</reference><source_url>{}</source_url><detail>{}</detail></finding>",
                xml_encode(finding.rule),
                xml_encode(finding.reference),
                xml_encode(finding.source_url),
                xml_encode(finding.detail)
            );
        }
        out.push_str("  </report>\n");
    }
    out.push_str("</oms_reports>\n");
    out
}

fn csv(reports: &[OmsReport], offset: usize) -> Result<String, Box<dyn std::error::Error>> {
    let mut writer = prettytable::csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "telegram",
        "version",
        "specification",
        "passed",
        "assessed",
        "unchecked",
        "security_mode_result",
        "candidate_profile",
        "rule",
        "reference",
        "status",
        "detail",
        "record_number",
        "source_url",
    ])?;
    for (index, report) in reports.iter().enumerate() {
        for finding in &report.findings {
            writer.serialize((
                index + offset + 1,
                report.version,
                report.specification,
                report.passed,
                report.assessed,
                report.unchecked,
                report.security_mode.result,
                report.security_mode.candidate_profile,
                finding.rule,
                finding.reference,
                finding.status.as_str(),
                finding.detail,
                finding.record_number,
                finding.source_url,
            ))?;
        }
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}
