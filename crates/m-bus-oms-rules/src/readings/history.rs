use crate::telegram::{RuleFinding, RuleStatus, TelegramReport};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

#[derive(Default)]
pub struct DeviceReadingHistory {
    initial_address: Option<Vec<u8>>,
    descriptors: BTreeMap<(bool, u64), u8>,
    static_due_dates: BTreeSet<u64>,
}

pub struct RegisterValue {
    pub subunit: u64,
    pub tariff: u64,
    pub data: Vec<u8>,
}

impl DeviceReadingHistory {
    pub fn observe(
        &mut self,
        report: &mut TelegramReport,
        static_push: bool,
        consecutive: bool,
        mut record: impl FnMut(usize) -> Option<RegisterValue>,
    ) {
        let mut correlated = Vec::new();
        for finding in &mut report.findings {
            let Some(number) = finding.record_number else {
                continue;
            };
            if finding.rule == "Due date for billing value"
                && matches!(finding.status, RuleStatus::CouldNotCheck)
                && consecutive
            {
                if record(number)
                    .is_some_and(|reading| self.static_due_dates.contains(&reading.subunit))
                {
                    finding.status = RuleStatus::Passed;
                    finding.detail = "An earlier static telegram in the consecutive capture supplied DT2!D for this meter and subunit.";
                    report.assessed += 1;
                    report.passed += 1;
                    report.unchecked -= 1;
                }
                continue;
            }
            if !matches!(finding.status, RuleStatus::Passed)
                || !matches!(finding.rule, "ID7!" | "YD1!" | "YD1!T" | "DT2!D")
            {
                continue;
            }
            let Some(reading) = record(number) else {
                continue;
            };
            let (valid, rule, reference, source_url, detail) = match finding.rule {
                "ID7!" if matches!(reading.data.as_slice(), [14, value @ ..] if value.len() == 14 && value.is_ascii()) =>
                {
                    let Some(previous) = &self.initial_address else {
                        self.initial_address = Some(reading.data);
                        continue;
                    };
                    (
                        previous == &reading.data,
                        "Initial DIN address across capture",
                        "§8.4.5.2",
                        primary!(76, 948),
                        "The initial DIN address must stay unchanged for this meter.",
                    )
                }
                "YD1!" | "YD1!T" => {
                    let [index] = reading.data.as_slice() else {
                        continue;
                    };
                    let tariff = finding.rule == "YD1!T";
                    let number = if tariff {
                        reading.tariff
                    } else {
                        reading.subunit
                    };
                    if number == 0 || tariff != (*index >= 128) {
                        continue;
                    }
                    let previous = self.descriptors.entry((tariff, number)).or_insert(*index);
                    if *previous == *index {
                        continue;
                    }
                    (
                        false,
                        "Descriptor definition across capture",
                        "Annex K, K.1",
                        primary!(123, 469),
                        "A previously observed tariff or subunit descriptor has a different index for the same meter.",
                    )
                }
                "DT2!D" if static_push => {
                    self.static_due_dates.insert(reading.subunit);
                    continue;
                }
                _ => continue,
            };
            correlated.push(RuleFinding {
                rule,
                reference,
                source_url,
                detail,
                status: if valid {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                record_number: Some(number),
            });
        }
        for finding in correlated {
            report.push(finding);
        }
    }
}
