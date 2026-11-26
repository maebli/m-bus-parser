use super::{Coding, Reading, Taf7Parameters};
use crate::telegram::{RuleFinding, RuleStatus};
use alloc::{collections::BTreeSet, vec::Vec};

#[derive(Ord, PartialOrd, PartialEq, Eq)]
struct Register {
    vib: Vec<u8>,
    storage: u64,
    tariff: u64,
    subunit: u64,
    function: u8,
}

#[derive(Default)]
pub(in crate::readings) struct Profiles {
    inverse: bool,
    base_time: bool,
    actuality: bool,
    bases: BTreeSet<Register>,
    differential: Vec<(usize, Register)>,
}

fn orthogonal_profile(vib: &[u8]) -> Option<(usize, u8)> {
    let mut i = match *vib.first()? {
        0xfb | 0xfd if matches!(vib.get(1), Some(0xfd)) => 3,
        0xfb | 0xfd => 2,
        0xfc => 2 + usize::from(*vib.get(1)?),
        0xff => return None,
        first if first & 0x80 != 0 => 1,
        _ => return None,
    };
    while i < vib.len() {
        let byte = vib[i];
        match byte & 0x7f {
            kind @ (0x13 | 0x1e | 0x1f) => return Some((i, kind)),
            0x7d | 0x7f => i += 2,
            _ => i += 1,
        }
    }
    None
}

fn outcome(valid: Option<bool>) -> RuleStatus {
    match valid {
        Some(true) => RuleStatus::Passed,
        Some(false) => RuleStatus::Failed,
        None => RuleStatus::CouldNotCheck,
    }
}

fn check(
    rule: &'static str,
    reference: &'static str,
    source_url: &'static str,
    valid: Option<bool>,
    detail: &'static str,
    record_number: Option<usize>,
) -> RuleFinding {
    RuleFinding {
        rule,
        reference,
        source_url,
        status: outcome(valid),
        detail,
        record_number,
    }
}

impl Profiles {
    pub(in crate::readings) fn observe<H: AsRef<[u8]>>(
        &mut self,
        reading: &Reading<H>,
        dib_end: usize,
        taf7: Option<Taf7Parameters>,
        n: usize,
        results: &mut Vec<RuleFinding>,
    ) -> bool {
        let header = reading.header.as_ref();
        let dif = header[0];
        let vib = &header[dib_end + 1..];
        let recent = dib_end > 0 && header[dib_end] == 0;
        let coding = dif & 15;
        let register = |vib| Register {
            vib,
            storage: reading.storage,
            tariff: reading.tariff,
            subunit: reading.subunit,
            function: (dif >> 4) & 3,
        };
        let Some((index, kind)) = orthogonal_profile(vib) else {
            if taf7.is_some() {
                if !recent && reading.storage == 3 && dif & 0x30 == 0 && reading.tariff == 0 {
                    self.base_time |= matches!(vib, [0x6d])
                        && (Coding::Timestamp.accepts(coding) || Coding::Time.accepts(coding));
                    self.actuality |=
                        matches!(vib, [0x74..=0x77]) && Coding::Number.accepts(coding);
                }
                if Coding::Number.accepts(coding) || coding == 5 {
                    self.bases.insert(register(vib.to_vec()));
                }
            }
            return false;
        };
        let terminal = index + 1 == vib.len();
        results.push(check(
            "Compact profile modifier position",
            "Annex G, G.2",
            primary!(105, 499),
            Some(terminal),
            "The compact profile modifier must be the final VIFE.",
            Some(n),
        ));
        if kind == 0x1e {
            results.push(check(
                "Compact register profile final DIFE",
                "Annex G, G.1",
                primary!(105, 312),
                Some(recent),
                "A compact profile for historical registers uses a final zero DIFE.",
                Some(n),
            ));
        }
        let data = reading.data.as_ref();
        let layout = if let [length, control, distance, samples @ ..] = data {
            let width = match control & 15 {
                width @ 1..=4 => Some(usize::from(width)),
                5 | 12 => Some(4),
                6 | 14 => Some(6),
                7 => Some(8),
                width @ 9..=11 => Some(usize::from(width - 8)),
                _ => None,
            };
            width
                .filter(|width| {
                    coding == 13
                        && *length <= 0xbf
                        && usize::from(*length) == 2 + samples.len()
                        && samples.len().is_multiple_of(*width)
                })
                .map(|width| (*control, *distance, samples.len() / width))
        } else {
            None
        };
        results.push(check(
            "Compact profile element length",
            "Annex G, G.2 / EN 13757-3, Annex F",
            primary!(105, 499),
            Some(layout.is_some()),
            "The length must cover two control bytes and a whole number of encoded elements.",
            Some(n),
        ));
        let Some(parameters) = taf7 else {
            return true;
        };
        self.inverse |= kind == 0x13;
        results.push(check(
            "TAF7 profile direction",
            "Annex R, R.2.2",
            annex_r!(9, 591),
            Some(kind == 0x13),
            "TAF7 uses an inverse compact profile.",
            Some(n),
        ));
        results.push(check(
            "TAF7 profile storage number",
            "Annex R, R.3.2",
            annex_r!(13, 80),
            Some(reading.storage == 3),
            "The TAF7 compact profile uses storage number 3.",
            Some(n),
        ));
        if let Some((control, distance, count)) = layout {
            if control >> 6 != 0 && terminal {
                let mut base_vib = vib[..index].to_vec();
                if let Some(last) = base_vib.last_mut() {
                    *last &= 0x7f;
                }
                self.differential.push((n, register(base_vib)));
            }
            let interval = match distance {
                1..=250 => Some(
                    u64::from(distance)
                        * [1_000, 60_000, 3_600_000, 86_400_000][usize::from(control >> 4 & 3)],
                ),
                _ => None,
            };
            results.push(check("TAF7 measurement count", "Annex R, R.2.3 equation 6", annex_r!(11, 430),
                interval.zip(parameters.nominal_transmission_interval_ms).map(|(spacing, nominal)| {
                    nominal != 0 && (count as u128) * u128::from(spacing) == 3 * u128::from(nominal) + 2 * u128::from(spacing)
                }), "The encoded number of profile values must equal three nominal transmission intervals divided by the profile interval, plus two.", Some(n)));
            results.push(check(
                "TAF7 profile interval and acceptance window",
                "Annex R, R.2.3 equation 2",
                annex_r!(10, 555),
                interval
                    .zip(parameters.acceptance_window_ms)
                    .map(|(i, window)| i <= window),
                "The fixed profile interval must fit within the declared acceptance window.",
                Some(n),
            ));
            results.push(check("TAF7 profile interval and measuring period", "Annex R, R.2.3 equation 2", annex_r!(10, 555), interval.zip(parameters.measuring_period_ms).map(|(i, period)| period != 0 && i.is_multiple_of(period)), "The fixed profile interval must be an integer multiple of the declared measuring period.", Some(n)));
        }
        true
    }

