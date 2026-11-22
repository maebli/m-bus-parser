use crate::{Cursor, DecodeError, Decoder, DeviceType, Field, MeterInfo};

pub(crate) const fn status(version: u8) -> Decoder {
    Decoder {
        name: "ULTRAHEAT error register",
        manufacturer: *b"LUG",
        versions: Some((version, version)),
        device: Some(DeviceType::HeatMeterReturn),
        source:
            "https://github.com/wmbusmeters/wmbusmeters/blob/master/drivers/src/qheat_55_us.xmq",
        decode: error_register,
    }
}

fn error_register(
    _: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Result<usize, DecodeError> {
    if tail.len() != 3 {
        return Ok(0);
    }
    let mut cursor = Cursor::new(tail);
    emit(
        cursor
            .read(Cursor::u24_le)?
            .unsigned("error register")
            .flags(&[
                (0, "F0 flow warning"),
                (1, "F4 battery warning"),
                (2, "Temperature sensor installation"),
                (3, "Volume unit installation"),
                (4, "Negative temperature difference"),
                (5, "Negative flow"),
                (8, "F8 persistent fault"),
                (9, "F9 communication fault"),
                (16, "F0 flow fault"),
                (17, "F1 flow temperature sensor open"),
                (18, "F2 return temperature sensor open"),
                (19, "F3 temperature electronics fault"),
                (20, "F4 battery empty"),
                (21, "F5 flow temperature sensor short"),
                (22, "F6 return temperature sensor short"),
                (23, "F7 memory fault"),
            ]),
    );
    Ok(cursor.position())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode;

    #[test]
    fn f001_flow_fault_and_opaque_service_frames() {
        let meter = MeterInfo {
            manufacturer: Some(*b"LUG"),
            version: Some(10),
            device: Some(DeviceType::HeatMeterReturn),
        };
        let mut seen = false;
        decode(crate::BUILTIN, &meter, &[0, 0, 1], &mut |f| {
            assert!(f.active_labels().eq(["F0 flow fault"]));
            assert_eq!(f.range, 0..3);
            seen = true;
        })
        .unwrap()
        .unwrap();
        assert!(seen);
        for tail in [&[0u8, 0][..], &[0, 0, 1, 2]] {
            let result = decode(crate::BUILTIN, &meter, tail, &mut |f| {
                assert_eq!(f.name, "unparsed")
            })
            .unwrap()
            .unwrap();
            assert_eq!(result.consumed, 0);
        }
    }
}
