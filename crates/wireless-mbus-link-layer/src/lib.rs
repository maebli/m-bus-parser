use crc16::{EN_13757, State};
use m_bus_core::{
    ApplicationLayerError, DeviceType, FrameError, Function, IdentificationNumber, ManufacturerCode,
};

#[derive(Debug, PartialEq)]
pub enum Frame<'a> {
    FormatA {
        function: Function,
        manufacturer_id: ManufacturerId,
        data: &'a [u8],
    },
    FormatB {
        function: Function,
        manufacturer_id: ManufacturerId,
        data: &'a [u8],
    },
}

#[derive(Debug, PartialEq)]
pub struct ManufacturerId {
    manufacturer_code: ManufacturerCode,
    identification_number: IdentificationNumber,
    device_type: DeviceType,
    version: u8,
    is_unique_globally: bool,
}

impl TryFrom<&[u8]> for ManufacturerId {
    type Error = FrameError;
    fn try_from(data: &[u8]) -> Result<Self, FrameError> {
        let mut iter = data.iter();
        Ok(ManufacturerId {
            manufacturer_code: ManufacturerCode::from_id(u16::from_le_bytes([
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
            ]))
            .map_err(|e| FrameError::TooShort)?,
            identification_number: IdentificationNumber::from_bcd_hex_digits([
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
            ])
            .map_err(|e| FrameError::TooShort)?,
            version: *iter.next().ok_or(FrameError::TooShort)?,
            device_type: DeviceType::from(*iter.next().ok_or(FrameError::TooShort)?),
            is_unique_globally: false, /*todo not sure about this field*/
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for Frame<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, FrameError> {
        let length = data.len();
        let length_byte = *data.first().ok_or(FrameError::EmptyData)? as usize;
        let c_field = *data.get(1).ok_or(FrameError::TooShort)?;
        let manufacturer_id = ManufacturerId::try_from(&data[2..])?;

        match length_byte {
            length => Ok(Frame::FormatA {
                function: Function::try_from(c_field)?,
                manufacturer_id,
                data,
            }),
            l if l == length - 2 => Ok(Frame::FormatB {
                function: Function::try_from(c_field)?,
                manufacturer_id,
                data,
            }),
            _ => Err(FrameError::WrongLength {
                expected: length_byte,
                actual: data.len(),
            }),
        }
    }
}

fn validate_crc(data: &[u8]) -> Result<(), FrameError> {
    let crc_byte_index = data.len() - 2;
    let actual = State::<EN_13757>::calculate(&data[..crc_byte_index]);
    let expected: u16 = 0;
    if expected == actual {
        Ok(())
    } else {
        Err(FrameError::WrongCrc { expected, actual })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dummy() {
        let _id = 33225544;
        let _medium = 7; // water
        let _man = "SEN";
        let _version = 104;
        let frame: &[u8] = &[
            0x18, 0x44, 0xAE, 0x4C, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00, 0x00,
        ];
        let parsed = Frame::try_from(frame);
        println!("{:#?}", parsed);
    }
}