    pub(in crate::readings) fn finish(
        &self,
        parameters: Taf7Parameters,
        complete: bool,
        results: &mut Vec<RuleFinding>,
    ) {
        let present = |found| {
            if found {
                Some(true)
            } else {
                complete.then_some(false)
            }
        };
        for (rule, found, url, detail) in [
            (
                "TAF7 inverse compact profile",
                self.inverse,
                annex_r!(9, 591),
                "The declared TAF7 data message must contain an inverse compact profile.",
            ),
            (
                "TAF7 base time",
                self.base_time,
                annex_r!(12, 825),
                "DT1!B or DT5!B is required at storage number 3. Its encoded time may be invalid.",
            ),
            (
                "TAF7 actuality duration",
                self.actuality,
                annex_r!(12, 942),
                "DP1!B must carry the elapsed time between measurement and transmission at storage number 3.",
            ),
        ] {
            results.push(check(
                rule,
                "Annex R, R.2.2 / R.3.2",
                url,
                present(found),
                detail,
                None,
            ));
        }
        for (n, register) in &self.differential {
            results.push(check("TAF7 differential base value", "Annex R, R.3.2", annex_r!(12, 725), present(self.bases.contains(register)), "A differential profile needs a base reading for the same register, tariff, subunit and storage number.", Some(*n)));
        }
        results.push(check(
            "TAF7 measuring period and acceptance window",
            "Annex R, R.2.3 equation 1",
            annex_r!(10, 471),
            parameters
                .measuring_period_ms
                .zip(parameters.acceptance_window_ms)
                .map(|(p, w)| p != 0 && p <= w),
            "The declared measuring period must fit inside the acceptance window.",
            None,
        ));
        results.push(check(
            "TAF7 acceptance window and registration period",
            "Annex R, R.2.3 equation 3",
            annex_r!(10, 622),
            parameters
                .acceptance_window_ms
                .zip(parameters.registration_period_ms)
                .map(|(w, r)| w != 0 && u128::from(w) * 100 == u128::from(r)),
            "The declared acceptance window is one percent of the registration period.",
            None,
        ));
    }
}
