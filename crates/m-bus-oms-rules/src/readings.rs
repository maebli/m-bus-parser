use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::catalog::{identify, meter_tag};
use crate::telegram::{RuleFinding, RuleStatus};

pub mod history;
mod profiles;

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Taf7Parameters {
    pub measuring_period_ms: Option<u64>,
    pub registration_period_ms: Option<u64>,
    pub acceptance_window_ms: Option<u64>,
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

type TagGroup = (&'static str, &'static [&'static str], bool);

#[derive(Default)]
pub(super) struct Readings {
    seen: BTreeSet<(&'static str, u64, u64, u64)>,
    due_readings: Vec<(usize, u64)>,
    due_dates: BTreeSet<u64>,
    sensor_types: BTreeSet<u8>,
    descriptors: BTreeMap<(bool, u64), u8>,
    maximum_registers: BTreeMap<MaximumPowerRegister, MaximumPowerPair>,
    profiles: profiles::Profiles,
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
        let vib = &header[end + 1..];
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

        if vib == [0xfd, 0x23]
            && let [index] = reading.data.as_ref()
            && coding == 1
        {
            let tariff = *index >= 128;
            let (number, other) = if tariff {
                (reading.tariff, reading.subunit)
            } else {
                (reading.subunit, reading.tariff)
            };
            let address_valid = number > 0
                && other == 0
                && reading.storage == 0
                && dif & 0x30 == 0
                && !(end > 0 && header[end] == 0);
            results.push(RuleFinding {
                rule: "Descriptor register address", reference: "Annex K, K.3–K.4", source_url: primary!(125, 836),
                status: if address_valid { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "The index selects either a nonzero tariff or a nonzero subunit. The other register, storage and function must be zero.", record_number: Some(n),
            });
            if address_valid
                && let Some(previous) = self.descriptors.insert((tariff, number), *index)
            {
                results.push(RuleFinding {
                    rule: "Stable descriptor definition",
                    reference: "Annex K, K.1",
                    source_url: primary!(123, 469),
                    status: if previous == *index {
                        RuleStatus::Passed
                    } else {
                        RuleStatus::Failed
                    },
                    detail: "Each declared tariff or subunit keeps the same index definition.",
                    record_number: Some(n),
                });
            }
            let permitted: &[u8] = match index {
                1..=4 => &[0x02],
                160..=163 | 232..=233 => &[0x04, 0x0a, 0x0b, 0x0c, 0x0d],
                164 | 191 => &[
                    0x03, 0x04, 0x06, 0x07, 0x0a, 0x0b, 0x0c, 0x0d, 0x15, 0x16, 0x28,
                ],
                165 | 190 | 230..=231 => &[0x02, 0x04, 0x0a, 0x0b, 0x0c, 0x0d],
                _ => &[],
            };
            if !permitted.is_empty() {
                results.push(RuleFinding {
                    rule: "Descriptor index for medium",
                    reference: "Annex K, Tables K.4–K.5",
                    source_url: if tariff {
                        primary!(126, 190)
                    } else {
                        primary!(125, 361)
                    },
                    status: match device {
                        Some(d) if permitted.contains(&d) => RuleStatus::Passed,
                        Some(_) => RuleStatus::Failed,
                        None => RuleStatus::CouldNotCheck,
                    },
                    detail: "The declared descriptor index must be assigned to the meter medium.",
                    record_number: Some(n),
                });
            }
        }
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
        let data = reading.data.as_ref();
        if tag.0 == "ID11!"
            && valid
            && device == Some(0x1f)
            && reading.subunit == 0
            && let [length @ 0xe0..=0xef, types @ ..] = data
            && usize::from(length - 0xe0) == types.len()
        {
            self.sensor_types.extend(types.iter().copied());
            let listed = types
                .iter()
                .any(|code| matches!(code, 1..=3 | 5..=15 | 18..=21));
            results.push(RuleFinding {
                rule: "OMS sensor designation",
                reference: "§8.4.5.3 / Table 9",
                source_url: primary!(77, 89),
                status: if listed {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail: "A device of type 1Fh requires a subdevice type from Table 9.",
                record_number: Some(n),
            });
        }
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
                matches!(data, [length @ 0xe0..=0xef, bytes @ ..] if usize::from(length - 0xe0) == bytes.len()),
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
        const HEAT: TagGroup = ("One heat register (EJ1! or EW1!)", &["EJ1!", "EW1!"], true);
        let groups: &[TagGroup] = match device {
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
            Some(0x18 | 0x1a | 0x1d..=0x1f) => {
                self.required_sensor_tags(device, false, complete, results);
                return;
            }
            Some(0x20 | 0x21) => &[
                ("Control state (DC1!)", &["DC1!"], false),
                ("Physical state (DC2!)", &["DC2!"], false),
            ],
            _ => return,
        };
        self.check_tag_groups(groups, complete, results);
    }

    pub(super) fn required_sensor_tags(
        &self,
        device: Option<u8>,
        static_message: bool,
        complete: bool,
        results: &mut Vec<RuleFinding>,
    ) {
        let direct = match device {
            Some(0x18) => Some(21),
            Some(0x1a) => Some(20),
            Some(0x1d) => Some(18),
            Some(0x1e) => Some(19),
            Some(0x1f) => None,
            _ => return,
        };
        if device == Some(0x1f) && !static_message {
            self.check_tag_groups(
                &[("Subdevice types (ID11!)", &["ID11!"], false)],
                complete,
                results,
            );
            if self
                .sensor_types
                .iter()
                .any(|code| matches!(code, 1..=3 | 5..=15))
            {
                self.check_tag_groups(
                    &[("Standard error flags (MM3!)", &["MM3!"], false)],
                    complete,
                    results,
                );
            } else if self.sensor_types.is_empty() {
                results.push(RuleFinding {
                    rule: "Sensor-specific data points", reference: "Annex B, B.2.3", source_url: annex_b!(35),
                    status: RuleStatus::CouldNotCheck, detail: "A readable ID11! declaration is needed to select the sensor requirements.", record_number: None,
                });
            }
        }
        const RUNTIME: TagGroup = (
            "Operating or on time (DP5! or DP6!)",
            &["DP5!", "DP6!"],
            false,
        );
        for sensor in direct.into_iter().chain(self.sensor_types.iter().copied()) {
            let groups: &[TagGroup] = match (sensor, static_message) {
                (21, true) => &[
                    ("Static upper pressure threshold (PR3!)", &["PR3!"], false),
                    ("Static lower pressure threshold (PR4!)", &["PR4!"], false),
                ],
                (_, true) => &[],
                (1, _) => &[("Humidity (RH1!)", &["RH1!"], false)],
                (2, _) => &[("Moisture (MO1!)", &["MO1!"], false)],
                (3, _) => &[("Temperature (TC4!)", &["TC4!"], false)],
                (5, _) => &[("Light level (LT1! or LT2!)", &["LT1!", "LT2!"], false)],
                (6, _) => &[("Rainfall (RF1!)", &["RF1!"], false)],
                (7, _) => &[("pH (PH1!)", &["PH1!"], false)],
                (8, _) => &[("Wind speed (WS1!)", &["WS1!"], false)],
                (9, _) => &[("Glass break (ND1! or TS1!)", &["ND1!", "TS1!"], false)],
                (10 | 11, _) => &[("Contact state (MM16!)", &["MM16!"], false)],
                (13, _) => &[("Air quality (VC1! or VC2!)", &["VC1!", "VC2!"], false)],
                (14, _) => &[("Carbon dioxide (CC2!)", &["CC2!"], false)],
                (15, _) => &[("Turbidity (TB1!)", &["TB1!"], false)],
                (18, _) => &[
                    RUNTIME,
                    ("CO alarm state (MM14!)", &["MM14!"], false),
                    ("Carbon monoxide (CC1!)", &["CC1!"], false),
                ],
                (19, _) => &[
                    RUNTIME,
                    ("Heat alarm state (MM17!)", &["MM17!"], false),
                    ("Temperature (TC4!)", &["TC4!"], false),
                ],
                (20, _) => &[RUNTIME, ("Smoke alarm state (MM15!)", &["MM15!"], false)],
                (21, _) => &[
                    ("Pressure state (MM13!)", &["MM13!"], false),
                    ("Pressure (PR1! or PR2!)", &["PR1!", "PR2!"], false),
                ],
                _ => &[],
            };
            self.check_tag_groups(groups, complete, results);
        }
    }

    fn check_tag_groups(
        &self,
        groups: &[TagGroup],
        complete: bool,
        results: &mut Vec<RuleFinding>,
    ) {
        for &(rule, tags, exclusive) in groups {
            if results.iter().any(|finding| finding.rule == rule) {
                continue;
            }
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
                source_url: if rule.starts_with("Static ") {
                    annex_b!(37)
                } else {
                    primary!(75, 525)
                },
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

    pub(super) fn consumer_information(
        &self,
        device: Option<u8>,
        complete: bool,
        results: &mut Vec<RuleFinding>,
    ) {
        let mut require = |rule, tags: &[&str]| {
            let present = self.seen.iter().any(|(tag, tariff, subunit, _)| {
                *subunit == 0
                    && (*tariff == 0 || *tariff == 1 && device == Some(0x0d))
                    && tags.contains(tag)
            });
            results.push(RuleFinding {
                rule,
                reference: "Annex M, Table M.27",
                source_url: annex_m!(41, 79),
                status: if present {
                    RuleStatus::Passed
                } else if complete {
                    RuleStatus::Failed
                } else {
                    RuleStatus::CouldNotCheck
                },
                detail: "This consumer information message requires one of the indicated readings.",
                record_number: None,
            });
        };
        match device {
            Some(0x02) => {
                require("Consumer energy import (EW1!)", &["EW1!"]);
                require("Consumer billing energy (EW1!R)", &["EW1!R"]);
            }
            Some(0x03) => require(
                "Consumer gas volume (VM1!, VM2! or VM3!)",
                &["VM1!", "VM2!", "VM3!"],
            ),
            Some(0x08) => {
                require("Consumer due date (DT2!D)", &["DT2!D"]);
                require("Consumer allocation (HC1!)", &["HC1!"]);
                require("Consumer billing allocation (HC1!D)", &["HC1!D"]);
            }
            Some(0x04 | 0x0a..=0x0d) => {
                require("Consumer due date (DT2!D)", &["DT2!D"]);
                require(
                    "Consumer heat register (EJ1!, EW1! or EC1!)",
                    &["EJ1!", "EW1!", "EC1!"],
                );
                require(
                    "Consumer billing heat (EJ1!D, EW1!D or EC1!D)",
                    &["EJ1!D", "EW1!D", "EC1!D"],
                );
                if device == Some(0x0d) {
                    require(
                        "Consumer cooling register (EJ1!T, EW1!T, EJ2!, EW2! or EC2!)",
                        &["EJ1!T", "EW1!T", "EJ2!", "EW2!", "EC2!"],
                    );
                    require(
                        "Consumer billing cooling (EJ1!DT, EW1!DT, EJ2!D, EW2!D or EC2!D)",
                        &["EJ1!DT", "EW1!DT", "EJ2!D", "EW2!D", "EC2!D"],
                    );
                }
                require("Consumer flow temperature (TC1!)", &["TC1!"]);
                require("Consumer return temperature (TC2!)", &["TC2!"]);
            }
            Some(0x06 | 0x07 | 0x15 | 0x16 | 0x28) => {
                require("Consumer due date (DT2!D)", &["DT2!D"]);
                require("Consumer water volume (VM1!)", &["VM1!"]);
                require("Consumer billing volume (VM1!D)", &["VM1!D"]);
            }

            _ => {}
        }

        if device == Some(0x02) {
            let available = self
                .seen
                .iter()
                .any(|(tag, _, subunit, _)| *subunit == 0 && tag.starts_with("EW2!"));
            let present = self.seen.contains(&("EW2!", 0, 0, 0));
            results.push(RuleFinding {
                rule: "Consumer export energy (EW2!)",
                reference: "Annex M, Table M.27",
                source_url: annex_m!(41, 79),
                status: if present {
                    RuleStatus::Passed
                } else if complete && available {
                    RuleStatus::Failed
                } else {
                    RuleStatus::CouldNotCheck
                },
                detail: if available {
                    "The meter reports export energy and must provide the current EW2! register."
                } else {
                    "Export availability must be established before EW2! is required."
                },
                record_number: None,
            });
        }
        let standalone = matches!(device, Some(0x20 | 0x21));
        let subunit = u64::from(!standalone);
        let physical = self.seen.contains(&("DC2!", 0, subunit, 0));
        let control = self.seen.contains(&("DC1!", 0, subunit, 0));
        results.push(RuleFinding {
            rule: "Consumer disconnector state (DC1!)", reference: "Annex M, Table M.27 note b", source_url: annex_m!(41, 79),
            status: if control { RuleStatus::Passed } else if complete && (standalone || physical) { RuleStatus::Failed } else { RuleStatus::CouldNotCheck },
            detail: if standalone || physical || control { "A device with a breaker or valve must report its current control state." } else { "The availability of a breaker or valve must be established before DC1! is required." },
            record_number: None,
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
        for pair in self.maximum_registers.values() {
            results.push(RuleFinding {
                rule: "Power maximum and time of maximum", reference: "Annex A, note 2",
                source_url: "https://oms-group.org/wp-content/uploads/2024/05/OMS-Spec_Vol2_AnnexA_F121.pdf#page=6&view=FitH,668",
                status: if pair.power && pair.timestamp { RuleStatus::Passed } else if unavailable.is_none() && self.unknown.is_none() { RuleStatus::Failed } else { RuleStatus::CouldNotCheck },
                detail: "An electricity maximum-power register and its time must occur together for the same direction, tariff, subunit and storage number.",
                record_number: Some(pair.record),
            });
        }
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
