use super::*;

fn profile_label(label: &ValueLabel) -> bool {
    matches!(
        label,
        ValueLabel::CompactProfile
            | ValueLabel::CompactProfileWithRegisterNumbers
            | ValueLabel::InverseCompactProfile
    )
}

fn same_register(a: &user_data::DataRecord<'_>, b: &user_data::DataRecord<'_>) -> bool {
    let (Some(ai), Some(bi), Some(av), Some(bv)) = (
        a.data_information(),
        b.data_information(),
        a.value_information(),
        b.value_information(),
    ) else {
        return false;
    };
    ai.device == bi.device
        && ai.tariff == bi.tariff
        && ai.storage_number == bi.storage_number
        && ai.function_field == bi.function_field
        && av.decimal_scale_exponent == bv.decimal_scale_exponent
        && record_offset_power10(a) == record_offset_power10(b)
        && av.units().eq(bv.units())
        && av.labels().filter(|v| !profile_label(v)).eq(bv.labels())
}

fn integer(bytes: &[u8], signed: bool) -> i128 {
    let fill = if signed && bytes.last().is_some_and(|b| b & 0x80 != 0) {
        -1
    } else {
        0
    };
    bytes
        .iter()
        .rev()
        .fold(fill, |value, byte| (value << 8) | i128::from(*byte))
}

fn element(bytes: &[u8], bcd: bool, signed: bool) -> Option<i128> {
    if !bcd {
        let value = integer(bytes, signed);
        let invalid = if signed {
            -(1i128 << (bytes.len() * 8 - 1))
        } else {
            (1i128 << (bytes.len() * 8)) - 1
        };
        return (value != invalid).then_some(value);
    }
    let mut nibbles = bytes
        .iter()
        .rev()
        .flat_map(|byte| [byte >> 4, byte & 15])
        .peekable();
    let negative = signed && nibbles.peek() == Some(&15);
    if negative {
        nibbles.next();
    }
    let magnitude = nibbles.try_fold(0i128, |n, digit| {
        (digit < 10).then_some(n * 10 + i128::from(digit))
    })?;
    Some(if negative { -magnitude } else { magnitude })
}

#[derive(Clone, Copy)]
enum ProfileNumber {
    Integer(i128),
    Real(f64),
}

impl ProfileNumber {
    fn real(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self::Real(value))
    }

    fn as_real(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Real(value) => value,
        }
    }

    fn apply(self, difference: Self, subtract: bool) -> Option<Self> {
        if let (Self::Integer(base), Self::Integer(delta)) = (self, difference) {
            return if subtract {
                base.checked_sub(delta)
            } else {
                base.checked_add(delta)
            }
            .map(Self::Integer);
        }
        let delta = difference.as_real();
        Self::real(self.as_real() + if subtract { -delta } else { delta })
    }

    fn decimal(self, scale: isize, offset: Option<isize>) -> String {
        let significand = match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
        };
        if let Some((whole, fraction)) = significand.split_once('.') {
            return scaled_decimal(
                &format!("{whole}{fraction}"),
                scale - fraction.len() as isize,
                offset,
            );
        }
        scaled_decimal(&significand, scale, offset)
    }
}

fn base_number(record: &user_data::DataRecord<'_>) -> Option<ProfileNumber> {
    let value = match record.value()? {
        DataType::Number(number) | DataType::LossyNumber(number) => *number,
        _ => return None,
    };
    let bytes = record
        .raw_bytes()
        .get(record.data_record_header.get_size()..)?;
    match record.data_information()?.data_field_coding {
        DataFieldCoding::Real32Bit => return ProfileNumber::real(value),
        DataFieldCoding::Integer8Bit
        | DataFieldCoding::Integer16Bit
        | DataFieldCoding::Integer24Bit
        | DataFieldCoding::Integer32Bit
        | DataFieldCoding::Integer48Bit
        | DataFieldCoding::Integer64Bit => {
            return element(bytes, false, true).map(ProfileNumber::Integer)
        }
        DataFieldCoding::VariableLength if matches!(bytes.first(), Some(0xe0..=0xf6)) => {
            let (last, lower) = bytes.get(1..)?.split_last()?;
            if *last == 0x80 && lower.iter().all(|byte| *byte == 0) {
                return None;
            }
        }
        _ => {}
    }
    exact_significand(record, value)?
        .parse()
        .ok()
        .map(ProfileNumber::Integer)
}

fn spacing(unit: u8, value: u8) -> Option<(u8, &'static str)> {
    match (unit, value) {
        (_, 0) => Some((unit + 1, "column")),
        (0, 1..=250) => Some((value, "second")),
        (1, 1..=250) => Some((value, "minute")),
        (2, 1..=250) => Some((value, "hour")),
        (3, 1..=250) => Some((value, "day")),
        (3, 253) => Some((1, "half_month")),
        (1, 254) => Some((6, "month")),
        (2, 254) => Some((3, "month")),
        (3, 254) => Some((1, "month")),
        _ => None,
    }
}

