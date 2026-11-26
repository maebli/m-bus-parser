mod readings;

use super::{ConfigurationField, DecodedOutput, Link, OmsReport, OutputError};
use crate::rscada_xml::xml_encode;
use m_bus_oms_rules::sequence::{AccessObservation, ExchangeFrame, KeyInformationHistory};
use std::fmt::Write;

pub use m_bus_oms_rules::layers::OmsRadioMode;
pub use m_bus_oms_rules::readings::Taf7Parameters;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OmsCaptureOptions {
    pub consecutive: bool,
    pub radio_mode: Option<OmsRadioMode>,
    pub taf7: Option<Taf7Parameters>,
}

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

pub(super) fn clock_fields(
    application: Option<&super::ApplicationOutput>,
) -> Option<m_bus_oms_rules::layers::ClockFields> {
    let application = application?;
    if application.protocol != "security_information_transfer" {
        return None;
    }
    let fields = application.decoded.as_ref()?;
    if fields.get("protocol_id")?.as_u64()? != 4 {
        return None;
    }
    let data = fields.get("apdu_hex")?.as_str()?;
    let octet = |field| u8::try_from(fields.get(field)?.as_u64()?).ok();
    Some(m_bus_oms_rules::layers::ClockFields {
        control: u8::from_str_radix(data.get(..2)?, 16).ok()?,
        length: data.len() / 2,
        key: octet("key_id")?,
        block_control: octet("block_control")?,
        immediate: fields.get("target_time_hex")?.as_str()? == "0000000030",
        verified: fields.get("application_mac_verified")?.as_bool()?,
    })
}

pub(super) fn encrypted_record_prefix(
    frame: &super::FrameOutput,
    block: Option<&crate::user_data::UserDataBlock<'_>>,
    security: &super::SecurityContext,
) -> Option<usize> {
    if !security.encrypted {
        return (matches!(frame.kind.as_str(), "wireless" | "lpwan") && !security.ell_encrypted)
            .then_some(0);
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

pub(super) fn check_current_din(frame: &DecodedOutput, report: &mut OmsReport) {
    let Some(identity) = frame.meter.identity.as_ref() else {
        return;
    };
    let (Some(serial), Some(manufacturer), Some(device)) = (
        identity.id.as_deref(),
        identity.manufacturer_code.as_deref(),
        identity.device_type_code,
    ) else {
        return;
    };
    let findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule == "ID8!")
        .filter_map(|f| {
            let n = f.record_number?;
            let record = frame.records.get(n.checked_sub(1)?)?;
            let data = super::decode_hex_bytes(&record.data_hex).ok()?;
            m_bus_oms_rules::address::current_din_address(&data, manufacturer, serial, device, n)
        })
        .collect();
    for finding in findings {
        report.push(finding);
    }
}

pub(super) fn wired_observation(
    frame: &DecodedOutput,
) -> Option<m_bus_oms_rules::wired::WiredObservation> {
    use m_bus_oms_rules::wired::WiredObservation;
    let offset = match frame.frame.kind.as_str() {
        "single_character" => return Some(WiredObservation::Ack),
        "short" => 4,
        "long" | "control" => 10,
        _ => return None,
    };
    let control = u8::from_str_radix(frame.frame.control_field.as_deref()?, 16).ok()?;
    let address =
        u8::from_str_radix(frame.raw.normalized_frame_hex.get(offset..offset + 2)?, 16).ok()?;
    Some(WiredObservation::Frame { control, address })
}

pub(super) fn access_observation(
    frame: &DecodedOutput,
    link: Link,
    configuration: Option<ConfigurationField>,
) -> Option<AccessObservation> {
    if !matches!(link, Link::Wireless(0x44 | 0x46..=0x48)) {
        return None;
    }
    if let Some(mac) = &frame.lpwan {
        let llc = mac.llc.as_ref()?;
        return Some(AccessObservation {
            sender: llc.sender?,
            synchronous: Some(llc.flags & 0x40 != 0),
            repeated: false,
            number: llc.access_number,
        });
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
        security_mode: frame.security.mode_code,
        key_information_present: if frame.security.mode_code == Some(10) {
            frame
                .transport
                .configuration_extension
                .map(|ext| ext & 0x40 != 0)
        } else {
            frame
                .transport
                .header_kind
                .as_ref()
                .map(|_| frame.transport.afl_key_information.is_some())
        },
        key_id: frame.security.key_id,
        key_version: frame.security.key_version,
        tls_hello: frame.security.tls.as_ref().and_then(|tls| tls.records.iter().find_map(|record| record.hello)),
        security_counter: frame.security.message_counter,
        verified_encryption: frame.security.decryption_state == "decrypted_authenticated"
            && frame
                .transport
                .configuration
                .as_ref()
                .and_then(|c| u16::from_str_radix(&c.raw, 16).ok())
                .is_some_and(|cf| match frame.security.mode_code {
                    Some(7) => cf & 0xf0 != 0,
                    Some(10) => cf & 0xff != 0,
                    _ => false,
                }),
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
    render_oms_capture_with_options(
        frames,
        format,
        selected,
        OmsCaptureOptions {
            consecutive,
            ..Default::default()
        },
    )
}

pub fn render_oms_capture_with_options(
    frames: &[DecodedOutput],
    format: OmsFormat,
    selected: Option<usize>,
    options: OmsCaptureOptions,
) -> Result<String, OutputError> {
    let consecutive = options.consecutive;
    if selected.is_some_and(|index| index >= frames.len()) {
        return Err(OutputError::InvalidOption {
            option: "telegram",
            message: "Telegram index outside the capture.".into(),
        });
    }
    let mut history = Default::default();
    let mut reports = Vec::new();
    let mut previous_wired: Option<m_bus_oms_rules::wired::WiredObservation> = None;
    let mut previous: Option<ExchangeFrame> = None;
    let mut lpwan = m_bus_oms_rules::lpwan::LpwanHistory::default();
    let mut key_information = KeyInformationHistory::default();
    let mut reading_history = readings::CaptureReadings::default();
    for (index, frame) in frames.iter().enumerate() {
        let mut report = frame.assess_oms_with_history(&mut history, options);
        if let Some(mac) = &frame.lpwan {
            lpwan.observe(mac, consecutive, &mut report);
        }
        let wired = wired_observation(frame);
        if consecutive {
            if let (Some(request), Some(reply)) = (previous_wired, wired) {
                request.check_reply(reply, &mut report);
            }
        }
        previous_wired = wired;
        let current = exchange_frame(frame);
        if let (Some(request), Some(reply)) = (previous, current) {
            request.check_reply(reply, consecutive, &mut report);
        }
        if let Some(frame) = current {
            if options.radio_mode.is_some_and(OmsRadioMode::is_c) {
                key_information.check_mode_c_ell(frame, &mut report);
            }
            key_information.observe(frame, consecutive, &mut report);
        }
        reading_history.observe(frame, &mut report, consecutive);
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
