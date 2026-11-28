mod protocols;
pub use protocols::{ApplicationProtocolEvidence, DlmsAssociation, SmlBlock};

use crate::layers::LayerFields;
use alloc::{vec, vec::Vec};
use m_bus_core::{ConfigurationField, Function, SecurityMode};

use crate::OmsSecurityModeCheck;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
pub enum RuleStatus {
    Passed,
    Failed,
    CouldNotCheck,
    NotApplicable,
}

use RuleStatus::{CouldNotCheck, Failed, NotApplicable, Passed};

impl RuleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Passed => "passed",
            Failed => "failed",
            CouldNotCheck => "could_not_check",
            NotApplicable => "not_applicable",
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RuleFinding {
    pub rule: &'static str,
    pub reference: &'static str,
    pub source_url: &'static str,
    pub status: RuleStatus,
    pub detail: &'static str,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub record_number: Option<usize>,
}

#[derive(Clone, Copy)]
pub enum Link {
    Wireless(u8),
    Wired(Function),
    Unavailable,
}

impl Link {
    pub fn function(self) -> Option<Function> {
        match self {
            Self::Wireless(0x00 | 0x10 | 0x20 | 0x30) => Some(Function::Ack),
            Self::Wireless(0x01 | 0x11 | 0x21 | 0x31) => Some(Function::Nack),
            Self::Wireless(0x06) => Some(Function::CnfIr),
            Self::Wireless(0x43) => Some(Function::SndUd2),
            Self::Wireless(0x46) => Some(Function::SendIr),
            Self::Wireless(0x47) => Some(Function::AccNr),
            Self::Wireless(0x48) => Some(Function::AccDmd),
            Self::Wireless(control) => Function::try_from(control).ok(),
            Self::Wired(function) => Some(function),
            Self::Unavailable => None,
        }
    }

    pub fn check(self) -> RuleFinding {
        let (status, detail) = match self {
            Self::Wireless(
                0x00 | 0x10 | 0x20 | 0x30 | 0x01 | 0x11 | 0x21 | 0x31 | 0x08 | 0x18 | 0x28 | 0x38
                | 0x06 | 0x40 | 0x43 | 0x44 | 0x46 | 0x47 | 0x48 | 0x53 | 0x73 | 0x5a | 0x7a | 0x5b
                | 0x7b,
            ) => (Passed, "The wireless C-field is listed."),
            Self::Wireless(_) => (Failed, "The wireless C-field is outside the listed values."),
            Self::Wired(Function::SndNr) => (Failed, "SND-NR requires a wireless link."),
            Self::Wired(_) => (
                Passed,
                "The wired telegram uses a function other than SND-NR.",
            ),
            Self::Unavailable => (CouldNotCheck, "No link control field is available."),
        };
        RuleFinding {
            rule: "Link control field",
            reference: "§5.2.4, Tables 15–16",
            source_url: primary!(52, 342),
            status,
            detail,
            record_number: None,
        }
    }
}

pub fn check_mode_zero_blocks(configuration: Option<ConfigurationField>) -> RuleFinding {
    let (status, detail) = match configuration {
        None => (
            CouldNotCheck,
            "No transport configuration field is available.",
        ),
        Some(field) if field.security_mode() != SecurityMode::NoEncryption => {
            (NotApplicable, "This transport rule applies to mode 0.")
        }
        Some(field) if field.mode_specific_lower() >> 4 != 0 => {
            (Failed, "Mode 0 declares encrypted blocks.")
        }
        Some(_) => (Passed, "Mode 0 declares zero encrypted blocks."),
    };
    RuleFinding {
        rule: "Mode 0 block count",
        reference: "§7.2.4.2",
        source_url: primary!(62, 257),
        status,
        detail,
        record_number: None,
    }
}

