use alloc::vec::Vec;

use super::{Reading, Readings, Tag, manual_release};
use crate::telegram::{RuleFinding, RuleStatus};

impl Readings {
    pub(super) fn check_value<H: AsRef<[u8]>>(
        &mut self,
        reading: &Reading<H>,
        tag: &Tag,
        coding: u8,
        device: Option<u8>,
        n: usize,
        results: &mut Vec<RuleFinding>,
    ) {
        let valid = tag.1.accepts(coding);
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
    }
}
