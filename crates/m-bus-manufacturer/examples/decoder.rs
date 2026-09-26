//! Synthetic layout only. Copy the Decoder implementation and tests into decoders/.
use m_bus_manufacturer::{
    Cursor, DecodeError, DecoderDescriptor, Field, ManufacturerDecoder, MeterInfo, Selector, Unit,
    UnitName,
};

pub struct Decoder;
impl ManufacturerDecoder for Decoder {
    fn descriptor(&self) -> &'static DecoderDescriptor {
        static DESCRIPTOR: DecoderDescriptor = DecoderDescriptor {
            name: "Synthetic temperature meter",
            source: "Synthetic example; replace with a vendor specification URL and section",
            selector: Selector {
                manufacturer: *b"ABC",
                versions: Some((1, 1)),
                device: None,
            },
        };
        &DESCRIPTOR
    }
    fn decode(
        &self,
        _: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Result<usize, DecodeError> {
        let mut cursor = Cursor::new(tail);
        let status = cursor.u8()?;
        emit(
            Field::unsigned("status", u64::from(status), 0..1)
                .flags(&[(0, "has_temperature"), (1, "low_battery")]),
        );
        if status & 1 != 0 {
            let start = cursor.position();
            let temperature = cursor.i16_le()?;
            emit(
                Field::signed("temperature", temperature, start..cursor.position())
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
fn main() {
    let meter = MeterInfo {
        manufacturer: Some(*b"ABC"),
        version: Some(1),
        device: None,
    };
    m_bus_manufacturer::Registry::only(&[&Decoder])
        .decode(&meter, &[3, 0x29, 9], &mut |field| println!("{field:?}"))
        .expect("matching example")
        .expect("valid sample");
}

#[cfg(test)]
mod tests {
    use super::*;
    use m_bus_manufacturer::{
        testing::{assert_fixtures, Fixture},
        ErrorKind, Integer, Labels, Value,
    };
    const METER: MeterInfo = MeterInfo {
        manufacturer: Some(*b"ABC"),
        version: Some(1),
        device: None,
    };
    const FLAGS: Labels<'static> = Labels::Flags(&[(0, "has_temperature"), (1, "low_battery")]);
    pub(super) const FIXTURES: &[Fixture<'_>] = &[
        Fixture {
            name: "positive",
            meter: METER,
            tail: &[3, 0x29, 9],
            fields: &[
                Field {
                    name: "status",
                    value: Value::Integer(Integer::Unsigned(3)),
                    range: 0..1,
                    exponent: 0,
                    units: &[],
                    labels: FLAGS,
                },
                Field {
                    name: "temperature",
                    value: Value::Integer(Integer::Signed(2345)),
                    range: 1..3,
                    exponent: -2,
                    units: &[Unit {
                        name: UnitName::Celsius,
                        exponent: 1,
                    }],
                    labels: Labels::None,
                },
            ],
            result: Ok(3),
        },
        Fixture {
            name: "negative",
            meter: METER,
            tail: &[1, 6, 0xff],
            fields: &[
                Field {
                    name: "status",
                    value: Value::Integer(Integer::Unsigned(1)),
                    range: 0..1,
                    exponent: 0,
                    units: &[],
                    labels: FLAGS,
                },
                Field {
                    name: "temperature",
                    value: Value::Integer(Integer::Signed(-250)),
                    range: 1..3,
                    exponent: -2,
                    units: &[Unit {
                        name: UnitName::Celsius,
                        exponent: 1,
                    }],
                    labels: Labels::None,
                },
            ],
            result: Ok(3),
        },
        Fixture {
            name: "leftover",
            meter: METER,
            tail: &[0, 0xaa],
            fields: &[Field {
                name: "status",
                value: Value::Integer(Integer::Unsigned(0)),
                range: 0..1,
                exponent: 0,
                units: &[],
                labels: FLAGS,
            }],
            result: Ok(1),
        },
        Fixture {
            name: "truncated",
            meter: METER,
            tail: &[1, 0x29],
            fields: &[Field {
                name: "status",
                value: Value::Integer(Integer::Unsigned(1)),
                range: 0..1,
                exponent: 0,
                units: &[],
                labels: FLAGS,
            }],
            result: Err(DecodeError::new(
                1,
                ErrorKind::InsufficientData {
                    needed: 2,
                    remaining: 1,
                },
            )),
        },
    ];
    #[test]
    fn known_answers() {
        assert_fixtures(&Decoder, FIXTURES);
    }
}
