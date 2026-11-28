use alloc::vec::Vec;

use super::Readings;
use crate::telegram::{RuleFinding, RuleStatus};

impl Readings {
    pub(crate) fn consumer_information(
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
}
