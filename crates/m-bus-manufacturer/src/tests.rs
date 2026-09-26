use super::*;
use std::{borrow::ToOwned, vec::Vec};

#[test]
fn exact_integers_all_widths_and_endianness() {
    for width in 1..=8 {
        let max_unsigned = u64::MAX >> ((8 - width) * 8);
        for big in [false, true] {
            assert_eq!(
                Cursor::new(&[0xff; 8]).unsigned(width, big),
                Ok(max_unsigned)
            );
            assert_eq!(Cursor::new(&[0xff; 8]).signed(width, big), Ok(-1));
            let mut bytes = [0; 8];
            bytes[if big { 0 } else { width - 1 }] = 0x80;
            assert_eq!(
                Cursor::new(&bytes).signed(width, big),
                Ok(i64::MIN >> ((8 - width) * 8))
            );
            let value = 0x0123_4567_89ab_cdefu64 & max_unsigned;
            let le = value.to_le_bytes();
            let be = value.to_be_bytes();
            assert_eq!(
                Cursor::new(if big { &be[8 - width..] } else { &le[..width] }).unsigned(width, big),
                Ok(value)
            );
        }
    }
}

#[test]
fn failed_cursor_reads_do_not_advance() {
    let mut cursor = Cursor::new(&[1, 2, 3]);
    assert_eq!(cursor.u8(), Ok(1));
    for size in [3, usize::MAX] {
        assert_eq!(
            cursor.take(size),
            Err(DecodeError::new(
                1,
                ErrorKind::InsufficientData {
                    needed: size,
                    remaining: 2
                }
            ))
        );
        assert_eq!(cursor.position(), 1);
    }
    assert_eq!(
        cursor.unsigned(0, false).unwrap_err().kind,
        ErrorKind::InvalidWidth
    );
    assert_eq!(
        cursor.signed(9, true).unwrap_err().kind,
        ErrorKind::InvalidWidth
    );
    assert_eq!(cursor.rest(), &[2, 3]);
    assert!(cursor.remaining().is_empty());
}

#[test]
fn bcd_is_exact_and_rejects_invalid_digits() {
    assert_eq!(Cursor::new(&[0x99; 6]).bcd(12, false), Ok(999_999_999_999));
    assert_eq!(Cursor::new(&[0x12, 0x34]).bcd(4, true), Ok(1234));
    assert_eq!(Cursor::new(&[0x12, 0x34]).bcd(4, false), Ok(3412));
    for byte in 0u8..=255 {
        let bytes = [byte];
        let mut cursor = Cursor::new(&bytes);
        if byte >> 4 > 9 || byte & 15 > 9 {
            assert_eq!(
                cursor.bcd(2, false),
                Err(DecodeError::new(0, ErrorKind::InvalidBcd))
            );
            assert_eq!(cursor.position(), 0);
        } else {
            assert_eq!(
                cursor.bcd(2, false),
                Ok(u64::from(byte >> 4) * 10 + u64::from(byte & 15))
            );
        }
    }
    assert_eq!(
        Cursor::new(&[]).bcd(3, false).unwrap_err().kind,
        ErrorKind::InvalidWidth
    );
}

#[test]
fn dates_reuse_protocol_components_and_check_lengths() {
    let date = Cursor::new(&[0x8c, 0x11]).date_g().unwrap();
    assert!(matches!(
        date,
        DateValue::Date {
            day: SingleEveryOrInvalid::Single(12),
            ..
        }
    ));
    let mut cursor = Cursor::new(&[0, 0, 0]);
    assert!(cursor.datetime_f().is_err());
    assert_eq!(cursor.position(), 0);
    // Invalid/every components remain representable, rather than failing a whole record.
    assert!(Cursor::new(&[0xff; 4]).datetime_f().is_ok());
}

#[test]
fn selectors_require_metadata_and_validate_restrictions() {
    let selector = Decoder {
        manufacturer: *b"ABC",
        versions: Some((1, 3)),
        device: Some(DeviceType::WaterMeter),
        ..DEMO
    };
    let mut meter = MeterInfo {
        manufacturer: Some(*b"ABC"),
        version: Some(3),
        device: Some(DeviceType::WaterMeter),
    };
    assert!(selector.matches(&meter));
    meter.version = None;
    assert!(!selector.matches(&meter));
    meter.version = Some(3);
    meter.device = None;
    assert!(!selector.matches(&meter));
    assert!(!Decoder {
        versions: Some((4, 1)),
        ..selector
    }
    .is_valid());
    assert!(!Decoder {
        manufacturer: *b"abc",
        ..selector
    }
    .is_valid());
}