impl OmsSecurityModeCheck {
    pub fn finding(&self) -> RuleFinding {
        let (status, detail) = match self.result {
            "listed" => (Passed, "The declared mode is listed for OMS."),
            "requires_mode_0" => (Failed, "This link function requires security mode 0."),
            "not_listed" => (Failed, "The declared mode is outside the listed modes."),
            _ => (
                CouldNotCheck,
                "No explicit transport security mode is available.",
            ),
        };
        RuleFinding {
            rule: "Security mode",
            reference: "Table 43",
            source_url: primary!(83, 145),
            status,
            detail,
            record_number: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
pub enum RecordAvailability {
    Complete,
    Partial,
    Encrypted,
    Fragmented,
    Unavailable,
}

impl RecordAvailability {
    fn unchecked_reason(self) -> Option<&'static str> {
        use RecordAvailability::*;
        match self {
            Complete => None,
            Partial => Some("Only part of the record stream was decoded."),
            Encrypted => Some("Decrypt the application payload to check its records."),
            Fragmented => Some("Reassemble the telegram to check its records."),
            Unavailable => Some("No decoded variable M-Bus records are available."),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OmsReport<F = [RuleFinding; 6]> {
    pub version: &'static str,
    pub specification: &'static str,
    pub passed: usize,
    pub assessed: usize,
    pub unchecked: usize,
    pub security_mode: OmsSecurityModeCheck,
    pub findings: F,
}

pub fn assess_5_0_1<H: AsRef<[u8]>>(
    link: Link,
    configuration: Option<ConfigurationField>,
    availability: RecordAvailability,
    headers: impl IntoIterator<Item = Option<H>>,
) -> OmsReport {
    use RecordAvailability::*;
    let mut unavailable = availability.unchecked_reason();
    let mut volumes = VolumeReadings::default();
    if matches!(availability, Complete | Partial) {
        for (index, header) in headers.into_iter().enumerate() {
            if let Some(header) = header {
                volumes.observe(header.as_ref(), index + 1);
            } else {
                unavailable = Some("A decoded record header is unavailable.");
            }
        }
    }
    let security_mode = OmsSecurityModeCheck::assess_5_0_1(
        link.function(),
        configuration.map(|field| field.security_mode()),
    );
    let [encoding, unique, date] = volumes.findings(unavailable);
    let findings = [
        link.check(),
        check_mode_zero_blocks(configuration),
        security_mode.finding(),
        encoding,
        unique,
        date,
    ];
    report(security_mode, findings)
}

pub fn static_content(configuration: ConfigurationField) -> bool {
    match configuration.security_mode().to_bits() {
        5 => configuration.mode_specific_lower() & 12 == 8,
        7 | 10 | 13 => configuration.raw() >> 14 == 2,
        _ => false,
    }
}

pub fn fragment_destination(addressed: bool) -> RuleFinding {
    RuleFinding {
        rule: "Fragment destination address",
        reference: "§3.1.3.3",
        source_url: primary!(31, 776),
        status: if addressed { Passed } else { Failed },
        detail: "Each wireless data fragment requires a long ELL with the destination address.",
        record_number: None,
    }
}

pub fn assess_telegram(
    link: Link,
    configuration: Option<ConfigurationField>,
    fields: LayerFields,
    availability: RecordAvailability,
) -> TelegramReport {
    let security_mode = OmsSecurityModeCheck::assess_5_0_1(
        link.function(),
        configuration.map(|cf| cf.security_mode()),
    );
    let mut findings = vec![
        link.check(),
        check_mode_zero_blocks(configuration),
        security_mode.finding(),
    ];
    fields.check(link, configuration, &mut findings);
    if matches!(availability, RecordAvailability::Fragmented) {
        if matches!(link, Link::Wireless(_)) {
            findings.push(fragment_destination(fields.outer_ci == Some(0x8e)));
        }
        let function = link.function();
        findings.push(RuleFinding {
            rule: "Fragment link function",
            reference: "§6.3, Tables 15–16",
            source_url: primary!(57, 91),
            status: match function {
                Some(Function::SndUd { .. } | Function::RspUd { .. }) => Passed,
                Some(_) => Failed,
                None => CouldNotCheck,
            },
            detail: "Fragmented application data requires SND-UD or RSP-UD.",
            record_number: None,
        });
    }
    let application = fields.application_ci.or(fields.outer_ci);
    if let Some(ci @ (0x50 | 0x53..=0x57 | 0x66..=0x68 | 0x60 | 0x61 | 0x64 | 0x65 | 0x7c..=0x7f)) =
        application
    {
        fields.application_protocol.check(ci, &mut findings);
    }
    report(security_mode, findings)
}

pub type TelegramReport = OmsReport<Vec<RuleFinding>>;

impl TelegramReport {
    pub fn push(&mut self, finding: RuleFinding) {
        match finding.status {
            Passed => {
                self.passed += 1;
                self.assessed += 1;
            }
            Failed => self.assessed += 1,
            CouldNotCheck => self.unchecked += 1,
            NotApplicable => {}
        }
        self.findings.push(finding);
    }
}

fn report<F: AsRef<[RuleFinding]>>(
    security_mode: OmsSecurityModeCheck,
    findings: F,
) -> OmsReport<F> {
    let rows = findings.as_ref();
    let passed = rows.iter().filter(|f| matches!(f.status, Passed)).count();
    let failed = rows.iter().filter(|f| matches!(f.status, Failed)).count();
    let unchecked = rows
        .iter()
        .filter(|f| matches!(f.status, CouldNotCheck))
        .count();
    OmsReport {
        version: "5.0.1",
        specification: "https://oms-group.org/wp-content/uploads/2024/10/OMS-Spec_Vol2_Primary_v501_01.pdf",
        passed,
        assessed: passed + failed,
        unchecked,
        findings,
        security_mode,
    }
}

#[derive(Default)]
pub struct VolumeReadings {
    current: Option<usize>,
    due: Option<usize>,
    duplicate: Option<usize>,
    invalid_encoding: Option<usize>,
    has_due_date: bool,
}

impl VolumeReadings {
    pub fn observe(&mut self, header: &[u8], record_number: usize) {
        if header == [0x42, 0x6c] {
            self.has_due_date = true;
            return;
        }
        let dif = match header {
            [dif, 0x10..=0x17] | [dif, 0x90..=0x97, 0x7d] => *dif,
            _ => return,
        };
        let reading = match dif >> 4 {
            0 => &mut self.current,
            4 => &mut self.due,
            _ => return,
        };
        if reading.replace(record_number).is_some() {
            self.duplicate.get_or_insert(record_number);
        }
        if !matches!(dif & 0x0f, 1..=4 | 6 | 7 | 9..=12 | 14) {
            self.invalid_encoding.get_or_insert(record_number);
        }
    }

    pub fn findings(&self, unavailable: Option<&'static str>) -> [RuleFinding; 3] {
        let observed = self.current.is_some() || self.due.is_some();
        let unchecked = match unavailable {
            Some(reason) => (CouldNotCheck, reason, None),
            None => (
                NotApplicable,
                "No matching volume reading was observed.",
                None,
            ),
        };
        let encoding = match (self.invalid_encoding, unavailable, observed) {
            (Some(n), _, _) => (
                Failed,
                "This volume reading requires integer or BCD encoding.",
                Some(n),
            ),
            (None, None, true) => (
                Passed,
                "The observed volume readings use integer or BCD encoding.",
                None,
            ),
            _ => unchecked,
        };
        let unique = match (self.duplicate, unavailable, observed) {
            (Some(n), _, _) => (
                Failed,
                "This volume reading repeats a data point in the telegram.",
                Some(n),
            ),
            (None, None, true) => (Passed, "Each observed volume tag occurs once.", None),
            _ => unchecked,
        };
        let date = match self.due {
            Some(n) if self.has_due_date => (
                Passed,
                "The telegram includes the DT2!D date record.",
                Some(n),
            ),
            Some(n) => (
                CouldNotCheck,
                "Check the static telegram for the DT2!D date record.",
                Some(n),
            ),
            None if unavailable.is_none() => (
                NotApplicable,
                "No VM1!D due-date volume reading was observed.",
                None,
            ),
            None => unchecked,
        };
        [
            (
                "Volume encoding (VM1! / VM1!D)",
                "Annex B, MB-Data-Tags",
                crate::sources::tag_page("VM1!"),
                encoding,
            ),
            (
                "Unique volume readings (VM1! / VM1!D)",
                "§8.4.4",
                primary!(75, 148),
                unique,
            ),
            ("Due date for VM1!D", "§8.4.5.1", primary!(76, 319), date),
        ]
        .map(
            |(rule, reference, source_url, (status, detail, record_number))| RuleFinding {
                rule,
                reference,
                source_url,
                status,
                detail,
                record_number,
            },
        )
    }
}
