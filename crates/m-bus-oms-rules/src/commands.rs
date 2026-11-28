use alloc::vec::Vec;

use crate::{
    catalog::identify,
    readings::{Coding, Reading},
    telegram::{RuleFinding, RuleStatus},
};

#[derive(Default)]
pub(super) struct ReadoutSelection {
    address: Option<usize>,
    first_edit: Option<usize>,
}

impl ReadoutSelection {
    pub(super) fn finish(self, complete: bool, results: &mut Vec<RuleFinding>) {
        let Some(edit) = self.first_edit else {
            return;
        };
        results.push(RuleFinding {
            rule: "Readout-list application selection order", reference: "Annex M, Table M.37", source_url: annex_m!(48, 543),
            status: match self.address { Some(n) if n < edit => RuleStatus::Passed, Some(_) => RuleStatus::Failed, None if complete => RuleStatus::Failed, None => RuleStatus::CouldNotCheck },
            detail: "The setMM6! command must select the application before the first readout-list addition or deletion.", record_number: Some(edit),
        });
    }
}

pub(super) fn check_record<H: AsRef<[u8]>>(
    record: Reading<H>,
    target: Option<u8>,
    number: usize,
    findings: &mut Vec<RuleFinding>,
    selection: &mut ReadoutSelection,
) {
    let encoding = check_encoding(&record, number, target, findings, selection);
    findings.push(encoding);
}

fn check_encoding<H: AsRef<[u8]>>(
    record: &Reading<H>,
    number: usize,
    target: Option<u8>,
    findings: &mut Vec<RuleFinding>,
    selection: &mut ReadoutSelection,
) -> RuleFinding {
    let mut finding = RuleFinding {
        rule: "Command encoding",
        reference: "Annex B, MB-Command-Tags",
        source_url: annex_b!(39),
        status: RuleStatus::CouldNotCheck,
        detail: "No listed command was identified for this record.",
        record_number: Some(number),
    };
    let header = record.header.as_ref();
    let Some(end) = header.iter().position(|byte| byte & 0x80 == 0) else {
        return finding;
    };
    let dif = header[0];
    let coding = dif & 15;
    let vib = &header[end + 1..];
    let tag_for = |vib: &[u8]| {
        identify(
            vib,
            record.tariff,
            (dif >> 4) & 3,
            record.storage,
            end > 0 && header[end] == 0,
            coding,
        )
    };
    let mut base = Vec::from(vib);
    let mut action = None;
    let mut tag = tag_for(vib);
    if tag.is_none() && matches!(base.last(), Some(0x13 | 0x1e | 0x1f)) && base.len() >= 3 {
        let action_byte = base[base.len() - 2];
        if matches!(action_byte, 0x80..=0x82 | 0x86..=0x87 | 0x8c..=0x8d | 0x8f) {
            base.pop();
            if let Some(last) = base.last_mut() {
                *last &= 0x7f;
            }
        }
    }
    if tag.is_none() && base.len() >= 2 {
        action = base.pop();
        if let Some(last) = base.last_mut() {
            if *last & 0x80 == 0 {
                return finding;
            }
            *last &= 0x7f;
        }
        tag = tag_for(&base);
    }
    let Some(tag) = tag else { return finding };
    let valid = match (action, tag.0) {
        (Some(0x0c | 0x0d), _) => coding == 0,
        (Some(0x0f), "DT2!D" | "DT2!N" | "EJ1!" | "EW1!" | "ID4!" | "MM6!" | "VM1!") => coding == 0,
        (Some(0..=2), "EJ1!" | "EW1!" | "VM1!") => Coding::Number.accepts(coding),
        (Some(0), "DT2!N") => Coding::Date.accepts(coding),
        (Some(0) | None, "ID2!" | "ID8!" | "ID2! / ID8!") => coding == 7 || coding == 12,
        (None, "ID3!") | (Some(0) | None, "ID10!") => Coding::Byte.accepts(coding),
        (Some(0), "ID4!") => Coding::NumberOrVariable.accepts(coding),
        (Some(6), "MM2!" | "MM3!") | (None, "DC1!") => Coding::Integer.accepts(coding),
        (Some(7), "MM3!") => coding == 0,
        (Some(0), "MM6!") => coding == 0 || Coding::Integer.accepts(coding),
        (None, "CT1!") => Coding::IntegerOrVariable.accepts(coding),
        _ => return finding,
    };
    if valid {
        match (action, tag.0) {
            (Some(0), "MM6!") => {
                selection.address.get_or_insert(number);
            }
            (Some(0x0c | 0x0d), _) => {
                selection.first_edit.get_or_insert(number);
            }
            _ => {}
        }
    }
    if action.is_none() && tag.0 == "DC1!" {
        crate::readings::check_disconnector_subunit(target, record.subunit, number, findings);
        if valid {
            let data = record.data.as_ref();
            crate::readings::manual_release(data, number, findings);
            let allowed = data
                .split_first()
                .is_some_and(|(value, rest)| *value <= 2 && rest.iter().all(|byte| *byte == 0));
            findings.push(RuleFinding {
                rule: "Disconnector command value",
                reference: "Annex M, Table M.8",
                source_url: annex_m!(20, 102),
                status: if allowed {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail: "A disconnector command accepts values 0, 1 and 2.",
                record_number: Some(number),
            });
        }
    }
    finding.source_url = match (action, tag.0) {
        (Some(0x0c | 0x0d), _) | (_, "CT1!" | "DC1!") => annex_b!(39),
        (_, "DT2!D" | "DT2!N" | "EJ1!") | (Some(0 | 1 | 15), "EW1!") => annex_b!(40),
        (_, "VM1!" | "MM3!" | "MM6!") => annex_b!(42),
        _ => annex_b!(41),
    };
    finding.status = if valid {
        RuleStatus::Passed
    } else {
        RuleStatus::Failed
    };
    finding.detail = if valid {
        "The command's DIF encoding is listed for its action and data point."
    } else {
        "The DIF encoding is outside the list for this command."
    };
    finding
}
