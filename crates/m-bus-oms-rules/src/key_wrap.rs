use crate::telegram::{RuleFinding, RuleStatus, TelegramReport};

pub fn is_wrapped_structure(selector: u8) -> bool {
    matches!(selector, 1..=3 | 0x20 | 0x30)
}

pub fn valid_wire_length(selector: u8, len: usize) -> bool {
    match selector {
        1 => len == 32,
        2 => len == 16,
        3 => len == 24,
        0x20 => len >= 16 && len.is_multiple_of(8),
        0x30 => len >= 28 && (len - 4).is_multiple_of(8),
        _ => false,
    }
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Evidence {
    pub integrity: bool,
    pub structure: Option<bool>,
    pub use_case_fields: Option<bool>,
    pub counter: Option<bool>,
    pub wrapping_key_scope: Option<bool>,
}

impl Evidence {
    pub fn from_plaintext(
        selector: u8,
        control: u8,
        wrapping_key: u8,
        data: &[u8],
        outer_counter: Option<&[u8; 4]>,
    ) -> Self {
        let mut result = Self {
            integrity: true,
            ..Default::default()
        };
        result.structure = Some(match selector {
            1 => data.len() == 23,
            2 => data.len() == 7,
            3 => data.len() == 10 && data.last().is_some_and(|options| *options <= 1),
            0x20 => valid_key_list(data),
            0x30 => data.len() >= 10,
            _ => false,
        });
        let target_id = match (selector, data) {
            (1, [.., id, _]) if data.len() == 23 => Some(*id),
            (2 | 3, [_, _, _, _, _, id, ..]) if result.structure == Some(true) => Some(*id),
            _ => None,
        };
        result.wrapping_key_scope = target_id.and_then(|id| match wrapping_key {
            0x10 => Some(matches!(id, 0x10 | 0x12 | 0x14)),
            0x12 => Some(matches!(id, 0..=8 | 0x0f)),
            0x14 => Some(matches!(id, 0x18 | 0x1f..=0x27)),
            0x11 | 0x13 | 0x15 | 0x16 => Some(false),
            _ => None,
        });
        let uc08 = matches!(wrapping_key, 0x10..=0x15);
        match (selector, control, data) {
            (1, 0, [.., a, b, c, d, e, _, version]) if data.len() == 23 && uc08 => {
                result.use_case_fields =
                    Some([*a, *b, *c, *d, *e] == [0, 0, 0, 0x80, 0x30] && *version != 0xff);
            }
            (1, 0, [.., a, b, c, d, e, key, _]) if data.len() == 23 && wrapping_key == 0xff => {
                result.use_case_fields =
                    Some([*a, *b, *c, *d, *e] == [0, 0, 0, 0x80, 0x30] && *key == 0);
            }
            (3, 4, [a, b, c, d, e, new, _, old, _, options]) if uc08 || wrapping_key == 0xff => {
                result.use_case_fields = Some(
                    [*a, *b, *c, *d, *e] == [0, 0, 0, 0, 0x30]
                        && new == old
                        && (*new != 0 || *options == 1)
                        && (wrapping_key != 0xff || *new == 0),
                );
            }
            (0x30, _, _) => {
                result.counter = outer_counter
                    .zip(data.first_chunk::<4>())
                    .map(|(a, b)| a == b)
            }
            _ => {}
        }
        result
    }

    pub fn assess(evidence: Option<&Self>, selector: u8, key: u8, report: &mut TelegramReport) {
        let mut check = |rule, valid: Option<bool>, reference, source_url, detail| {
            report.push(RuleFinding {
                rule,
                reference,
                source_url,
                detail,
                record_number: None,
                status: match valid {
                    Some(true) => RuleStatus::Passed,
                    Some(false) => RuleStatus::Failed,
                    None => RuleStatus::CouldNotCheck,
                },
            })
        };
        let source = "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=39&view=FitH,72";
        let clear = key == 0xff;
        check(
            if clear {
                "SITP clear key container fields"
            } else {
                "SITP AES-KWP integrity"
            },
            evidence.map(|e| e.integrity),
            "Annex F.A, Table F.A.5",
            source,
            if clear {
                "The clear structure must contain the prescribed integrity constant, length and zero padding."
            } else {
                "AES key unwrap with padding must authenticate the container using the supplied wrapping key."
            },
        );
        check(
            "SITP unwrapped structure",
            evidence.and_then(|e| e.structure),
            "Annex F.A, Table F.A.5",
            source,
            "The available content must have the format and length specified for its data selector.",
        );
        if matches!(selector, 1..=3) && matches!(key, 0x10..=0x16) {
            check(
                "SITP wrapper key scope",
                evidence.and_then(|e| e.wrapping_key_scope),
                "Section 9.2.4, Table 45",
                primary!(85, 80),
                "The wrapping key must be assigned to the class of key being exchanged.",
            );
        }
        if matches!(selector, 1 | 3) && (clear || matches!(key, 0x10..=0x15)) {
            let (reference, url) = match (clear, selector) {
                (false, 1) => ("Annex M, Table M.29", annex_m!(43, 80)),
                (false, _) => ("Annex M, Table M.30", annex_m!(44, 80)),
                (true, 1) => (
                    "Annex F, Table F.22",
                    "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=26&view=FitH,113",
                ),
                (true, _) => (
                    "Annex F, Table F.25",
                    "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=27&view=FitH,113",
                ),
            };
            check(
                "SITP key update parameters",
                evidence.and_then(|e| e.use_case_fields),
                reference,
                url,
                "The transfer must defer activation. The combined operation must act immediately on the same key ID and reset the counter when switching the master key.",
            );
        }
        if selector == 0x30 {
            check(
                "SITP wrapped key counter",
                evidence.and_then(|e| e.counter),
                "Annex F.A, Table F.A.5",
                source,
                "The key counter preceding the encrypted structure must match the authenticated counter inside it.",
            );
        }
    }
}

fn valid_key_list(mut data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    while let [_, count, rest @ ..] = data {
        if *count == 0 {
            return false;
        }
        let Some(next) = rest.get(usize::from(*count)..) else {
            return false;
        };
        data = next;
    }
    data.is_empty()
}