const DEMO: Decoder = Decoder {
    name: "Demo",
    source: "Synthetic test",
    manufacturer: *b"ABC",
    versions: None,
    device: None,
    decode: demo,
};

fn demo(_: &MeterInfo, tail: &[u8], emit: &mut dyn FnMut(Field<'_>)) -> Result<usize, DecodeError> {
    let mut cursor = Cursor::new(tail);
    let value = cursor.u8()?;
    emit(Field::unsigned("value", value.into(), 0..1));
    if value == 0 {
        cursor.skip(2)?;
    }
    Ok(cursor.position())
}

#[test]
fn first_match_leftovers_and_partial_errors() {
    let decoders = &[
        DEMO,
        Decoder {
            decode: |_, _, _| panic!("must not retry"),
            ..DEMO
        },
    ];
    let meter = MeterInfo {
        manufacturer: Some(*b"ABC"),
        ..MeterInfo::default()
    };
    let mut fields = Vec::new();
    let result = decode(decoders, &meter, &[1, 2], &mut |f| {
        fields.push((f.name.to_owned(), f.range))
    })
    .unwrap()
    .unwrap();
    assert_eq!(result.consumed, 1);
    assert_eq!(fields, [("value".into(), 0..1), ("unparsed".into(), 1..2)]);
    fields.clear();
    assert_eq!(
        decode(decoders, &meter, &[0], &mut |f| fields
            .push((f.name.to_owned(), f.range)))
        .unwrap()
        .unwrap_err(),
        DecodeError::new(
            1,
            ErrorKind::InsufficientData {
                needed: 2,
                remaining: 0
            }
        )
    );
    assert_eq!(fields, [("value".into(), 0..1)]);
    assert!(
        decode(decoders, &MeterInfo::default(), &[1], &mut |_| panic!(
            "no match"
        ))
        .is_none()
    );
}

#[test]
fn full_width_enum_and_flags_do_not_round() {
    let field = Field::unsigned("wide", u64::MAX, 0..8)
        .enumeration(&[(Integer::Unsigned(u64::MAX), "maximum")]);
    assert_eq!(field.active_labels().collect::<Vec<_>>(), ["maximum"]);
    let flags = Field::unsigned("flags", 1 << 63, 0..8).flags(&[(63, "top"), (64, "invalid")]);
    assert_eq!(flags.active_labels().collect::<Vec<_>>(), ["top"]);
    let negative = Field::signed("negative", i64::MIN, 0..8)
        .enumeration(&[(Integer::Signed(i64::MIN), "minimum")]);
    assert_eq!(negative.active_labels().collect::<Vec<_>>(), ["minimum"]);
}

#[test]
fn bad_decoder_ranges_and_lengths_return_errors() {
    let meter = MeterInfo {
        manufacturer: Some(*b"ABC"),
        ..MeterInfo::default()
    };
    let bad_functions: [DecodeFn; 2] = [
        |_, _, emit| {
            emit(Field::unsigned("bad", 0, 0..usize::MAX));
            Ok(0)
        },
        |_, _, _| Ok(usize::MAX),
    ];
    for function in bad_functions {
        let decoders = [Decoder {
            decode: function,
            ..DEMO
        }];
        assert!(decode(&decoders, &meter, &[], &mut |_| panic!(
            "invalid field emitted"
        ))
        .unwrap()
        .is_err());
    }
}

#[test]
fn readings_track_offsets_and_failed_reads_roll_back() {
    let mut cursor = Cursor::new(&[0, 6, 0xff, 42]);
    cursor.skip(1).unwrap();
    let temperature = cursor.read(Cursor::i16_le).unwrap();
    assert_eq!(temperature.value, -250);
    assert_eq!(
        temperature.signed("temperature"),
        Field::signed("temperature", -250, 1..3)
    );
    let error = cursor
        .read(|cursor| {
            cursor.u8()?;
            cursor.u16_le()
        })
        .unwrap_err();
    assert_eq!(error.offset, 4);
    assert_eq!(cursor.position(), 3);
    let status = cursor.read(Cursor::u8).unwrap();
    assert_eq!(
        status.unsigned("status"),
        Field::unsigned("status", 42, 3..4)
    );
}
