use alloc::vec::Vec;

use super::{Reading, Readings, descriptor_count};
use crate::telegram::{RuleFinding, RuleStatus};

impl Readings {
    pub(super) fn observe_descriptors<H: AsRef<[u8]>>(
        &mut self,
        reading: &Reading<H>,
        dib: &[u8],
        vib: &[u8],
        device: Option<u8>,
        n: usize,
        results: &mut Vec<RuleFinding>,
    ) {
        let dif = dib[0];
        let coding = dif & 15;
        let recent = dib.len() > 1 && dib.last() == Some(&0);

        if matches!(vib, [0xfd, 0x20..=0x29]) {
            self.first_descriptor.get_or_insert(n);
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
            let address_valid =
                number > 0 && other == 0 && reading.storage == 0 && dif & 0x30 == 0 && !recent;
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
            let address_valid = reading.tariff == 0
                && reading.subunit == 0
                && dif & 0x30 == 0
                && (1..=199).contains(&reading.storage)
                && !recent;
            results.push(RuleFinding {
                rule: "Storage descriptor address", reference: "Annex K, K.2", source_url: primary!(124, 90),
                status: if address_valid { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "The Annex B storage descriptor uses storage 1–199 with zero tariff, function and subunit and no final zero DIFE.",
                record_number: Some(n),
            });
            if *descriptor == 0x22 {
                results.push(RuleFinding {
                    rule: "Storage block size", reference: "Annex K, Table K.3", source_url: primary!(124, 790),
                    status: if descriptor_count(coding, reading.data.as_ref()).is_some_and(|count| count > 0) { RuleStatus::Passed } else { RuleStatus::Failed },
                    detail: "The declared storage block size must be a positive count of the applied storage numbers.", record_number: Some(n),
                });
            }
            if matches!(descriptor, 0x20 | 0x21) {
                results.push(RuleFinding {
                    rule: "Storage range endpoint", reference: "Annex K, K.2.2", source_url: primary!(124, 790),
                    status: if coding == 0 && reading.data.as_ref().is_empty() { RuleStatus::Passed } else { RuleStatus::Failed },
                    detail: "A first or last cyclic storage number is carried in the DIB and has no data value.",record_number: Some(n),
                });
            }
        }
    }
}
