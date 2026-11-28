use super::{Reading, descriptor_count};
use crate::telegram::{RuleFinding, RuleStatus};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

#[derive(Default)]
pub(super) struct StorageRanges {
    dates: BTreeMap<(u64, u64), (u8, u16)>,
    conflicting_dates: bool,
    series: BTreeMap<(&'static str, u64, u64), BTreeSet<u64>>,
    markers: Vec<Marker>,
}

struct Marker {
    kind: u8,
    storage: u64,
    count: Option<u64>,
    record: usize,
}

impl StorageRanges {
    pub(super) fn observe_date<H: AsRef<[u8]>>(
        &mut self,
        reading: &Reading<H>,
        vib: &[u8],
        dif: u8,
        recent: bool,
    ) -> bool {
        if recent || !(8..=199).contains(&reading.storage) || reading.tariff != 0 || dif & 0x30 != 0
        {
            return false;
        }
        let (day, month) = match (vib, dif & 15, reading.data.as_ref()) {
            ([0x6c], 2, [day, month]) => (day, month),
            ([0x6d], 4, [minutes, _, day, month]) if minutes & 0x80 == 0 => (day, month),
            ([0x6d], 6, [_, minutes, _, day, month, _]) if minutes & 0x80 == 0 => (day, month),
            _ => return false,
        };
        let year = day >> 5 | (month >> 4) << 3;
        let (day, month) = (day & 31, month & 15);
        let last_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year % 4 == 0 => 29,
            2 => 28,
            _ => 0,
        };
        if year > 99 || !(1..=last_day).contains(&day) {
            return false;
        }
        let date = (year, u16::from(month) * 32 + u16::from(day));
        self.conflicting_dates |= self
            .dates
            .insert((reading.subunit, reading.storage), date)
            .is_some_and(|old| old != date);
        true
    }

    pub(super) fn observe<H: AsRef<[u8]>>(
        &mut self,
        tag: &'static str,
        reading: &Reading<H>,
        vib: &[u8],
        n: usize,
    ) {
        if tag == "YD3!P" {
            if let [0xfd, kind @ 0x20..=0x22] = vib
                && reading.subunit == 0
            {
                let coding = reading.header.as_ref()[0] & 15;
                self.markers.push(Marker {
                    kind: *kind,
                    storage: reading.storage,
                    count: descriptor_count(coding, reading.data.as_ref()),
                    record: n,
                });
            }
        } else if tag.ends_with("!P") && !tag.starts_with("DT") && !tag.starts_with("YD") {
            self.series
                .entry((tag, reading.tariff, reading.subunit))
                .or_default()
                .insert(reading.storage);
        }
    }

    pub(super) fn finish(&self, complete: bool, results: &mut Vec<RuleFinding>) {
        if !complete || self.conflicting_dates {
            return;
        }
        let mut independent = BTreeSet::new();
        let mut shared_marker_types = 7u8;
        let mut distinct_sets = 0;
        for (&(_, _, subunit), storages) in &self.series {
            if storages.len() < 2 || !storages.is_disjoint(&independent) {
                continue;
            }
            let timestamps: Option<BTreeSet<_>> = storages
                .iter()
                .map(|storage| {
                    self.dates
                        .get(&(subunit, *storage))
                        .map(|date| (*date, *storage))
                })
                .collect();
            let Some(times) = timestamps else {
                continue;
            };
            let (Some(&(first_date, oldest)), Some(&(last_date, newest))) =
                (times.first(), times.last())
            else {
                continue;
            };
            if first_date.0 != last_date.0
                || times
                    .iter()
                    .map(|(date, _)| date)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != times.len()
            {
                continue;
            }
            independent.extend(storages);
            distinct_sets += 1;
            let mut types = 0u8;
            let markers = || {
                self.markers
                    .iter()
                    .filter(|m| storages.contains(&m.storage))
            };
            for marker in markers() {
                types |= 1 << (marker.kind - 0x20);
            }
            shared_marker_types &= types;
            let size_declared = markers().filter(|m| m.kind == 0x22).count() == 1
                && markers().any(|m| m.kind == 0x22 && m.count == Some(storages.len() as u64));
            if size_declared {
                for marker in markers() {
                    let valid = marker.storage == if marker.kind == 0x21 { newest } else { oldest };
                    results.push(RuleFinding {rule: "Storage range date endpoint", reference: "Annex K, K.2.2", source_url: primary!(124, 790),
                        status: if valid {RuleStatus::Passed} else {RuleStatus::Failed}, detail: "For this complete dated storage block, the first or size descriptor must mark the oldest date and the last descriptor must mark the newest date.", record_number: Some(marker.record)});
                }
                if storages.first() != Some(&oldest) {
                    let first = markers().any(|m| m.kind == 0x20 && m.storage == oldest);
                    results.push(RuleFinding {rule: "Cyclic storage range start", reference: "Annex K, K.2.2", source_url: primary!(125, 89),
                        status: if first {RuleStatus::Passed} else {RuleStatus::Failed}, detail: "When the oldest dated value is outside the lowest storage position, FD20 must declare its position.", record_number: None});
                }
            }
        }
        if distinct_sets > 1 {
            results.push(RuleFinding {rule: "Historical storage range descriptors", reference: "Annex K, K.2.2", source_url: primary!(125, 89),
                status: if shared_marker_types != 0 {RuleStatus::Passed} else {RuleStatus::Failed}, detail: "Independently dated historical storage sets in this message must all include the same type of range descriptor.", record_number: None});
        }
    }
}
