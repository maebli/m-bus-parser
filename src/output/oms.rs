use super::{DecodedOutput, OmsReport, OutputError};
use crate::rscada_xml::xml_encode;
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

pub fn render_oms_reports(
    frames: &[DecodedOutput],
    format: OmsFormat,
) -> Result<String, OutputError> {
    let reports: Vec<_> = frames.iter().map(DecodedOutput::assess_oms_5_0_1).collect();
    match format {
        OmsFormat::Json => match reports.as_slice() {
            [report] => serde_json::to_string_pretty(report),
            _ => serde_json::to_string_pretty(&reports),
        }
        .map_err(|error| OutputError::Serialization {
            format: "oms-json",
            message: error.to_string(),
        }),
        OmsFormat::Xml => Ok(xml(&reports)),
        OmsFormat::Csv => csv(&reports).map_err(|error| OutputError::Serialization {
            format: "oms-csv",
            message: error.to_string(),
        }),
    }
}

fn xml(reports: &[OmsReport]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<oms_reports>\n");
    for (index, report) in reports.iter().enumerate() {
        let _ = writeln!(out, "  <report telegram=\"{}\" version=\"{}\" passed=\"{}\" assessed=\"{}\" unchecked=\"{}\">", index + 1, report.version, report.passed, report.assessed, report.unchecked);
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

fn csv(reports: &[OmsReport]) -> Result<String, Box<dyn std::error::Error>> {
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
                index + 1,
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
