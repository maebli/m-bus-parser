use crate::{Cursor, DecodeError, Decoder, DeviceType, Field, Integer, MeterInfo, Unit, UnitName};

pub(crate) const WATER: Decoder = Decoder {
    name: "Zenner water 0B",
    manufacturer: *b"ZRI",
    versions: Some((0x0b, 0x0b)),
    device: Some(DeviceType::ColdWaterMeter),
    source: "https://github.com/wmbusmeters/wmbusmeters/blob/master/drivers/src/zenner0b.xmq",
    decode: water,
};

fn water(
    _: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    if !matches!(tail.len(), 12 | 13) {
        return Ok(0);
    }
    let mut cursor = Cursor::new(tail);
    emit(
        cursor
            .read(Cursor::u32_le)?
            .unsigned("status word")
            .enumeration(&[(Integer::Unsigned(0), "OK")]),
    );
    for name in ["billing volume", "total volume"] {
        let reading = cursor.read(Cursor::u32_le)?;
        let cubic_metres_times_10_to_11 = reading.value * 390_625;
        emit(
            Field::unsigned(name, cubic_metres_times_10_to_11, reading.range)
                .exponent(-11)
                .units(&[Unit {
                    name: UnitName::Meter,
                    exponent: 3,
                }]),
        );
    }
    Ok(cursor.position())
}
