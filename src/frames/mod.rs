#[derive(Debug,PartialEq)]
pub enum FrameType<'a> {
    SingleCharacter{
        character: u8
    },
    ShortFrame{
        function: Function, 
        address: u8
    },
    LongFrame{
        function: Function,
        address: u8,
        control_information: u8, 
        data: &'a [u8]},
    ControlFrame{
        function: Function, 
        address: u8, 
        control_information: u8
    },
}

#[derive(Debug,PartialEq)]
pub enum Function{
    SND_NK,
    SND_UD{FCB: bool},
    REQ_UD2{FCB: bool},
    REQ_UD1{FCB: bool},
    RSP_UD{ACD:bool, DFC:bool}
}

impl Function {
    fn from(byte: u8) -> Function {
        match byte {
            0x40 => Function::SND_NK,
            0x53 => Function::SND_UD{FCB: false},
            0x73 => Function::SND_UD{FCB: true},
            0x5B => Function::REQ_UD2{FCB: false},
            0x7B => Function::REQ_UD2{FCB: true},
            0x5A => Function::REQ_UD1{FCB: false},
            0x7A => Function::REQ_UD1{FCB: true},
            0x08 => Function::RSP_UD{ACD: false, DFC: false},
            0x18 => Function::RSP_UD{ACD: false, DFC: true},
            0x28 => Function::RSP_UD{ACD: true, DFC: false},
            0x38 => Function::RSP_UD{ACD: true, DFC: true},
            _ => panic!("Invalid function byte: {}", byte),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FrameError {
    EmptyData,
    InvalidStartByte,
    InvalidStopByte,
    LengthMismatch,
    LengthShorterThanSix{ 
        length: usize
    },
    WrongChecksum{
        expected: u8,
        actual: u8,
    },
}

pub trait Frame {
    fn from(data: &[u8]) -> Result<Self, FrameError>
    where
        Self: Sized;
}

pub fn parse(data: &[u8])  {
    match parse_frame(data) {
        Ok(frame) => println!("Frame: {:?}", frame),
        Err(e) => println!("Error: {:?}", e),
    }
}

pub fn parse_frame(data: &[u8])  -> Result<FrameType, FrameError> {

    if data.is_empty() {
        return Err(FrameError::EmptyData);
    }

    if data.len() == 1 && data[0] == 0xE5 {
        return Ok(FrameType::SingleCharacter{character: 0xE5});
    }
    
    match data[0] {
        0x68 => {
            
            if data[data.len() - 1] != 0x16 {
                return Err(FrameError::InvalidStopByte);
            }

            if data.len() < 6 {
                return Err(FrameError::LengthShorterThanSix{length: data.len()});
            }

            validate_checksum(&data[4..])?;

            let length = data[1] as usize;

            if data[1] != data[2] || data.len() != length + 6 {
                return Err(FrameError::LengthMismatch);
            }

            let control_field = data[4];
            match control_field {
                0x53 => Ok(FrameType::ControlFrame{
                    function: Function::from(data[4]), 
                    address: data[5],
                    control_information: data[6]
                }),
                _ => Ok(FrameType::LongFrame{
                    function: Function::from(data[4]),
                    address: data[5],
                    control_information: data[6],
                    data: &data[7..data.len() - 2],
                }),
            }
        },
        0x10 => {
            validate_checksum(&data[1..])?;
            if data.len() == 5 && data[4] == 0x16 {
                Ok(FrameType::ShortFrame{
                    function: Function::from(data[1]),
                    address: data[2],
                })
            } else {
                Err(FrameError::LengthMismatch)
            }

        },
        _ => Err(FrameError::InvalidStartByte),
    }
}

fn validate_checksum(data: &[u8]) -> Result<(), FrameError> {
    // Assuming the checksum is the second to last byte in the data array.
    let checksum_byte_index = data.len() - 2;
    let checksum_byte = data[checksum_byte_index];

    let calculated_checksum = data[..checksum_byte_index]
        .iter()
        .fold(0, |acc:u8, &x| acc.wrapping_add(x));

    if checksum_byte == calculated_checksum {
        Ok(())
    } else {
        Err(FrameError::WrongChecksum{
            expected: checksum_byte,
            actual: calculated_checksum,
        })
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_frame_type() {

        let single_character_frame = vec![0xE5];
        let short_frame = vec![0x10, 0x7B, 0x8b, 0x06,0x16];
        let control_frame = vec![0x68, 0x03, 0x03, 0x68, 0x53, 0x01, 0x00, 0x54, 0x16];
        
        let example = vec![
            0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01, 
            0x00, 0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B, 
            0x00, 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48, 
            0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11, 
            0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72, 
            0x00, 0x72, 0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
        ];
        


        assert_eq!(parse_frame(&single_character_frame), Ok(FrameType::SingleCharacter{character: 0xE5}));
        assert_eq!(parse_frame(&short_frame), Ok(FrameType::ShortFrame{function: Function::from(0x7B), address: 0x8B}));
        assert_eq!(parse_frame(&control_frame), Ok(FrameType::ControlFrame {
            function: Function::from(0x53),
            address: 0x01,
            control_information: 0x00,
        }));

        assert_eq!(parse_frame(&example),Ok(FrameType::LongFrame {  
            function: Function::from(8), 
            address: 1, 
            control_information: 114, 
            data: &[
                1, 0, 0, 0, 150, 21, 1, 0, 24, 0,
                0, 0, 12, 120, 86, 0, 0, 0, 1, 253, 
                27, 0, 2, 252, 3, 72, 82, 37, 116, 
                68, 13, 34, 252, 3, 72, 82, 37, 116, 
                241, 12, 18, 252, 3, 72, 82, 37, 116, 
                99, 17, 2, 101, 180, 9, 34, 101, 134, 
                9, 18, 101, 183, 9, 1, 114, 0, 114,
                 101, 0, 0, 178, 1, 101, 0, 0, 31] }));
    }

}
