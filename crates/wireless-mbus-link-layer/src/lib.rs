pub enum Frame<'a> {
    FormatA { function: Function, data: &'a [u8] },
    FormatB { function: Function },
}

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

// check if this can be unified with wired mbus frame error some how
pub enum FrameError {
    EmptyData,
    WrongLength,
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
        let length = *data.first().ok_or(FrameError::EmptyData)? as usize;

        if length != data.len() {
            return Err(FrameError::WrongLength);
        }
        Ok(Frame::FormatA {
            function: Function::SndNke { prm: false },
            data,
        })
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
    fn test_dummy() {}
}
