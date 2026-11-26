use crate::telegram::{RuleFinding, RuleStatus};

pub fn current_din_address(
    data: &[u8],
    manufacturer: &str,
    serial: &str,
    device: u8,
    number: usize,
) -> Option<RuleFinding> {
    let [14, serial_wire @ .., _, _, c, b, a, category] = data else {
        return None;
    };
    if serial_wire.len() != 8 {
        return None;
    }
    let expected_category = match device {
        0x02 => b'1',
        0x03 => b'7',
        0x04 | 0x0c | 0x0d => b'6',
        0x06 | 0x15 | 0x17 => b'9',
        0x07 | 0x16 => b'8',
        0x08 => b'4',
        0x0a | 0x0b => b'5',
        0x0e | 0x25 | 0x31..=0x33 | 0x35..=0x38 => b'E',
        0 | 1 | 5 | 9 | 0x0f | 0x14 | 0x18..=0x1b | 0x1d..=0x21 | 0x28..=0x2a => b'F',
        _ => return None,
    };
    let id_matches = serial.bytes().eq(serial_wire.iter().rev().copied());
    let identity =
        manufacturer.as_bytes() == [*a, *b, *c] && id_matches && *category == expected_category;
    let (status, detail) = if identity {
        (
            RuleStatus::Passed,
            "The current DIN manufacturer, fabrication number and OBIS category agree with the end-device identity.",
        )
    } else {
        (
            RuleStatus::Failed,
            "The current DIN manufacturer, fabrication number or OBIS category disagrees with the end-device identity.",
        )
    };
    Some(RuleFinding {
        rule: "Current DIN address conversion",
        reference: "§3.2, Table 6 / §3.3",
        source_url: primary!(36, 580),
        status,
        detail,
        record_number: Some(number),
    })
}
