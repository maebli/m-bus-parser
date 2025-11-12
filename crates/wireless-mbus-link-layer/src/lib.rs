use m_bus_core::{ApplicationLayerError, IdentificationNumber, ManufacturerCode};

#[derive(Debug, PartialEq)]
pub enum Frame<'a> {
    FormatA { function: Function, data: &'a [u8] },
    FormatB { function: Function, data: &'a [u8] },
}

#[derive(Debug, PartialEq)]
pub enum Function {
    SndNke { prm: bool },
    SndUd { prm: bool },
    SndUd2 { prm: bool },
    SndNR { prm: bool },
    SndUd3 { prm: bool },
    SndIr { prm: bool },
    AccNr { prm: bool },
    AccDmd { prm: bool },
    ReqUd1 { prm: bool },
    ReqUd2 { prm: bool },
    Ack { prm: bool },
    Nack { prm: bool },
    CnfIr { prm: bool },
    RspUd { prm: bool },
}

pub struct DeviceType {}

pub struct ManufacturerId {
    manufacturer_code: ManufacturerCode,
    device_type: DeviceType,
    version: u8,
    is_unique_globally: bool,
}

// check if this can be unified with wired mbus frame error some how
#[derive(Debug, PartialEq)]
pub enum FrameError {
    EmptyData,
    TooShort,
    WrongLength { expected: usize, actual: usize },
}

impl TryFrom<u8> for Function {
    type Error = FrameError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            _ => todo!(),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Frame<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, FrameError> {
        let length = data.len();
        let length_byte = *data.first().ok_or(FrameError::EmptyData)? as usize;
        let c_field = *data.get(1).ok_or(FrameError::TooShort)? as usize;

        match length_byte {
            length => Ok(Frame::FormatA {
                function: Function::SndNke { prm: false },
                data,
            }),
            l if l == length - 2 => Ok(Frame::FormatB {
                function: Function::SndNke { prm: false },
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
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dummy() {
        let _id = 33225544;
        let frame: &[u8] = &[
            0x18, 0x44, 0xAE, 0x4C, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00, 0x00,
        ];
        let parsed = Frame::try_from(frame);
        println!("{:#?}", parsed);
    }
}
