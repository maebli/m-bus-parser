use crate::user_data::{Counter, StatusField};

pub(crate) struct Reading {
    pub value: i32,
    pub unit: &'static str,
    pub stored: bool,
    pub unit_byte: u8,
}

pub(crate) fn readings(types: u16, status: u8, first: &Counter, second: &Counter) -> [Reading; 2] {
    let [a, b] = types.to_be_bytes();
    [(a, b, first), (b, a, second)].map(|(code, other, counter)| {
        let historic = code & 0x3f == 0x3e;
        Reading {
            value: counter.value(),
            unit: unit(if historic { other } else { code }),
            stored: historic || status & StatusField::COUNTER_FIXED_DATE.bits() != 0,
            unit_byte: code,
        }
    })
}

pub(crate) fn medium(cnt1_type: u8, cnt2_type: u8) -> &'static str {
    match ((cnt1_type & 0xC0) >> 6) | ((cnt2_type & 0xC0) >> 4) {
        0x00 => "Other",
        0x01 => "Oil",
        0x02 => "Electricity",
        0x03 => "Gas",
        0x04 => "Heat",
        0x05 => "Steam",
        0x06 => "Hot Water",
        0x07 => "Water",
        0x08 => "H.C.A.",
        0x09 | 0x0F => "Reserved",
        0x0A => "Gas Mode 2",
        0x0B => "Heat Mode 2",
        0x0C => "Hot Water Mode 2",
        0x0D => "Water Mode 2",
        0x0E => "H.C.A. Mode 2",
        _ => "unknown",
    }
}

fn unit(medium_unit_byte: u8) -> &'static str {
    match medium_unit_byte & 0x3F {
        0x00 => "h,m,s",
        0x01 => "D,M,Y",
        0x02 => "Wh",
        0x03 => "10 Wh",
        0x04 => "100 Wh",
        0x05 => "kWh",
        0x06 => "10 kWh",
        0x07 => "100 kWh",
        0x08 => "MWh",
        0x09 => "10 MWh",
        0x0A => "100 MWh",
        0x0B => "kJ",
        0x0C => "10 kJ",
        0x0D => "100 kJ",
        0x0E => "MJ",
        0x0F => "10 MJ",
        0x10 => "100 MJ",
        0x11 => "GJ",
        0x12 => "10 GJ",
        0x13 => "100 GJ",
        0x14 => "W",
        0x15 => "10 W",
        0x16 => "100 W",
        0x17 => "kW",
        0x18 => "10 kW",
        0x19 => "100 kW",
        0x1A => "MW",
        0x1B => "10 MW",
        0x1C => "100 MW",
        0x1D => "kJ/h",
        0x1E => "10 kJ/h",
        0x1F => "100 kJ/h",
        0x20 => "MJ/h",
        0x21 => "10 MJ/h",
        0x22 => "100 MJ/h",
        0x23 => "GJ/h",
        0x24 => "10 GJ/h",
        0x25 => "100 GJ/h",
        0x26 => "ml",
        0x27 => "10 ml",
        0x28 => "100 ml",
        0x29 => "l",
        0x2A => "10 l",
        0x2B => "100 l",
        0x2C => "m^3",
        0x2D => "10 m^3",
        0x2E => "100 m^3",
        0x2F => "ml/h",
        0x30 => "10 ml/h",
        0x31 => "100 ml/h",
        0x32 => "l/h",
        0x33 => "10 l/h",
        0x34 => "100 l/h",
        0x35 => "m^3/h",
        0x36 => "10 m^3/h",
        0x37 => "100 m^3/h",
        0x38 => "1e-3 °C",
        0x39 => "units for HCA",
        0x3A..=0x3D => "reserved",
        0x3E => "reserved but historic",
        0x3F => "without units",
        _ => "unknown",
    }
}