fn element_layout(
    control: u8,
    interval: u8,
    data_len: usize,
) -> Result<(usize, bool, u8, &'static str), &'static str> {
    let coding = control & 15;
    let (width, bcd) = match coding {
        n @ 1..=4 => (usize::from(n), false),
        5 => (4, false),
        6 => (6, false),
        7 => (8, false),
        n @ 9..=12 => (usize::from(n - 8), true),
        14 => (6, true),
        _ => return Err("unsupported compact profile element coding"),
    };
    if !data_len.is_multiple_of(width) {
        return Err("truncated compact profile element");
    }
    let (interval, unit) =
        spacing((control >> 4) & 3, interval).ok_or("invalid compact profile spacing")?;
    Ok((width, bcd, interval, unit))
}

pub(super) fn decode<'a>(
    record: &user_data::DataRecord<'a>,
    bases: &[&[u8]],
    history: &mut super::profiles::ProfileHistory<'a>,
) -> Result<Option<ValueOutput>, &'static str> {
    let history_value = history.observe(record);
    if matches!(record.value(), Some(DataType::InverseCompactProfile(profile))
        if profile.monthly_bcd_differences().is_some())
    {
        return Ok(history_value);
    }
    let Some(info) = record.value_information() else {
        return Ok(None);
    };
    if !info.labels().any(|l| profile_label(&l))
        || !record
            .data_information()
            .is_some_and(|info| info.data_field_coding == DataFieldCoding::VariableLength)
    {
        return Ok(None);
    }
    let Some([length, control, interval, data @ ..]) = record
        .raw_bytes()
        .get(record.data_record_header.get_size()..)
    else {
        return history_value
            .map(Some)
            .ok_or("missing compact profile fields");
    };
    if usize::from(*length) != 2 + data.len() || *length > 0xbf {
        return history_value
            .map(Some)
            .ok_or("invalid compact profile length");
    }
    let coding = control & 15;
    let (width, bcd, interval, unit) = match element_layout(*control, *interval, data.len()) {
        Ok(layout) => layout,
        Err(_) if history_value.is_some() => return Ok(history_value),
        Err(reason) => return Err(reason),
    };
    let inverse = info.has_label(ValueLabel::InverseCompactProfile);
    let mode = control >> 6;
    let mut current = None;
    if mode != 0 && !data.is_empty() {
        let base = bases
            .iter()
            .rev()
            .filter_map(|bytes| user_data::DataRecord::try_from(*bytes).ok())
            .find(|base| same_register(record, base))
            .ok_or("compact profile requires a matching base reading")?;
        current = Some(base_number(&base).ok_or("invalid compact profile base reading")?);
    }
    let mut samples = Vec::new();
    for bytes in data.chunks_exact(width) {
        let number = if coding == 5 {
            let value = f32::from_le_bytes(bytes.try_into().map_err(|_| "invalid REAL32 length")?);
            if matches!(mode, 1 | 2) && value < 0.0 {
                None
            } else {
                ProfileNumber::real(f64::from(value))
            }
        } else {
            element(bytes, bcd, matches!(mode, 0 | 3)).map(ProfileNumber::Integer)
        };
        current = if mode == 0 {
            number
        } else {
            current
                .zip(number)
                .and_then(|(base, delta)| base.apply(delta, inverse ^ (mode == 2)))
        };
        samples.push(
            current.map(|n| n.decimal(info.decimal_scale_exponent, record_offset_power10(record))),
        );
    }
    Ok(Some(ValueOutput {
        kind: "compact_profile".into(),
        value: Some(serde_json::json!({
            "direction": if inverse {"backward"} else {"forward"},
            "spacing": interval, "spacing_unit": unit, "values": samples
        })),
    }))
}

pub(super) fn time_point(record: &user_data::DataRecord<'_>) -> Option<ValueOutput> {
    let vib = record
        .data_record_header
        .raw_data_record_header
        .value_information_block
        .as_ref()?;
    if vib.value_information.data & 0x7f != 0x6d
        || record.data_information()?.data_field_coding != DataFieldCoding::VariableLength
    {
        return None;
    }
    let (length, data) = record
        .raw_bytes()
        .get(record.data_record_header.get_size()..)?
        .split_first()?;
    if !(0xe2..=0xea).contains(length) {
        return None;
    }
    let (flags, value) = data.split_last()?;
    let ticks = integer(value, true);
    let zone = (flags << 3) as i8 >> 3;
    let resolution = match (flags >> 5) & 3 {
        0 => (2, 1),
        1 => (1, 1),
        2 => (1, 256),
        _ => (1, 32768),
    };
    let relative = zone == -16;
    let valid =
        ticks != -(1i128 << (value.len() * 8 - 1)) && (relative || (-12..=15).contains(&zone));
    Some(ValueOutput { kind: if relative { "time_offset" } else { "timestamp" }.into(), value: valid.then(|| serde_json::json!({
        "ticks": ticks.to_string(), "seconds_per_tick_numerator": resolution.0, "seconds_per_tick_denominator": resolution.1,
        "reference": if relative { "transmission" } else if flags & 0x80 != 0 { "1970-01-01T00:00:00Z" } else { "2013-01-01T00:00:00Z" },
        "utc_offset_hours": if (-12..=14).contains(&zone) {Some(zone)} else {None}
    })) })
}
