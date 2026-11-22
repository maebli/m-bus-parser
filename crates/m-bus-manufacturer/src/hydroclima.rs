use crate::{Cursor, DecodeError, Decoder, DeviceType, Field, MeterInfo, Unit, UnitName};

pub(crate) const fn allocator(version: u8) -> Decoder {
    Decoder {
        name: "B METERS HYDROCLIMA 2",
        source:
            "https://github.com/wmbusmeters/wmbusmeters/blob/master/drivers/src/hydroclimav2.xmq",
        manufacturer: *b"BMP",
        versions: Some((version, version)),
        device: Some(DeviceType::HeatCostAllocator),
        decode: allocator_tail,
    }
}

const CELSIUS: &[Unit] = &[Unit {
    name: UnitName::Celsius,
    exponent: 1,
}];
const HCA: &[Unit] = &[Unit {
    name: UnitName::HCA,
    exponent: 1,
}];

fn temperature(
    cursor: &mut Cursor<'_>,
    name: &str,
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<(), DecodeError> {
    let reading = cursor.read(Cursor::i16_le)?;
    if reading.value != i64::from(i16::MIN) {
        emit(reading.signed(name).exponent(-2).units(CELSIUS));
    }
    Ok(())
}

fn allocator_tail(
    meter: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    let mut cursor = Cursor::new(tail);
    let format = cursor.u8()?;
    if !matches!(
        (meter.version, format),
        (Some(0x53), 0x10) | (Some(0x85), 0x11)
    ) {
        return Ok(0);
    }
    emit(
        cursor
            .read(Cursor::u16_le)?
            .unsigned("alarm register")
            .flags(&[
                (0, "Configuration missing"),
                (1, "Verification pending"),
                (8, "Tamper"),
                (9, "Heater temperature out of range"),
                (10, "Heater temperature low"),
                (11, "Consumption overflow"),
                (13, "Oscillator fault"),
                (15, "Power failure"),
            ]),
    );
    cursor.skip(4)?;
    if format == 0x11 {
        cursor.skip(2)?;
        temperature(&mut cursor, "average heater temperature", emit)?;
    }
    temperature(&mut cursor, "average ambient temperature", emit)?;
    temperature(&mut cursor, "maximum ambient temperature", emit)?;
    cursor.skip(2)?;
    if format == 0x10 {
        cursor.skip(2)?;
        temperature(&mut cursor, "previous average ambient temperature", emit)?;
        temperature(&mut cursor, "previous average heater temperature", emit)?;
        cursor.skip(2)?;
    } else {
        temperature(&mut cursor, "previous average heater temperature", emit)?;
        temperature(&mut cursor, "previous average ambient temperature", emit)?;
        for name in [
            "consumption at set date 1",
            "consumption at set date 2",
            "consumption at set date 3",
            "consumption at set date 4",
        ] {
            emit(cursor.read(Cursor::u16_le)?.unsigned(name).units(HCA));
        }
        for name in [
            "ambient temperature at set date 1",
            "ambient temperature at set date 2",
            "ambient temperature at set date 3",
        ] {
            temperature(&mut cursor, name, emit)?;
        }
        cursor.skip(9)?;
    }
    emit(
        cursor
            .read(Cursor::u24_le)?
            .unsigned("total consumption")
            .units(HCA),
    );
    Ok(cursor.position())
}
