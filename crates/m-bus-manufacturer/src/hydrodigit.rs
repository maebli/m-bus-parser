use crate::{
    Cursor, DateValue, DecodeError, Decoder, DeviceType, Field, MeterInfo, Month,
    SingleEveryOrInvalid::Single, Unit, UnitName, Value,
};

pub(crate) const fn water(version: u8, device: DeviceType) -> Decoder {
    Decoder {
        name: "B METERS HYDRODIGIT water",
        source: "https://github.com/wmbusmeters/wmbusmeters/blob/master/drivers/src/hydrodigit.xmq",
        manufacturer: *b"BMT",
        versions: Some((version, version)),
        device: Some(device),
        decode: water_tail,
    }
}

const VOLUME: &[Unit] = &[Unit {
    name: UnitName::Meter,
    exponent: 3,
}];
const MONTHS: [Month; 12] = [
    Month::January,
    Month::February,
    Month::March,
    Month::April,
    Month::May,
    Month::June,
    Month::July,
    Month::August,
    Month::September,
    Month::October,
    Month::November,
    Month::December,
];

fn event_date(
    cursor: &mut Cursor<'_>,
    name: &str,
    fraud: bool,
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<(), DecodeError> {
    let start = cursor.position();
    let bytes = cursor.take(3)?;
    let month = if fraud { bytes[1] & 0x1f } else { bytes[1] };
    if fraud {
        emit(
            Field::unsigned(
                "tamper indicators",
                u64::from(bytes[1] & 0xe0),
                start + 1..start + 2,
            )
            .flags(&[
                (5, "Module removed"),
                (6, "Sensor interference"),
                (7, "Magnetic interference"),
            ]),
        );
    }
    if [bytes[0], month, bytes[2]] == [0, 0, 0] {
        return Ok(());
    }
    let packed = [bytes[0], month, bytes[2]];
    let mut date = Cursor::new(&packed);
    let components = (date.bcd(2, false), date.bcd(2, false), date.bcd(2, false));
    if let (Ok(y), Ok(m @ 1..=12), Ok(d @ 1..=31)) = components {
        let year = 2000 + y as u16;
        let last_day = match m {
            2 if year.is_multiple_of(4) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if d <= last_day {
            emit(Field::new(
                name,
                Value::Date(DateValue::Date {
                    year: Single(year),
                    month: Single(MONTHS[m as usize - 1]),
                    day: Single(d as u8),
                }),
                start..cursor.position(),
            ));
        }
    }
    Ok(())
}

fn water_tail(
    _: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    let mut cursor = Cursor::new(tail);
    let layout = cursor.u8()?;
    if layout & 1 != 0 {
        let battery = cursor.read(Cursor::u8)?;
        let centivolts = [
            370u64, 190, 210, 220, 230, 240, 250, 265, 280, 290, 305, 320, 335, 350, 370, 370,
        ];
        emit(
            Field::unsigned(
                "battery voltage",
                centivolts[usize::from(battery.value & 15)],
                battery.range,
            )
            .exponent(-2)
            .units(&[Unit {
                name: UnitName::Volt,
                exponent: 1,
            }]),
        );
    }
    if layout & 2 != 0 {
        event_date(&mut cursor, "tamper date", true, emit)?;
    }
    if layout & 0x80 != 0 {
        event_date(&mut cursor, "leak date", false, emit)?;
    }
    if layout & 4 != 0 {
        emit(
            cursor
                .read(Cursor::u32_le)?
                .unsigned("reverse volume")
                .exponent(-3)
                .units(VOLUME),
        );
    }
    if layout & 0x10 != 0 {
        for name in [
            "January total",
            "February total",
            "March total",
            "April total",
            "May total",
            "June total",
            "July total",
            "August total",
            "September total",
            "October total",
            "November total",
            "December total",
        ] {
            let volume = cursor.read(Cursor::u24_le)?;
            if volume.value < 10_000_000 {
                emit(volume.unsigned(name).exponent(-2).units(VOLUME));
            }
        }
    }
    Ok(cursor.position())
}
