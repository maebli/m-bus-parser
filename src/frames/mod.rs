#[derive(Debug,PartialEq)]
pub enum FrameType {
    SingleCharacter,
    ShortFrame,
    LongFrame,
    ControlFrame,
    ExtendedLinkLayer,
    Unknown,
}
#[derive(Debug, PartialEq)]
pub enum FrameDetectionError {
    EmptyData,
    InvalidStartOrStopByte,
    LengthMismatch,
    WrongChecksum,
}

pub fn detect_frame_type(data: &[u8])  -> Result<FrameType, FrameDetectionError> {
   
    if data.is_empty() {
        return Ok(FrameType::Unknown);
    }

    if data.len() == 1 && data[0] == 0xE5 {
        return Ok(FrameType::SingleCharacter);
    }
    
    let cs = calculate_checksum(data);

    if cs != data[data.len() - 2] {
        return Err(FrameDetectionError::WrongChecksum);
    }

    match data[0] {
        0x68 => {
            // Check for minimum length for a valid long/control frame
            if data.len() < 6 || data[data.len() - 1] != 0x16 {
                return Ok(FrameType::Unknown);
            }

            let length = data[1] as usize;

            // Length bytes should match and the total size should be length + 6
            if data[1] != data[2] || data.len() != length + 6 {
                return Err(FrameDetectionError::LengthMismatch);
            }

            // Additional checks based on control field to distinguish between ControlFrame and LongFrame
            let control_field = data[4];
            match control_field {
                // Define specific control field values that indicate a ControlFrame
                // Example: 0x53 might indicate a ControlFrame (adjust as per your protocol specs)
                0x53 => Ok(FrameType::ControlFrame),

                // Otherwise, assume it's a LongFrame
                _ => Ok(FrameType::LongFrame),
            }
        },
        0x10 | 0x40 => {
            if data.len() == 4 && data[3] == 0x16 {
                Ok(FrameType::ShortFrame)
            } else {
                Ok(FrameType::Unknown)
            }
        },
        _ => Ok(FrameType::Unknown)
    }
}


// The Check Sum is calculated from the arithmetical sum of the data 
// without taking carry digits into account.
fn calculate_checksum(data: &[u8]) -> u8 {

    if data.len() < 2 {
        panic!("Data too short to calculate checksum");
    }

    data.iter()
        .take(data.len() - 2) // Take all bytes except the last two
        .fold(0u8, |acc, &x| acc.wrapping_add(x))
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_frame_type() {

        let single_character_frame = vec![0xE5];
        let short_frame = vec![0x10, 0x7B, 0x8b, 0x16];
        let long_frame = vec![0x68, 0x03, 0x03, 0x68, 0x08, 0x34, 0x12, 0x24, 0x16];
        let control_frame = vec![0x68, 0x01, 0x01, 0x68, 0x53, 0x25, 0x16]; 
        let control_frame_wrong_length= vec![0x68, 0x02, 0x02, 0x68, 0x53, 0x27, 0x16]; 

        assert_eq!(detect_frame_type(&single_character_frame), Ok(FrameType::SingleCharacter));
        assert_eq!(detect_frame_type(&short_frame), Ok(FrameType::ShortFrame));
        assert_eq!(detect_frame_type(&long_frame), Ok(FrameType::LongFrame));
        assert_eq!(detect_frame_type(&control_frame), Ok(FrameType::ControlFrame));
        assert_eq!(detect_frame_type(&control_frame_wrong_length),Err(FrameDetectionError::LengthMismatch));

    }
}
