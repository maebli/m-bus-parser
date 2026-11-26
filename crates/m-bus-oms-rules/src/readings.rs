use alloc::{collections::BTreeSet, vec::Vec};

use crate::catalog::{identify, meter_tag};
use crate::telegram::{RuleFinding, RuleStatus};

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

#[derive(Default)]
pub(super) struct Readings {
    seen: BTreeSet<(&'static str, u64, u64, u64)>,
    due_readings: Vec<(usize, u64)>,
    due_dates: BTreeSet<u64>,
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
        if let Some(valid) = reading.profile_time {
            results.push(RuleFinding {rule: "Compact profile base time", reference: "Annex R, R.2.2", source_url: annex_r!(9, 331),
                status: if valid { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "The base time must decode to a timestamp or a transmission-relative offset.",
                record_number: Some(n)});
            return;
        }
        let vib = &header[end + 1..];
        if let [0xfd, descriptor @ (0x20..=0x22 | 0x24..=0x29)] = vib {
            let interval = *descriptor >= 0x24;
            let address_valid = reading.tariff == 0
                && reading.subunit == 0
                && dif & 0x30 == 0
                && (!interval || reading.storage != 0);
            results.push(RuleFinding {
                rule: "Storage descriptor address", reference: "Annex K, K.2", source_url: primary!(124, 90),
                status: if address_valid { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "Tariff, function and subunit must be zero. A storage interval also requires a nonzero storage number.",
                record_number: Some(n),
            });
            if matches!(descriptor, 0x20 | 0x21) {
                results.push(RuleFinding {
                    rule: "Storage range endpoint", reference: "Annex K, K.2.2", source_url: primary!(124, 790),
                    status: if coding == 0 && reading.data.as_ref().is_empty() { RuleStatus::Passed } else { RuleStatus::Failed },
                    detail: "A first or last cyclic storage number is carried in the DIB and has no data value.",record_number: Some(n),
                });
            }
        }
        let Some(tag) = identify(
            vib,
            reading.tariff,
            (dif >> 4) & 3,
            reading.storage,
            end > 0 && header[end] == 0,
            coding,
        ) else {
            self.unknown.get_or_insert(n);
            return;
        };
        let valid = tag.1.accepts(coding);
        if let Some(protection) = reading.protection {
            let mut candidates = tag.0.split(" / ").map(encryption_required);
            if let Some(Some(required)) = candidates.next()
                && candidates.all(|candidate| candidate == Some(required))
            {
                let expected = if required {
                    RecordProtection::Encrypted
                } else {
                    RecordProtection::Clear
                };
                results.push(RuleFinding {
                    rule: "Data point encryption",
                    reference: "§9.1 / Annex B.2",
                    source_url: crate::sources::tag_page(tag.0),
                    status: if protection == expected {
                        RuleStatus::Passed
                    } else {
                        RuleStatus::Failed
                    },
                    detail: if required {
                        "Annex B requires this data point to be encrypted."
                    } else {
                        "Annex B requires this data point to be in cleartext."
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
        if matches!(tag.0, "DC1!" | "DC2!") {
            check_disconnector_subunit(device, reading.subunit, n, results);
        }
        let data = reading.data.as_ref();
        if matches!(tag.0, "ID7!" | "ID8!")
            && let [14, serial @ ..] = data
            && let Some(manufacturer) = serial.get(10..13).filter(|_| serial.len() == 14)
        {
            results.push(RuleFinding {
                rule: "DIN manufacturer code",
                reference: "§3.2, Table 6 / Annex B note 19",
                source_url: primary!(36, 580),
                status: if manufacturer.iter().all(u8::is_ascii_uppercase) {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail: "The DIN address contains three uppercase manufacturer letters.",
                record_number: Some(n),
            });
        }
        if tag.0 == "DC1!" && valid {
            manual_release(data, n, results);
        }
        let check = match tag.0 {
            "DC1!" | "DC2!" if valid => Some((
                "Disconnector state",
                "Annex M, Table M.8",
                annex_m!(20, 102),
                data.split_first().is_some_and(|(first, rest)| {
                    *first <= if tag.0 == "DC1!" { 2 } else { 1 } && rest.iter().all(|b| *b == 0)
                }),
                "Control states are 0, 1 or 2. Physical states are 0 or 1.",
            )),
            "ID7!" | "ID8!" => Some((
                "DIN address format",
                "Annex B, note 19",
                annex_b!(13, 967),
                matches!(data, [14, text @ ..] if text.len() == 14 && text.is_ascii()),
                "A DIN address requires 14 ASCII characters.",
            )),
            "ID11!" => Some((
                "Subdevice type encoding",
                "Annex B, note 22",
                annex_b!(14, 144),
                matches!(data.first(), Some(0xe0..=0xef)),
                "The length field must indicate a binary value.",
            )),
            "YD1!" => Some((
                "Subunit index",
                "Annex K, Table K.4",
                primary!(125, 361),
                matches!(data, [0..=29 | 100..=127]) && coding == 1,
                "The descriptor requires one byte with an assigned subunit index.",
            )),
            "YD1!T" => Some((
                "Tariff index",
                "Annex K, Table K.5",
                primary!(126, 190),
                matches!(
                    data,
                    [128..=139 | 150..=152 | 160..=165 | 190..=192 | 210..=212 | 230..=235]
                ) && coding == 1,
                "The descriptor requires one byte with an assigned tariff index.",
            )),
            _ => None,
        };
        if let Some((rule, reference, source_url, valid, detail)) = check {
            results.push(RuleFinding {
                rule,
                reference,
                source_url,
                status: if valid {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail,
                record_number: Some(n),
            });
        }
        if !self
            .seen
            .insert((tag.0, reading.tariff, reading.subunit, reading.storage))
        {
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

    pub(super) fn required_meter_tags(
        &self,
        device: Option<u8>,
        complete: bool,
        results: &mut Vec<RuleFinding>,
    ) {
        type Group = (&'static str, &'static [&'static str], bool);
        const HEAT: Group = ("One heat register (EJ1! or EW1!)", &["EJ1!", "EW1!"], true);
        const RUNTIME: Group = (
            "Operating or on time (DP5! or DP6!)",
            &["DP5!", "DP6!"],
            false,
        );
        let groups: &[Group] = match device {
            Some(0x02) => &[(
                "Electricity register (EW1!, EW2! or EW3!)",
                &["EW1!", "EW2!", "EW3!"],
                false,
            )],
            Some(0x03) => &[(
                "Gas volume (VM1!, VM2! or VM3!)",
                &["VM1!", "VM2!", "VM3!"],
                false,
            )],
            Some(0x04 | 0x0a..=0x0c) => &[HEAT],
            Some(0x0d) => &[
                HEAT,
                (
                    "Second energy register (EJ1!T, EW1!T, EJ2! or EW2!)",
                    &["EJ1!T", "EW1!T", "EJ2!", "EW2!"],
                    true,
                ),
            ],
            Some(0x06 | 0x07 | 0x15 | 0x16) => &[("Current volume (VM1!)", &["VM1!"], false)],
            Some(0x08) => &[
                ("Current allocation (HC1!)", &["HC1!"], false),
                ("Billing allocation (HC1!D)", &["HC1!D"], false),
                ("Billing date (DT2!D)", &["DT2!D"], false),
            ],
            Some(0x18) => &[
                ("Pressure state (MM13!)", &["MM13!"], false),
                ("Pressure (PR1! or PR2!)", &["PR1!", "PR2!"], false),
            ],
            Some(0x1a) => &[RUNTIME, ("Smoke alarm state (MM15!)", &["MM15!"], false)],
            Some(0x1d) => &[
                RUNTIME,
                ("CO alarm state (MM14!)", &["MM14!"], false),
                ("Carbon monoxide (CC1!)", &["CC1!"], false),
            ],
            Some(0x1e) => &[
                RUNTIME,
                ("Heat alarm state (MM17!)", &["MM17!"], false),
                ("Temperature (TC4!)", &["TC4!"], false),
            ],
            Some(0x1f) => &[
                ("Subdevice types (ID11!)", &["ID11!"], false),
                ("Standard error flags (MM3!)", &["MM3!"], false),
            ],
            Some(0x20 | 0x21) => &[
                ("Control state (DC1!)", &["DC1!"], false),
                ("Physical state (DC2!)", &["DC2!"], false),
            ],
            _ => return,
        };
        for &(rule, tags, exclusive) in groups {
            let count = tags
                .iter()
                .filter(|tag| {
                    self.seen.iter().any(|(seen, tariff, subunit, _)| {
                        seen == *tag && *subunit == 0 && *tariff <= 1
                    })
                })
                .count();
            let status = if exclusive && count > 1 {
                RuleStatus::Failed
            } else if !complete && (count == 0 || exclusive) {
                RuleStatus::CouldNotCheck
            } else if count == 0 {
                RuleStatus::Failed
            } else {
                RuleStatus::Passed
            };
            results.push(RuleFinding {
                rule,
                reference: "§8.4.4, Table 39 / Annex B",
                source_url: primary!(75, 525),
                status,
                detail: if exclusive {
                    "This telegram requires exactly one of the listed readings."
                } else {
                    "This telegram requires at least one listed reading."
                },
                record_number: None,
            });
        }
    }

    pub(super) fn finish(&self, unavailable: Option<&'static str>, results: &mut Vec<RuleFinding>) {
        if let Some(n) = self.unknown {
            results.push(RuleFinding {
                rule: "Unmapped record encoding", reference: "§8.4.4", source_url: primary!(75, 148), status: RuleStatus::CouldNotCheck,
                detail: "The record does not match a supported Annex B reading. Other EN 13757 data points are permitted.",
                record_number: Some(n),
            });
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
