//! Synthetic layout: status byte, then optional signed temperature in 0.01 °C.
use m_bus_manufacturer::{decode, Cursor, DecodeError, Decoder, Field, MeterInfo, Unit, UnitName};

const HAS_TEMPERATURE: u8 = 1 << 0;
const LOW_BATTERY: &[(u8, &str)] = &[(1, "low_battery")];
const CELSIUS: &[Unit] = &[Unit {
    name: UnitName::Celsius,
    exponent: 1,
}];

fn decode_abc(
    _: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    let mut cursor = Cursor::new(tail);
    let status = cursor.read(Cursor::u8)?;
    emit(status.unsigned("status").flags(LOW_BATTERY));

    if status.value & HAS_TEMPERATURE != 0 {
        let temperature = cursor.read(Cursor::i16_le)?;
        let field = temperature
            .signed("temperature")
            .exponent(-2)
            .units(CELSIUS);
        emit(field);
    }
    Ok(cursor.position())
}

const DECODERS: &[Decoder] = &[Decoder {
    name: "Synthetic temperature meter",
    source: "Example only; replace with a vendor specification URL and section",
    manufacturer: *b"ABC",
    versions: Some((1, 1)),
    device: None,
    decode: decode_abc,
}];

const METER: MeterInfo = MeterInfo {
    manufacturer: Some(*b"ABC"),
    version: Some(1),
    device: None,
};

fn main() {
    decode(DECODERS, &METER, &[3, 0x29, 9], &mut |field| {
        println!("{field:?}")
    })
    .expect("matching example")
    .expect("valid sample");
}

#[cfg(test)]
mod tests {
    use super::*;
    use m_bus_manufacturer::{ErrorKind, Integer, Value};

    #[test]
    fn temperature_and_status() {
        for (tail, expected) in [([3, 0x29, 9], 2345), ([3, 6, 0xff], -250)] {
            let mut count = 0;
            let consumed = decode_abc(&METER, &tail, &mut |field| {
                if count == 0 {
                    assert_eq!(
                        field,
                        Field::unsigned("status", 3, 0..1).flags(&[(1, "low_battery")])
                    );
                    assert_eq!(field.active_labels().collect::<Vec<_>>(), ["low_battery"]);
                } else {
                    assert_eq!(field.name, "temperature");
                    assert_eq!(field.value, Value::Integer(Integer::Signed(expected)));
                    assert_eq!(field.range, 1..3);
                    assert_eq!(field.exponent, -2);
                    assert_eq!(
                        field.units,
                        &[Unit {
                            name: UnitName::Celsius,
                            exponent: 1
                        }]
                    );
                }
                count += 1;
            })
            .unwrap();
            assert_eq!((consumed, count), (3, 2));
        }
    }

    #[test]
    fn absent_temperature_and_truncation() {
        let mut count = 0;
        assert_eq!(decode_abc(&METER, &[0], &mut |_| count += 1), Ok(1));
        assert_eq!(count, 1);
        for tail in [&[][..], &[1][..], &[1, 0x29][..]] {
            let error = decode_abc(&METER, tail, &mut |_| {}).unwrap_err();
            assert_eq!(error.offset, usize::from(!tail.is_empty()));
            assert!(matches!(error.kind, ErrorKind::InsufficientData { .. }));
        }
    }
}
