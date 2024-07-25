//! is part of the MBUS data link layer
//! It is used to encapsulate the application layer data
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Frame<'a> {
    SingleCharacter {
        character: u8,
    },
    ShortFrame {
        function: Function,
        address: Address,
    },
    LongFrame {
        function: Function,
        address: Address,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        data: &'a [u8],
    },
    ControlFrame {
        function: Function,
        address: Address,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        data: &'a [u8],
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Function {
    SndNk,
    SndUd { fcb: bool },
    ReqUd2 { fcb: bool },
    ReqUd1 { fcb: bool },
    RspUd { acd: bool, dfc: bool },
}

#[cfg(feature = "std")]
impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::SndNk => write!(f, "SndNk"),
            Function::SndUd { fcb } => write!(f, "SndUd (FCB: {})", fcb),
            Function::ReqUd2 { fcb } => write!(f, "ReqUd2 (FCB: {})", fcb),
            Function::ReqUd1 { fcb } => write!(f, "ReqUd1 (FCB: {})", fcb),
            Function::RspUd { acd, dfc } => write!(f, "RspUd (ACD: {}, DFC: {})", acd, dfc),
        }
    }
}

impl TryFrom<u8> for Function {
    type Error = FrameError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x40 => Ok(Self::SndNk),
            0x53 => Ok(Self::SndUd { fcb: false }),
            0x73 => Ok(Self::SndUd { fcb: true }),
            0x5B => Ok(Self::ReqUd2 { fcb: false }),
            0x7B => Ok(Self::ReqUd2 { fcb: true }),
            0x5A => Ok(Self::ReqUd1 { fcb: false }),
            0x7A => Ok(Self::ReqUd1 { fcb: true }),
            0x08 => Ok(Self::RspUd {
                acd: false,
                dfc: false,
            }),
            0x18 => Ok(Self::RspUd {
                acd: false,
                dfc: true,
            }),
            0x28 => Ok(Self::RspUd {
                acd: true,
                dfc: false,
            }),
            0x38 => Ok(Self::RspUd {
                acd: true,
                dfc: true,
            }),
            _ => Err(FrameError::InvalidFunction { byte }),
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Address {
    Uninitalized,
    Primary(u8),
    Secondary,
    Broadcast { reply_required: bool },
}

#[cfg(feature = "std")]
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Uninitalized => write!(f, "Uninitalized"),
            Address::Primary(byte) => write!(f, "Primary ({})", byte),
            Address::Secondary => write!(f, "Secondary"),
            Address::Broadcast { reply_required } => {
                write!(f, "Broadcast (Reply Required: {})", reply_required)
            }
        }
    }
}

