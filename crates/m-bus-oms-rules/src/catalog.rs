use super::readings::Coding::*;
use super::readings::Tag;

pub(super) fn identify(
    vib: &[u8],
    tariff: u64,
    function: u8,
    storage: u64,
    recent: bool,
    coding: u8,
) -> Option<Tag> {
    match vib {
        [0x7c, 0x03, 0x49, 0x55, 0x23] | [0xfd, 0xfd, 0x2c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 4, false) => Some(Tag("AD4!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x54, 0x46, 0x23] | [0xfd, 0xfd, 0x2d] => {
            match (tariff, function, storage, recent) {
                (0, 0, 5, false) => Some(Tag("AD5!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x4c, 0x41, 0x23] | [0xfd, 0xfd, 0x2e] => {
            match (tariff, function, storage, recent) {
                (0, 0, 6, false) => Some(Tag("AD6!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x4d, 0x41, 0x23] | [0xfd, 0xfd, 0x2f] => {
            match (tariff, function, storage, recent) {
                (0, 0, 7, false) => Some(Tag("AD7!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x44, 0x4f, 0x23] | [0xfd, 0xfd, 0x30] => {
            match (tariff, function, storage, recent) {
                (0, 0, 8, false) => Some(Tag("AD8!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x42, 0x53, 0x23] | [0xfd, 0xfd, 0x31] => {
            match (tariff, function, storage, recent) {
                (0, 0, 9, false) => Some(Tag("AD9!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x04, 0x44, 0x43, 0x53, 0x23] | [0xfd, 0xfd, 0x32] => {
            match (tariff, function, storage, recent) {
                (0, 0, 10, false) => Some(Tag("AD10!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x46, 0x53, 0x23] | [0xfd, 0xfd, 0x33] => {
            match (tariff, function, storage, recent) {
                (0, 0, 11, false) => Some(Tag("AD11!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x44, 0x53, 0x23] | [0xfd, 0xfd, 0x34] => {
            match (tariff, function, storage, recent) {
                (0, 0, 12, false) => Some(Tag("AD12!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x50, 0x43, 0x25] | [0xfd, 0xfd, 0x39] => {
            match (tariff, function, storage, recent) {
                (0, 0, 13, false) => Some(Tag("AD13!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x02, 0x6d, 0x6d] | [0xfd, 0xfd, 0x3a..=0x3b] => {
            match (tariff, function, storage, recent) {
                (0, 0, 14, false) => Some(Tag("AD14!", Number)),
                (0, 0, 0, false) => Some(Tag("RF1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x02, 0x6d, 0x63] => match (tariff, function, storage, recent) {
            (0, 0, 14, false) => Some(Tag("AD14!", Number)),
            _ => None,
        },
        [0xfd, 0xd0..=0xdf, 0xfc, 0x01] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CA1!", Number)),
            _ => None,
        },
        [0xfd, 0xd0..=0xdf, 0xfc, 0x02] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CA2!", Number)),
            _ => None,
        },
        [0xfd, 0xd0..=0xdf, 0xfc, 0x03] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CA3!", Number)),
            _ => None,
        },
        [0xfd, 0xd0..=0xdf, 0xfc, 0x04] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CA4!", Number)),
            _ => None,
        },
        [0x7c, 0x03, 0x31, 0x4f, 0x43] | [0xfd, 0xfd, 0x12..=0x13] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("CC1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x32, 0x4f, 0x43] | [0xfd, 0xfd, 0x10..=0x11] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("CC2!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x31, 0x44, 0x43] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CD1!", Number)),
            _ => None,
        },
        [0xfd, 0x9f, 0x1d] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("CT1!", IntegerOrVariable)),
            _ => None,
        },
        [0xfd, 0x1f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DC1!", Integer)),
            _ => None,
        },
        [0xfd, 0x1a] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DC2!", Integer)),
            _ => None,
        },
        [0x74..=0x77] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DP1!", Number)),
            (0, 0, 3, false) => Some(Tag("DP1!B", Number)),
            _ => None,
        },
        [0x70..=0x73] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DP2!", Number)),
            _ => None,
        },
        [0xfd, 0x3c..=0x3d] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DP3!", Number)),
            _ => None,
        },
        [0x24..=0x27] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DP5!", Number)),
            _ => None,
        },
        [0x20..=0x23] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("DP6!", Number)),
            _ => None,
        },
        [0x6d] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) if Timestamp.accepts(coding) => Some(Tag("DT1!", Timestamp)),
            (0, 0, 0, false) if Time.accepts(coding) => Some(Tag("DT5!", Time)),
            (0, 0, 0, false) => Some(Tag("DT1! / DT5!", Unmatched)),
            (0, 0, 3, false) if Timestamp.accepts(coding) => Some(Tag("DT1!B / DT1!Xn", Timestamp)),
            (0, 0, 3, false) if Time.accepts(coding) => Some(Tag("DT5!B", Time)),
            (0, 0, 3, false) => Some(Tag("DT1!B / DT5!B", Unmatched)),
            (0, 0, 0..=99 | 101..=124, true) => Some(Tag("DT1!R", Timestamp)),
            (0, 0, 8..=31, false) => Some(Tag("DT1!P / DT1!Xn", Timestamp)),
            (0, 0, 32..=199, false) => Some(Tag("DT1!P", Timestamp)),
            (0, 1, 0, false) => Some(Tag("DT1!A", Timestamp)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("DT1!AR", Timestamp)),
            (1..=15, 1, 0, false) => Some(Tag("DT1!AT", Timestamp)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("DT1!ART", Timestamp)),
            (0, 0, 3..=31, false) => Some(Tag("DT1!Xn", Timestamp)),
            _ => None,
        },
        [0x6c] => match (tariff, function, storage, recent) {
            (0, 0, 0..=99 | 101..=124, true) => Some(Tag("DT2!R", Date)),
            (0, 0, 0, false) => Some(Tag("DT2!", Date)),
            (0, 0, 1, false) => Some(Tag("DT2!D", Date)),
            (0, 1, 0, false) => Some(Tag("DT2!A", Date)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("DT2!AR", Date)),
            (1..=15, 1, 0, false) => Some(Tag("DT2!AT", Date)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("DT2!ART", Date)),
            (0, 0, 3..=31, false) => Some(Tag("DT2!Xn", Date)),
            _ => None,
        },
        [0xec, 0x7e] => match (tariff, function, storage, recent) {
            (0, 0, 1, false) => Some(Tag("DT2!N", Date)),
            _ => None,
        },
        [0xed, 0x3c] => match (tariff, function, storage, recent) {
            (0, 1, 0, false) => Some(Tag("DT3!A", Timestamp)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("DT3!AR", Timestamp)),
            (1..=15, 1, 0, false) => Some(Tag("DT3!AT", Timestamp)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("DT3!ART", Timestamp)),
            _ => None,
        },
        [0xec, 0x3c] => match (tariff, function, storage, recent) {
            (0, 1, 0, false) => Some(Tag("DT4!A", Date)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("DT4!AR", Date)),
            (1..=15, 1, 0, false) => Some(Tag("DT4!AT", Date)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("DT4!ART", Date)),
            _ => None,
        },
        [0xfb, 0x0c..=0x0f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("EC1!", Number)),
            (0, 0, 1, false) => Some(Tag("EC1!D", Number)),
            (0, 0, 8..=199, false) => Some(Tag("EC1!P", Number)),
            _ => None,
        },
        [0xfb, 0x8c..=0x8f, 0x3c] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("EC2!", Number)),
            (0, 0, 1, false) => Some(Tag("EC2!D", Number)),
            (0, 0, 8..=199, false) => Some(Tag("EC2!P", Number)),
            _ => None,
        },
        [0x08..=0x0f] | [0xfb, 0x08..=0x09] | [0xfb, 0x88..=0x89, 0x7d] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("EJ1!", Number)),
                (0, 0, 3, false) => Some(Tag("EJ1!B", Number)),
                (1, 0, 0, false) => Some(Tag("EJ1!T", Number)),
                (0, 0, 8..=199, false) => Some(Tag("EJ1!P", Number)),
                (0, 0, 1, false) => Some(Tag("EJ1!D", Number)),
                (1, 0, 1, false) => Some(Tag("EJ1!DT", Number)),
                _ => None,
            }
        }
        [0x88..=0x8f, 0x3c] | [0xfb, 0x88..=0x89, 0x3c] | [0xfb, 0x88..=0x89, 0xfd, 0x3c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("EJ2!", Number)),
                (0, 0, 3, false) => Some(Tag("EJ2!B", Number)),
                (0, 0, 1, false) => Some(Tag("EJ2!D", Number)),
                (0, 0, 8..=199, false) => Some(Tag("EJ2!P", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x6d, 0x68, 0x4f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ER1!", Number)),
            _ => None,
        },
        [0x00..=0x07] | [0xfb, 0x00..=0x01] | [0xfb, 0x80..=0x81, 0x7d] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("EW1!", Number)),
                (0, 0, 3, false) => Some(Tag("EW1!B", Number)),
                (0, 0, 1, false) => Some(Tag("EW1!D", Number)),
                (1..=15, 0, 0, false) => Some(Tag("EW1!T", Number)),
                (1, 0, 1, false) => Some(Tag("EW1!DT", Number)),
                (0, 0, 8..=199, false) => Some(Tag("EW1!P", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("EW1!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("EW1!RT", Number)),
                _ => None,
            }
        }
        [0x80..=0x87, 0x3c] | [0xfb, 0x80..=0x81, 0x3c] | [0xfb, 0x80..=0x81, 0xfd, 0x3c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("EW2!", Number)),
                (0, 0, 3, false) => Some(Tag("EW2!B", Number)),
                (0, 0, 1, false) => Some(Tag("EW2!D", Number)),
                (1..=15, 0, 0, false) => Some(Tag("EW2!T", Number)),
                (0, 0, 8..=199, false) => Some(Tag("EW2!P", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("EW2!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("EW2!RT", Number)),
                _ => None,
            }
        }
        [0x80..=0x87, 0xfc, 0x10]
        | [0xfb, 0x80..=0x81, 0xfc, 0x10]
        | [0xfb, 0x80..=0x81, 0xfd, 0xfc, 0x10] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("EW3!", Number)),
            (0, 0, 3, false) => Some(Tag("EW3!B", Number)),
            (1..=15, 0, 0, false) => Some(Tag("EW3!T", Number)),
            (0, 0, 8..=199, false) => Some(Tag("EW3!P", Number)),
            (0, 0, 0..=99 | 101..=124, true) => Some(Tag("EW3!R", Number)),
            (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("EW3!RT", Number)),
            _ => None,
        },
        [0xfb, 0x2c..=0x2f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("FR1!", Number)),
            _ => None,
        },
        [0x6e] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("HC1!", Number)),
            (0, 0, 1, false) => Some(Tag("HC1!D", Number)),
            (0, 0, 8..=199, false) => Some(Tag("HC1!P", Number)),
            _ => None,
        },
        [0xee, 0xfc, 0x11] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("HC1!", Number)),
            (0, 0, 8..=199, false) => Some(Tag("HC1!P", Number)),
            _ => None,
        },
        [0x78] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) if Serial.accepts(coding) => Some(Tag("ID1!", Serial)),
            (0, 0, 0, false) if Variable.accepts(coding) => Some(Tag("ID7!", Variable)),
            (0, 0, 0, false) => Some(Tag("ID1! / ID7!", Unmatched)),
            _ => None,
        },
        [0x79] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) if Address.accepts(coding) => Some(Tag("ID2!", Address)),
            (0, 0, 0, false) if Variable.accepts(coding) => Some(Tag("ID8!", Variable)),
            (0, 0, 0, false) => Some(Tag("ID2! / ID8!", Unmatched)),
            _ => None,
        },
        [0x7a] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID3!", Byte)),
            _ => None,
        },
        [0xfd, 0x11] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID4!", NumberOrVariable)),
            _ => None,
        },
        [0xfd, 0x10] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID5!", NumberOrVariable)),
            _ => None,
        },
        [0xfd, 0x08] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID6!", Integer)),
            _ => None,
        },
        [0xfd, 0x09] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID10!", Byte)),
            _ => None,
        },
        [0xfd, 0xfd, 0x01] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("ID11!", Variable)),
            _ => None,
        },
        [0x7c, 0x03, 0x31, 0x52, 0x49] | [0xfd, 0xfd, 0x21] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("IR1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x02, 0x78, 0x6c] | [0xfd, 0xfd, 0x1f] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("LT1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x02, 0x64, 0x63] | [0xfd, 0xfd, 0x20] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("LT2!", Number)),
                _ => None,
            }
        }
        [0xfd, 0x71] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM1!", Number)),
            (0, 2, 0, false) => Some(Tag("MM1!I", Number)),
            (0, 3, 0, false) => Some(Tag("MM1!E", Number)),
            _ => None,
        },
        [0xfd, 0x17] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM2!", Integer)),
            _ => None,
        },
        [0xfd, 0x97, 0x1d] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM3!", Integer)),
            _ => None,
        },
        [0xfd, 0x2a] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM4!", NumberOrVariable)),
            _ => None,
        },
        [0xfd, 0xfd, 0x00] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM6!", Integer)),
            _ => None,
        },
        [0xfd, 0x61] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM7!", Integer)),
            _ => None,
        },
        [0xfd, 0x74] | [0xfd, 0xfd, 0x02] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM8!", Number)),
            _ => None,
        },
        [0x7c, 0x03, 0x53, 0x42, 0x25] | [0xfd, 0xfd, 0x38] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MM12!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x31, 0x53, 0x44] | [0xfd, 0xfd, 0x40] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MM13!", Integer)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x32, 0x53, 0x44] | [0xfd, 0xfd, 0x42] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MM14!", Integer)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x33, 0x53, 0x44] | [0xfd, 0xfd, 0x41] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MM15!", Integer)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x34, 0x53, 0x44] | [0xfd, 0xfd, 0x44] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MM16!", Integer)),
                _ => None,
            }
        }
        [0xfd, 0xfd, 0x43] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("MM17!", Integer)),
            _ => None,
        },
        [0x7c, 0x03, 0x32, 0x48, 0x52] | [0xfd, 0xfd, 0x3e] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("MO1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x41, 0x42, 0x64] | [0xfd, 0xfd, 0x24] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("ND1!", Number)),
                _ => None,
            }
        }
        [0xfb, 0xaa, 0xfc, 0x05] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD1!", Number)),
            _ => None,
        },
        [0xfb, 0xaa, 0xfc, 0x06] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD2!", Number)),
            _ => None,
        },
        [0xfb, 0xaa, 0xfc, 0x07] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD3!", Number)),
            _ => None,
        },
        [0xfb, 0xab, 0xfc, 0x01] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD4!", Number)),
            _ => None,
        },
        [0xfb, 0xab, 0xfc, 0x02] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD5!", Number)),
            _ => None,
        },
        [0xfb, 0xab, 0xfc, 0x03] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PD6!", Number)),
            _ => None,
        },
        [0x7c, 0x02, 0x48, 0x70] | [0xfd, 0xfd, 0x27] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PH1!", Number)),
                _ => None,
            }
        }
        [0x30..=0x37] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PJ1!", Number)),
            (1, 0, 0, false) => Some(Tag("PJ1!T", Number)),
            _ => None,
        },
        [0xe8..=0xeb, 0x3e] | [0xe8..=0xeb, 0xf3, 0x3e] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PR1!", Number)),
                _ => None,
            }
        }
        [0x68..=0x6b] | [0xe8..=0xeb, 0x73] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PR2!", Number)),
            _ => None,
        },
        [0xe8..=0xeb, 0x48] | [0xe8..=0xeb, 0xc8, 0x73] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PR3!", Number)),
                _ => None,
            }
        }
        [0xe8..=0xeb, 0x40] | [0xe8..=0xeb, 0xc0, 0x73] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PR4!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x31, 0x54, 0x50] | [0xfd, 0xfd, 0x17] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x32, 0x54, 0x50] | [0xfd, 0xfd, 0x18] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT2!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x33, 0x54, 0x50] | [0xfd, 0xfd, 0x19] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT3!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x34, 0x54, 0x50] | [0xfd, 0xfd, 0x1a] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT4!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x35, 0x54, 0x50] | [0xfd, 0xfd, 0x1b] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT5!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x36, 0x54, 0x50] | [0xfd, 0xfd, 0x1c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT6!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x37, 0x54, 0x50] | [0xfd, 0xfd, 0x1d] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT7!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x38, 0x54, 0x50] | [0xfd, 0xfd, 0x1e] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PT8!", Number)),
                _ => None,
            }
        }
        [0x28..=0x2f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PW1!", Number)),
            (1, 0, 0, false) => Some(Tag("PW1!T", Number)),
            (0, 1, 0, false) => Some(Tag("PW1!A", Number)),
            (1..=15, 1, 0, false) => Some(Tag("PW1!AT", Number)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("PW1!AR", Number)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("PW1!ART", Number)),
            _ => None,
        },
        [0xa8..=0xaf, 0x3c] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("PW3!", Number)),
            (0, 1, 0, false) => Some(Tag("PW3!A", Number)),
            (1..=15, 1, 0, false) => Some(Tag("PW3!AT", Number)),
            (0, 1, 0..=99 | 101..=124, true) => Some(Tag("PW3!AR", Number)),
            (1..=15, 1, 0..=99 | 101..=124, true) => Some(Tag("PW3!ART", Number)),
            _ => None,
        },
        [0xfb, 0x78..=0x7f] => match (tariff, function, storage, recent) {
            (0, 1, 0, false) => Some(Tag("PW4!AC", Number)),
            (1..=15, 1, 0, false) => Some(Tag("PW4!ACT", Number)),
            _ => None,
        },
        [0xfb, 0xf8..=0xff, 0x3c] => match (tariff, function, storage, recent) {
            (0, 1, 0, false) => Some(Tag("PW6!AC", Number)),
            (1..=15, 1, 0, false) => Some(Tag("PW6!ACT", Number)),
            _ => None,
        },
        [0xa8..=0xaf, 0xfc, 0x10] | [0xfb, 0xa8..=0xa9, 0xfc, 0x10] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PW7!", Number)),
                _ => None,
            }
        }
        [0xa8..=0xaf, 0xfc, 0x0c] | [0xfb, 0xa8..=0xa9, 0xfc, 0x0c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("PW8!", Number)),
                _ => None,
            }
        }
        [0xfb, 0x02..=0x03] | [0xfb, 0x82..=0x83, 0x70..=0x77] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("RE1!", Number)),
                (1..=15, 0, 0, false) => Some(Tag("RE1!T", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("RE1!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("RE1!RT", Number)),
                _ => None,
            }
        }
        [0xfb, 0x82..=0x83, 0x3c] | [0xfb, 0x82..=0x83, 0xf0..=0xf7, 0x3c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("RE2!", Number)),
                (1..=15, 0, 0, false) => Some(Tag("RE2!T", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("RE2!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("RE2!RT", Number)),
                _ => None,
            }
        }
        [0xfd, 0xfd, 0x23] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("RF1!", Number)),
            _ => None,
        },
        [0xfb, 0x1a..=0x1b] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("RH1!", Number)),
            _ => None,
        },
        [0xfb, 0x14..=0x17] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("RP1!", Number)),
            _ => None,
        },
        [0xfb, 0x94..=0x97, 0x3c] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("RP2!", Number)),
            _ => None,
        },
        [0x7c, 0x03, 0x55, 0x4e, 0x46] | [0xfd, 0xfd, 0x25] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("TB1!", Number)),
                _ => None,
            }
        }
        [0x58..=0x5b] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("TC1!", Number)),
            _ => None,
        },
        [0x5c..=0x5f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("TC2!", Number)),
            _ => None,
        },
        [0xd8..=0xdb, 0x3e] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("TC3!", Number)),
            _ => None,
        },
        [0x64..=0x67] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("TC4!", Number)),
            _ => None,
        },
        [0xfb, 0x74..=0x77] => match (tariff, function, storage, recent) {
            (0, 2, 0, false) => Some(Tag("TC5!", Number)),
            (0, 1, 0, false) => Some(Tag("TC6!", Number)),
            _ => None,
        },
        [0x7c, 0x02, 0x4d, 0x4e] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("TS1!", Number)),
            _ => None,
        },
        [0x7c, 0x03, 0x31, 0x43, 0x56] | [0xfd, 0xfd, 0x14..=0x15] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VC1!", Number)),
                _ => None,
            }
        }
        [0x7c, 0x03, 0x32, 0x43, 0x56] | [0xfd, 0xfd, 0x16] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VC2!", Number)),
                _ => None,
            }
        }
        [0x38..=0x3f] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VF1!", Number)),
            (1, 0, 0, false) => Some(Tag("VF1!T", Number)),
            _ => None,
        },
        [0xb8..=0xbf, 0x3a] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VF2!", Number)),
            _ => None,
        },
        [0xb8..=0xbf, 0x3e] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VF3!", Number)),
            _ => None,
        },
        [0x10..=0x17] | [0x90..=0x97, 0x7d] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VM1!", Number)),
            (0, 0, 3, false) => Some(Tag("VM1!B", Number)),
            (0, 0, 1, false) => Some(Tag("VM1!D", Number)),
            (1..=15, 0, 0, false) => Some(Tag("VM1!T", Number)),
            (1, 0, 1, false) => Some(Tag("VM1!DT", Number)),
            (0, 0, 8..=199, false) => Some(Tag("VM1!P", Number)),
            (0, 0, 0..=99 | 101..=124, true) => Some(Tag("VM1!R", Number)),
            (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("VM1!RT", Number)),
            _ => None,
        },
        [0x90..=0x97, 0x3a] | [0x90..=0x97, 0xfd, 0x3a] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VM2!", Number)),
                (0, 0, 3, false) => Some(Tag("VM2!B", Number)),
                (1..=15, 0, 0, false) => Some(Tag("VM2!T", Number)),
                (0, 0, 8..=199, false) => Some(Tag("VM2!P", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("VM2!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("VM2!RT", Number)),
                _ => None,
            }
        }
        [0x90..=0x97, 0x3e] | [0x90..=0x97, 0xfd, 0x3e] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VM3!", Number)),
                (0, 0, 3, false) => Some(Tag("VM3!B", Number)),
                (1..=15, 0, 0, false) => Some(Tag("VM3!T", Number)),
                (0, 0, 8..=199, false) => Some(Tag("VM3!P", Number)),
                (0, 0, 0..=99 | 101..=124, true) => Some(Tag("VM3!R", Number)),
                (1..=15, 0, 0..=99 | 101..=124, true) => Some(Tag("VM3!RT", Number)),
                _ => None,
            }
        }
        [0x90..=0x97, 0x3b] | [0x90..=0x97, 0xfd, 0x3b] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VM4!", Number)),
                _ => None,
            }
        }
        [0x90..=0x97, 0x3c] | [0x90..=0x97, 0xfd, 0x3c] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("VM5!", Number)),
                _ => None,
            }
        }
        [0xfd, 0xc0..=0xcf, 0xfc, 0x01] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VV1!", Number)),
            _ => None,
        },
        [0xfd, 0xc0..=0xcf, 0xfc, 0x02] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VV2!", Number)),
            _ => None,
        },
        [0xfd, 0xc0..=0xcf, 0xfc, 0x03] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("VV3!", Number)),
            _ => None,
        },
        [0x7c, 0x03, 0x31, 0x53, 0x57] | [0xfd, 0xfd, 0x22] => {
            match (tariff, function, storage, recent) {
                (0, 0, 0, false) => Some(Tag("WS1!", Number)),
                _ => None,
            }
        }
        [0xfd, 0x23] => match (tariff, function, storage, recent) {
            (0, 0, 0, false) => Some(Tag("YD1!", Number)),
            (1..=15, 0, 0, false) => Some(Tag("YD1!T", Number)),
            _ => None,
        },
        [0xfd, 0x24..=0x27] | [0xfd, 0x28] | [0xfd, 0x29] => {
            match (tariff, function, storage, recent) {
                (0, 0, 1..=199, false) => Some(Tag("YD2!P", Number)),
                _ => None,
            }
        }
        [0xfd, 0x20] | [0xfd, 0x21] | [0xfd, 0x22] => match (tariff, function, storage, recent) {
            (0, 0, 1..=199, false) => Some(Tag("YD3!P", Number)),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn meter_tag(tag: &str, device: u8) -> Option<bool> {
    Some(match tag {
        "AD10!" | "AD11!" | "AD12!" | "AD13!" | "AD14!" | "AD4!" | "AD5!" | "AD6!" | "AD7!"
        | "AD8!" | "AD9!" | "CC1!" | "CC2!" | "CD1!" | "DT1!Xn" | "DT2!Xn" | "ER1!" | "ID11!"
        | "IR1!" | "LT1!" | "LT2!" | "MM12!" | "MM13!" | "MM14!" | "MM15!" | "MM16!" | "MM17!"
        | "MO1!" | "ND1!" | "PH1!" | "PR2!" | "PR3!" | "PR4!" | "PT1!" | "PT2!" | "PT3!"
        | "PT4!" | "PT5!" | "PT6!" | "PT7!" | "PT8!" | "RF1!" | "RH1!" | "TB1!" | "TC4!"
        | "TC5!" | "TC6!" | "TS1!" | "VC1!" | "VC2!" | "WS1!" => false,
        "CA1!" | "CA2!" | "CA3!" | "CA4!" | "DT1!A" | "DT1!AR" | "DT1!ART" | "DT1!AT" | "DT2!A"
        | "DT2!AR" | "DT2!ART" | "DT2!AT" | "DT3!A" | "DT3!AR" | "DT3!ART" | "DT3!AT" | "DT4!A"
        | "DT4!AR" | "DT4!ART" | "DT4!AT" | "DT5!" | "EW1!R" | "EW1!RT" | "EW2!R" | "EW2!RT"
        | "EW2!T" | "EW3!" | "EW3!B" | "EW3!P" | "EW3!R" | "EW3!RT" | "EW3!T" | "FR1!" | "MM7!"
        | "PD1!" | "PD2!" | "PD3!" | "PD4!" | "PD5!" | "PD6!" | "PW1!A" | "PW1!AR" | "PW1!ART"
        | "PW1!AT" | "PW3!" | "PW3!A" | "PW3!AR" | "PW3!ART" | "PW3!AT" | "PW4!AC" | "PW4!ACT"
        | "PW6!AC" | "PW6!ACT" | "PW7!" | "PW8!" | "RE1!" | "RE1!R" | "RE1!RT" | "RE1!T"
        | "RE2!" | "RE2!R" | "RE2!RT" | "RE2!T" | "RP1!" | "RP2!" | "VV1!" | "VV2!" | "VV3!" => {
            matches!(device, 0x02)
        }
        "DP1!" | "DP3!" | "DT1!P" | "YD1!" | "YD1!T" | "YD2!P" | "YD3!P" => matches!(
            device,
            0x02 | 0x08 | 0x0a | 0x0b | 0x0d | 0x04 | 0x0c | 0x03 | 0x07 | 0x16 | 0x06 | 0x15
        ),
        "CT1!" | "DP5!" | "DP6!" | "DT1!" | "DT2!" | "ID1!" | "ID10!" | "ID2!" | "ID3!"
        | "ID4!" | "ID5!" | "ID6!" | "ID7!" | "ID8!" | "MM1!" | "MM1!E" | "MM1!I" | "MM2!"
        | "MM3!" | "MM4!" | "MM6!" | "MM8!" => matches!(
            device,
            0x02 | 0x08
                | 0x0a
                | 0x0b
                | 0x0d
                | 0x04
                | 0x0c
                | 0x03
                | 0x07
                | 0x16
                | 0x06
                | 0x15
                | 0x20
                | 0x21
        ),
        "EW1!" | "EW1!P" | "PW1!" => matches!(device, 0x02 | 0x0a | 0x0b | 0x0d | 0x04 | 0x0c),
        "DP1!B" | "DP2!" | "DT1!B" | "DT5!B" => matches!(
            device,
            0x02 | 0x0a | 0x0b | 0x0d | 0x04 | 0x0c | 0x03 | 0x07 | 0x16 | 0x06 | 0x15
        ),
        "EW1!B" => matches!(device, 0x02 | 0x0a | 0x0b | 0x04 | 0x0c),
        "EW1!T" | "EW2!" | "EW2!B" | "EW2!P" => matches!(device, 0x02 | 0x0d),
        "DT1!R" | "DT2!R" => matches!(device, 0x02 | 0x03),
        "DC1!" | "DC2!" => matches!(
            device,
            0x02 | 0x03 | 0x07 | 0x16 | 0x06 | 0x15 | 0x20 | 0x21
        ),
        "HC1!" | "HC1!D" | "HC1!P" => matches!(device, 0x08),
        "TC1!" | "TC2!" => matches!(device, 0x08 | 0x0a | 0x0b | 0x0d | 0x04 | 0x0c),
        "DT2!D" | "DT2!N" => matches!(
            device,
            0x08 | 0x0a | 0x0b | 0x0d | 0x04 | 0x0c | 0x07 | 0x16 | 0x06 | 0x15
        ),
        "EC1!" | "EC1!D" | "EC1!P" | "EJ1!" | "EJ1!B" | "EJ1!D" | "EJ1!P" | "EW1!D" | "PJ1!" => {
            matches!(device, 0x0a | 0x0b | 0x0d | 0x04 | 0x0c)
        }
        "VF1!" | "VM1!" | "VM1!P" => matches!(
            device,
            0x0a | 0x0b | 0x0d | 0x04 | 0x0c | 0x03 | 0x07 | 0x16 | 0x06 | 0x15
        ),
        "VM1!D" => matches!(
            device,
            0x0a | 0x0b | 0x0d | 0x04 | 0x0c | 0x07 | 0x16 | 0x06 | 0x15
        ),
        "EC2!" | "EC2!D" | "EC2!P" | "EJ1!DT" | "EJ1!T" | "EJ2!" | "EJ2!B" | "EJ2!D" | "EJ2!P"
        | "EW1!DT" | "EW2!D" | "PJ1!T" | "PW1!T" | "VF1!T" | "VM1!DT" => matches!(device, 0x0d),
        "VM1!T" => matches!(device, 0x0d | 0x03),
        "PR1!" | "TC3!" | "VF2!" | "VF3!" | "VM1!R" | "VM1!RT" | "VM2!" | "VM2!B" | "VM2!P"
        | "VM2!R" | "VM2!RT" | "VM2!T" | "VM3!" | "VM3!B" | "VM3!P" | "VM3!R" | "VM3!RT"
        | "VM3!T" => matches!(device, 0x03),
        "VM1!B" => matches!(device, 0x03 | 0x07 | 0x16 | 0x06 | 0x15),
        "VM4!" | "VM5!" => matches!(device, 0x07 | 0x16 | 0x06 | 0x15),
        _ => return None,
    })
}
