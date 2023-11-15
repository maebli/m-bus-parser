#[derive(Debug,PartialEq)]
pub enum FrameType<'a> {
    SingleCharacter{
        character: u8
    },
    ShortFrame{
        function: u8, 
        address: u8
    },
    LongFrame{
        function: u8,
        address: u8,
        control_information: u8, 
        data: &'a [u8]},
    ControlFrame{
        function: u8, 
        address: u8, 
        control_information: u8
    },
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
                    function: data[4], 
                    address: data[5],
                    control_information: data[6]
                }),
                _ => Ok(FrameType::LongFrame{
                    function: data[4],
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
                    function: data[1],
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


        assert_eq!(parse_frame(&single_character_frame), Ok(FrameType::SingleCharacter{character: 0xE5}));
        assert_eq!(parse_frame(&short_frame), Ok(FrameType::ShortFrame{function: 0x7B, address: 0x8B}));
        assert_eq!(parse_frame(&control_frame), Ok(FrameType::ControlFrame {
            function: 0x53,
            address: 0x01,
            control_information: 0x00,
        }));
    }

}
