//! Synthetic layout: status byte, then optional signed temperature in 0.01 °C.
use m_bus_manufacturer::{
    Cursor, DecodeError, DecoderDescriptor, Field, ManufacturerDecoder, MeterInfo, Registry,
    Selector, Unit, UnitName,
};

struct Decoder;
impl ManufacturerDecoder for Decoder {
    fn descriptor(&self) -> DecoderDescriptor {
        DecoderDescriptor {
            name: "Synthetic temperature meter",
            source: "Example only; replace with a vendor specification URL and section",
            selector: Selector {
                manufacturer: *b"ABC",
                versions: Some((1, 1)),
                device: None,
            },
        }
    }
    fn decode(
        &self,
        _: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Result<usize, DecodeError> {
        let mut cursor = Cursor::new(tail);
        let status = cursor.u8()?;
        emit(Field::unsigned("status", status.into(), 0..1).flags(&[(1, "low_battery")]));
        if status & 1 != 0 {
            let temperature = cursor.i16_le()?;
            emit(
                Field::signed("temperature", temperature, 1..3)
                    .exponent(-2)
                    .units(&[Unit {
                        name: UnitName::Celsius,
                        exponent: 1,
                    }]),
            );
        }
        Ok(cursor.position())
    }
}

const METER: MeterInfo = MeterInfo {
    manufacturer: Some(*b"ABC"),
    version: Some(1),
    device: None,
};

fn main() {
    Registry::new(&[&Decoder])
        .decode(&METER, &[3, 0x29, 9], &mut |field| println!("{field:?}"))
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
            let consumed = Decoder
                .decode(&METER, &tail, &mut |field| {
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
        assert_eq!(Decoder.decode(&METER, &[0], &mut |_| count += 1), Ok(1));
        assert_eq!(count, 1);
        for tail in [&[][..], &[1][..], &[1, 0x29][..]] {
            let error = Decoder.decode(&METER, tail, &mut |_| {}).unwrap_err();
            assert_eq!(error.offset, usize::from(!tail.is_empty()));
            assert!(matches!(error.kind, ErrorKind::InsufficientData { .. }));
        }
    }
}