impl Address {
    const fn from(byte: u8) -> Self {
        match byte {
            0 => Self::Uninitalized,
            253 => Self::Secondary,
            254 => Self::Broadcast {
                reply_required: true,
            },
            255 => Self::Broadcast {
                reply_required: false,
            },
            _ => Self::Primary(byte),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FrameError {
    EmptyData,
    InvalidStartByte,
    InvalidStopByte,
    WrongLengthIndication,
    LengthShort,
    LengthShorterThanSix { length: usize },
    WrongChecksum { expected: u8, actual: u8 },
    InvalidControlInformation { byte: u8 },
    InvalidFunction { byte: u8 },
}

impl<'a> TryFrom<&'a [u8]> for Frame<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, FrameError> {
        let first_byte = *data.first().ok_or(FrameError::EmptyData)?;

        if first_byte == 0xE5 {
            return Ok(Frame::SingleCharacter { character: 0xE5 });
        }

        let second_byte = *data.get(1).ok_or(FrameError::LengthShort)?;
        let third_byte = *data.get(2).ok_or(FrameError::LengthShort)?;

        match first_byte {
            0x68 => {
                validate_checksum(data.get(4..).ok_or(FrameError::LengthShort)?)?;

                let length = *data.get(1).ok_or(FrameError::LengthShort)? as usize;

                if second_byte != third_byte || data.len() != length + 6 {
                    return Err(FrameError::WrongLengthIndication);
                }

                if *data.last().ok_or(FrameError::LengthShort)? != 0x16 {
                    return Err(FrameError::InvalidStopByte);
                }

                let control_field = *data.get(4).ok_or(FrameError::LengthShort)?;

                let address_field = *data.get(5).ok_or(FrameError::LengthShort)?;
                match control_field {
                    0x53 => Ok(Frame::ControlFrame {
                        function: Function::try_from(control_field)?,
                        address: Address::from(address_field),
                        data: data.get(6..data.len() - 2).ok_or(FrameError::LengthShort)?,
                    }),
                    _ => Ok(Frame::LongFrame {
                        function: Function::try_from(control_field)?,
                        address: Address::from(address_field),
                        data: data.get(6..data.len() - 2).ok_or(FrameError::LengthShort)?,
                    }),
                }
            }
            0x10 => {
                validate_checksum(data.get(1..).ok_or(FrameError::LengthShort)?)?;
                if data.len() == 5 && *data.last().ok_or(FrameError::InvalidStopByte)? == 0x16 {
                    Ok(Frame::ShortFrame {
                        function: Function::try_from(second_byte)?,
                        address: Address::from(third_byte),
                    })
                } else {
                    Err(FrameError::LengthShort)
                }
            }
            _ => Err(FrameError::InvalidStartByte),
        }
    }
}

fn validate_checksum(data: &[u8]) -> Result<(), FrameError> {
    // Assuming the checksum is the second to last byte in the data array.
    let checksum_byte_index = data.len() - 2;
    let checksum_byte = *data
        .get(checksum_byte_index)
        .ok_or(FrameError::LengthShort)?;

    let calculated_checksum = data
        .get(..checksum_byte_index)
        .ok_or(FrameError::LengthShort)?
        .iter()
        .fold(0, |acc: u8, &x| acc.wrapping_add(x));

    if checksum_byte == calculated_checksum {
        Ok(())
    } else {
        Err(FrameError::WrongChecksum {
            expected: checksum_byte,
            actual: calculated_checksum,
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FrameError {}

#[cfg(feature = "std")]
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::EmptyData => write!(f, "Data is empty"),
            FrameError::InvalidStartByte => write!(f, "Invalid start byte"),
            FrameError::InvalidStopByte => write!(f, "Invalid stop byte"),
            FrameError::LengthShort => write!(f, "Length mismatch"),
            FrameError::LengthShorterThanSix { length } => {
                write!(f, "Length is shorter than six: {}", length)
            }
            FrameError::WrongChecksum { expected, actual } => write!(
                f,
                "Wrong checksum, expected: {}, actual: {}",
                expected, actual
            ),
            FrameError::InvalidControlInformation { byte } => {
                write!(f, "Invalid control information: {}", byte)
            }
            FrameError::InvalidFunction { byte } => write!(f, "Invalid function: {}", byte),
            FrameError::WrongLengthIndication => write!(f, "Wrong length indication"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_frame_type() {
        let single_character_frame: &[u8] = &[0xE5];
        let short_frame: &[u8] = &[0x10, 0x7B, 0x8b, 0x06, 0x16];
        let control_frame: &[u8] = &[0x68, 0x03, 0x03, 0x68, 0x53, 0x01, 0x51, 0xA5, 0x16];

        let example: &[u8] = &[
            0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01,
            0x00, 0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B,
            0x00, 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48,
            0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11,
            0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72,
            0x00, 0x72, 0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
        ];

        assert_eq!(
            Frame::try_from(single_character_frame),
            Ok(Frame::SingleCharacter { character: 0xE5 })
        );
        assert_eq!(
            Frame::try_from(short_frame),
            Ok(Frame::ShortFrame {
                function: Function::try_from(0x7B).unwrap(),
                address: Address::from(0x8B)
            })
        );
        assert_eq!(
            Frame::try_from(control_frame),
            Ok(Frame::ControlFrame {
                function: Function::try_from(0x53).unwrap(),
                address: Address::from(0x01),
                data: &[0x51]
            })
        );

        assert_eq!(
            Frame::try_from(example),
            Ok(Frame::LongFrame {
                function: Function::try_from(8).unwrap(),
                address: Address::from(1),
                data: &[
                    114, 1, 0, 0, 0, 150, 21, 1, 0, 24, 0, 0, 0, 12, 120, 86, 0, 0, 0, 1, 253, 27,
                    0, 2, 252, 3, 72, 82, 37, 116, 68, 13, 34, 252, 3, 72, 82, 37, 116, 241, 12,
                    18, 252, 3, 72, 82, 37, 116, 99, 17, 2, 101, 180, 9, 34, 101, 134, 9, 18, 101,
                    183, 9, 1, 114, 0, 114, 101, 0, 0, 178, 1, 101, 0, 0, 31
                ]
            })
        );
    }
}
