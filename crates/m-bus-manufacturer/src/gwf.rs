use crate::{Cursor, DecodeError, Decoder, DeviceType, Field, Integer, MeterInfo, Unit, UnitName};

pub(crate) const fn water(version: u8, device: DeviceType) -> Decoder {
    Decoder {
        name: "GWF water radio",
        source: "https://github.com/wmbusmeters/wmbusmeters/blob/master/drivers/src/gwfwater.xmq",
        manufacturer: *b"GWF",
        versions: Some((version, version)),
        device: Some(device),
        decode: water_tail,
    }
}

fn water_tail(
    _: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    let mut cursor = Cursor::new(tail);
    if cursor.u8()? != 1 {
        return Ok(0);
    }
    let alarms = cursor.read(Cursor::u8)?;
    let power = cursor.read(Cursor::u8)?;
    emit(alarms.unsigned("alarms").flags(&[
        (1, "Continuous flow"),
        (3, "Broken pipe"),
        (5, "Low battery"),
        (6, "Backflow"),
    ]));
    emit(
        Field::unsigned(
            "power mode",
            u64::from(power.value & 1),
            power.range.clone(),
        )
        .enumeration(&[
            (Integer::Unsigned(0), "Normal"),
            (Integer::Unsigned(1), "Saving"),
        ]),
    );
    emit(
        Field::unsigned("battery life", u64::from(power.value >> 3) * 5, power.range)
            .exponent(-1)
            .units(&[Unit {
                name: UnitName::Year,
                exponent: 1,
            }]),
    );
    Ok(cursor.position())
}
