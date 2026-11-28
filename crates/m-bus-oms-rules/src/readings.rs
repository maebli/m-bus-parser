use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::catalog::{identify, meter_tag};
use crate::telegram::{RuleFinding, RuleStatus};

mod consumer_information;
mod descriptors;
pub mod history;
mod profiles;
mod ranges;
mod required_tags;
mod values;

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Taf7Parameters {
    pub measuring_period_ms: Option<u64>,
    pub registration_period_ms: Option<u64>,
    pub acceptance_window_ms: Option<u64>,
    pub nominal_transmission_interval_ms: Option<u64>,
}

pub(super) struct Tag(pub &'static str, pub Coding);

#[derive(Clone, Copy)]
pub(super) enum Coding {
    NoData,
    Integer,
    Number,
    NumberOrVariable,
    IntegerOrVariable,
    Variable,
    Serial,
    Address,
    Byte,
    Timestamp,
    Date,
    Time,
    Unmatched,
}

impl Coding {
    pub(super) fn accepts(self, dif: u8) -> bool {
        let integer = matches!(dif, 1..=4 | 6 | 7);
        let number = integer || matches!(dif, 9..=12 | 14);
        match self {
            Self::NoData => dif == 0,
            Self::Integer => integer,
            Self::Number => number,
            Self::NumberOrVariable => number || dif == 13,
            Self::IntegerOrVariable => integer || dif == 13,
            Self::Variable => dif == 13,
            Self::Serial => dif == 12,
            Self::Address => matches!(dif, 7 | 12),
            Self::Byte => dif == 1,
            Self::Timestamp => matches!(dif, 4 | 6),
            Self::Date => dif == 2,
            Self::Time => dif == 3,
            Self::Unmatched => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
pub enum RecordProtection {
    Clear,
    Encrypted,
    Mixed,
}

fn descriptor_count(coding: u8, data: &[u8]) -> Option<u64> {
    if Coding::Integer.accepts(coding) && data.last().is_some_and(|byte| byte & 0x80 == 0) {
        return data.iter().rev().try_fold(0u64, |count, byte| {
            count.checked_mul(256)?.checked_add(u64::from(*byte))
        });
    }
    if matches!(coding, 9..=12 | 14) {
        return data
            .iter()
            .rev()
            .flat_map(|byte| [byte >> 4, byte & 15])
            .try_fold(0u64, |count, digit| {
                (digit < 10).then_some(())?;
                count.checked_mul(10)?.checked_add(u64::from(digit))
            });
    }
    None
}

fn encryption_required(tag: &str) -> Option<bool> {
    match tag {
        "ID4!" | "ID5!" | "ID6!" | "ID11!" => None,
        "DT1!A" | "DT1!AR" | "DT1!AT" | "DT1!ART" | "DT1!Xn" | "DT2!A" | "DT2!AR" | "DT2!AT"
        | "DT2!ART" | "DT2!Xn" | "DT3!A" | "DT3!AR" | "DT3!AT" | "DT3!ART" | "DT4!A" | "DT4!AR"
        | "DT4!AT" | "DT4!ART" | "MM7!" | "MM12!" | "MM13!" | "MM14!" | "MM15!" | "MM16!"
        | "MM17!" => Some(true),
        _ => match tag.get(..2)? {
            "ID" => Some(false),
            "DP" | "DT" | "MM" => None,
            "AD" | "CA" | "CC" | "CD" | "CT" | "DC" | "EC" | "EJ" | "ER" | "EW" | "FR" | "HC"
            | "IR" | "LT" | "MO" | "ND" | "PD" | "PF" | "PH" | "PJ" | "PR" | "PT" | "PW" | "RE"
            | "RF" | "RH" | "RP" | "TB" | "TC" | "TS" | "VC" | "VF" | "VM" | "VV" | "WS" | "YD" => {
                Some(true)
            }
            _ => None,
        },
    }
}

pub struct Reading<H> {
    pub protection: Option<RecordProtection>,
    pub header: H,
    pub data: H,
    pub profile_time: Option<bool>,
    pub tariff: u64,
    pub subunit: u64,
    pub storage: u64,
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct MaximumPowerRegister {
    export: bool,
    recent: bool,
    tariff: u64,
    subunit: u64,
    storage: u64,
}

struct MaximumPowerPair {
    record: usize,
    power: bool,
    timestamp: bool,
}

#[derive(Default)]
pub(super) struct Readings {
    seen: BTreeSet<(&'static str, u64, u64, u64)>,
    due_readings: Vec<(usize, u64)>,
    due_dates: BTreeSet<u64>,
    sensor_types: BTreeSet<u8>,
    descriptors: BTreeMap<(bool, u64), u8>,
    selected_application: Option<(u16, u64, usize)>,
    first_descriptor: Option<usize>,
    maximum_registers: BTreeMap<MaximumPowerRegister, MaximumPowerPair>,
    profiles: profiles::Profiles,
    ranges: ranges::StorageRanges,
    range_encodings: BTreeSet<(u8, u64, u64, u64)>,
    pub(super) taf7: Option<Taf7Parameters>,
    first_duplicate: Option<usize>,
    unknown: Option<usize>,
}

impl Readings {
    pub(super) fn observe<H: AsRef<[u8]>>(
        &mut self,
        reading: Reading<H>,
        device: Option<u8>,
        n: usize,
        results: &mut Vec<RuleFinding>,
    ) {
        let header = reading.header.as_ref();
        let Some(end) = header.iter().position(|b| b & 0x80 == 0) else {
            self.unknown.get_or_insert(n);
            return;
        };
        let dif = header[0];
        let coding = dif & 15;
        let (dib, vib) = header.split_at(end + 1);
        if self.profiles.observe(&reading, end, self.taf7, n, results) {
            return;
        }
        if let Some(valid) = reading.profile_time {
            let invalid_sentinel = reading
                .data
                .as_ref()
                .split_last()
                .and_then(|(flags, bytes)| {
                    let (_, ticks) = bytes.split_first()?;
                    let (high, low) = ticks.split_last()?;
                    let zone = (flags << 3) as i8 >> 3;
                    Some(
                        (*high == 0x80 && low.iter().all(|byte| *byte == 0))
                            && (zone == -16 || (-12..=15).contains(&zone)),
                    )
                })
                == Some(true);
            results.push(RuleFinding {rule: "Compact profile base time", reference: "Annex G, G.2", source_url: primary!(105, 499),
                status: if valid || invalid_sentinel { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "The base time must use a supported time zone or transmission-relative offset. The invalid time sentinel is accepted.",
                record_number: Some(n)});
            return;
        }
        self.observe_descriptors(&reading, dib, vib, device, n, results);
        let historical_date =
            self.ranges
                .observe_date(&reading, vib, dif, end > 0 && header[end] == 0);
        let Some(tag) = identify(
            vib,
            reading.tariff,
            (dif >> 4) & 3,
            reading.storage,
            end > 0 && header[end] == 0,
            coding,
        ) else {
            if !historical_date {
                self.unknown.get_or_insert(n);
            }
            return;
        };
        let valid = tag.1.accepts(coding);
        if valid {
            self.ranges.observe(tag.0, &reading, vib, n);
        }
        if let Some(protection) = reading.protection {
            let mut requires_clear = false;
            let mut requires_encrypted = false;
            let mut optional = false;
            for candidate in tag.0.split(" / ") {
                match encryption_required(candidate) {
                    Some(true) => requires_encrypted = true,
                    Some(false) => requires_clear = true,
                    None => optional = true,
                }
            }
            if requires_clear || requires_encrypted {
                let accepted = optional
                    || requires_encrypted && protection == RecordProtection::Encrypted
                    || requires_clear && protection == RecordProtection::Clear;
                let rejected = requires_clear && protection != RecordProtection::Clear
                    || requires_encrypted && protection != RecordProtection::Encrypted;
                let ambiguous = accepted && rejected;
                results.push(RuleFinding {
                    rule: "Data point encryption",
                    reference: "§9.1 / Annex B.2",
                    source_url: crate::sources::tag_page(tag.0),
                    status: if ambiguous {
                        RuleStatus::CouldNotCheck
                    } else if rejected {
                        RuleStatus::Failed
                    } else {
                        RuleStatus::Passed
                    },
                    detail: if ambiguous {
                        "The candidate tags have different protection requirements for these bytes."
                    } else if requires_encrypted {
                        "Annex B requires encryption for the protected tag."
                    } else {
                        "Annex B requires this tag to be in cleartext."
                    },
                    record_number: Some(n),
                });
            }
        }
        results.push(RuleFinding {
            rule: tag.0,
            reference: "Annex B, MB-Data-Tags",
            source_url: crate::sources::tag_page(tag.0),
            status: if valid {
                RuleStatus::Passed
            } else {
                RuleStatus::Failed
            },
            detail: if valid {
                "The data field uses a listed encoding."
            } else {
                "The DIF data encoding is outside the list for this tag."
            },
            record_number: Some(n),
        });
        if let Some(
            device @ (0x02..=0x04 | 0x06..=0x08 | 0x0a..=0x0d | 0x15..=0x16 | 0x20..=0x21),
        ) = device.filter(|_| reading.subunit == 0)
        {
            let applicable = tag
                .0
                .split(" / ")
                .filter_map(|name| meter_tag(name, device))
                .any(|listed| listed);
            results.push(RuleFinding {
                rule: "Tag for device type",
                reference: "Annex B, MB-Data-Tags",
                source_url: crate::sources::tag_page(tag.0),
                status: if applicable {
                    RuleStatus::Passed
                } else {
                    RuleStatus::NotApplicable
                },
                detail: if applicable {
                    "The tag is listed for this device type."
                } else {
                    "The Annex B meter column does not assign this tag to the device type."
                },
                record_number: Some(n),
            });
        }
        if device == Some(0x0d)
            && reading.subunit == 0
            && matches!(tag.0, "EJ1!T" | "EW1!T" | "VM1!T")
        {
            results.push(RuleFinding {
                rule: "Combined meter cooling tariff", reference: "Annex B, note 15", source_url: annex_b!(13, 818),
                status: if reading.tariff == 1 { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "The combined heat and cooling meter uses tariff 1 for this cooling register.", record_number: Some(n),
            });
        }
        if device == Some(0x02)
            && let Some((kind, suffix)) = tag.0.split_once('!')
            && matches!(suffix, "A" | "AT" | "AR" | "ART")
            && matches!(kind, "PW1" | "PW3" | "DT1" | "DT2" | "DT3" | "DT4")
        {
            let export = matches!(kind, "PW3" | "DT3" | "DT4");
            let recent = suffix.contains('R');
            let register = MaximumPowerRegister {
                export,
                recent,
                tariff: reading.tariff,
                subunit: reading.subunit,
                storage: reading.storage,
            };
            let pair = self
                .maximum_registers
                .entry(register)
                .or_insert(MaximumPowerPair {
                    record: n,
                    power: false,
                    timestamp: false,
                });
            if kind.starts_with("PW") {
                pair.power = true;
            } else {
                pair.timestamp = true;
            }
        }
        if matches!(tag.0, "DC1!" | "DC2!") {
            check_disconnector_subunit(device, reading.subunit, n, results);
        }
        if tag.0 == "MM6!"
            && valid
            && let Some((application, block)) =
                crate::telegram::selected_application(reading.data.as_ref())
        {
            self.selected_application = Some((application, block, n));
        }
        self.check_value(&reading, &tag, coding, device, n, results);
        let newly_seen =
            self.seen
                .insert((tag.0, reading.tariff, reading.subunit, reading.storage));
        let distinct = if let ("YD3!P", [0xfd, kind]) = (tag.0, vib) {
            self.range_encodings
                .insert((*kind, reading.tariff, reading.subunit, reading.storage))
        } else {
            newly_seen
        };
        if !distinct {
            self.first_duplicate.get_or_insert(n);
        }
        if tag.0.ends_with("!D") && tag.0 != "DT2!D" {
            self.due_readings.push((n, reading.subunit));
        }
        if tag.0 == "DT2!D" && valid {
            self.due_dates.insert(reading.subunit);
        }
        if tag.0 == "YD1!T" {
            results.push(RuleFinding {
                rule: "Tariff descriptor subunit",
                reference: "Annex K, K.4",
                source_url: primary!(126, 190),
                status: if reading.subunit == 0 {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail: "A tariff descriptor requires subunit zero.",
                record_number: Some(n),
            });
        }
        if tag.0 == "YD1!" {
            results.push(RuleFinding {
                rule: "Subunit descriptor address",
                reference: "Annex B, note 17",
                source_url: annex_b!(13, 922),
                status: if reading.subunit > 0 {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail: "A subunit descriptor requires a nonzero subunit.",
                record_number: Some(n),
            });
        }
    }

    pub(super) fn fully_mapped(&self) -> bool {
        self.unknown.is_none()
    }

    pub(super) fn check_selected_content(
        &self,
        content: u8,
        index: u8,
        findings: &mut Vec<RuleFinding>,
    ) {
        let Some((application, _, number)) = self.selected_application else {
            return;
        };
        let expected = match application {
            7 => (2, 0),
            16..=18 => (0, application - 15),
            26..=29 => (0, application - 22),
            46..=49 => (1, application - 46),
            54..=57 => (1, application - 46),
            _ => return,
        };
        findings.push(RuleFinding {
            rule: "Selected application message content", reference: "§8.10, Table 41", source_url: primary!(80, 388),
            status: if (content, index) == (0, 0) { RuleStatus::CouldNotCheck }
                else if expected == (content, u16::from(index)) { RuleStatus::Passed } else { RuleStatus::Failed },
            detail: "The application reported in MM6! must agree with the content and index on a differentiated data push.",
            record_number: Some(number),
        });
    }

    pub(super) fn finish(&self, unavailable: Option<&'static str>, results: &mut Vec<RuleFinding>) {
        if let Some(n) = self.unknown {
            results.push(RuleFinding {
                rule: "Unmapped record encoding", reference: "§8.4.4", source_url: primary!(75, 148), status: RuleStatus::CouldNotCheck,
                detail: "The record does not match a supported Annex B reading. Other EN 13757 data points are permitted.",
                record_number: Some(n),
            });
        }
        if let (Some((7, block, _)), Some(number)) =
            (self.selected_application, self.first_descriptor)
        {
            results.push(RuleFinding {
                rule: "Static application descriptor block", reference: "§8.10", source_url: primary!(80, 257),
                status: if block <= 1 { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "When MM6! identifies static application 7, its descriptors must be in block 0 or 1.", record_number: Some(number),
            });
        }
        for pair in self.maximum_registers.values() {
            results.push(RuleFinding {
                rule: "Power maximum and time of maximum", reference: "Annex A, note 2",
                source_url: "https://oms-group.org/wp-content/uploads/2024/05/OMS-Spec_Vol2_AnnexA_F121.pdf#page=6&view=FitH,668",
                status: if pair.power && pair.timestamp { RuleStatus::Passed } else if unavailable.is_none() && self.unknown.is_none() { RuleStatus::Failed } else { RuleStatus::CouldNotCheck },
                detail: "An electricity maximum-power register and its time must occur together for the same direction, tariff, subunit and storage number.",
                record_number: Some(pair.record),
            });
        }
        self.ranges
            .finish(unavailable.is_none() && self.fully_mapped(), results);
        if let Some(parameters) = self.taf7 {
            self.profiles
                .finish(parameters, unavailable.is_none(), results);
        }
        let (status, detail) = if self.first_duplicate.is_some() {
            (
                RuleStatus::Failed,
                "This reading repeats a tag with the same tariff, storage number and subunit.",
            )
        } else if let Some(reason) = unavailable {
            (RuleStatus::CouldNotCheck, reason)
        } else if self.seen.is_empty() {
            (
                RuleStatus::NotApplicable,
                "No listed readings were identified.",
            )
        } else {
            (
                RuleStatus::Passed,
                "Each identified Annex B reading occurs once.",
            )
        };
        results.push(RuleFinding {
            rule: "Unique Annex B readings",
            reference: "§8.4.4",
            source_url: primary!(75, 148),
            status,
            detail,
            record_number: self.first_duplicate,
        });
        for &(n, subunit) in &self.due_readings {
            let present = self.due_dates.contains(&subunit);
            results.push(RuleFinding {
                rule: "Due date for billing value",
                reference: "§8.4.5.1",
                source_url: primary!(76, 319),
                status: if present {
                    RuleStatus::Passed
                } else {
                    RuleStatus::CouldNotCheck
                },
                detail: if present {
                    "The corresponding DT2!D is present."
                } else {
                    "Check the static telegram for the corresponding DT2!D."
                },
                record_number: Some(n),
            });
        }
    }
}

pub(super) fn check_disconnector_subunit(
    device: Option<u8>,
    subunit: u64,
    number: usize,
    findings: &mut Vec<RuleFinding>,
) {
    let status = match device {
        Some(0x01..=0x07 | 0x0a..=0x0d | 0x10 | 0x15..=0x17 | 0x28) if subunit == 1 => {
            RuleStatus::Passed
        }
        Some(0x01..=0x07 | 0x0a..=0x0d | 0x10 | 0x15..=0x17 | 0x28) => RuleStatus::Failed,
        Some(0x20 | 0x21) => return,
        _ => RuleStatus::CouldNotCheck,
    };
    findings.push(RuleFinding {
        rule: "Integrated disconnector subunit",
        reference: "Annex M, M.2.4, Table M.8",
        source_url: annex_m!(20, 102),
        status,
        detail: "A disconnector integrated in a meter uses subunit 1.",
        record_number: Some(number),
    });
}

pub(super) fn manual_release(data: &[u8], number: usize, findings: &mut Vec<RuleFinding>) {
    if data
        .split_first()
        .is_some_and(|(value, rest)| *value == 2 && rest.iter().all(|byte| *byte == 0))
    {
        findings.push(RuleFinding {
            rule: "Manual switch-on capability",
            reference: "Annex M, Table M.8 note a",
            source_url: annex_m!(20, 102),
            status: RuleStatus::CouldNotCheck,
            detail: "Value 2 requires manual switch-on. Check the declared disconnector type.",
            record_number: Some(number),
        });
    }
}
