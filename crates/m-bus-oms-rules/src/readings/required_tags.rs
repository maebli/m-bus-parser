use alloc::vec::Vec;

use super::Readings;
use crate::telegram::{RuleFinding, RuleStatus};

type TagGroup = (&'static str, &'static [&'static str], bool);

impl Readings {
    pub(crate) fn required_meter_tags(
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

    pub(crate) fn required_sensor_tags(
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
}
