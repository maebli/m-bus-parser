use crate::ApplicationLayerError;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtendedLinkLayer {
    pub communication_control: u8,
    pub access_number: u8,
    pub receiver_address: Option<ReceiverAddress>,
    pub encryption: Option<EncryptionFields>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReceiverAddress {
    pub manufacturer: u16,
    pub address: [u8; 6],
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EncryptionFields {
    pub session_number: [u8; 4],
    pub payload_crc: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EllFormat {
    /// Extended Link Layer I (2 bytes: CC, ACC)
    FormatI,
    /// Extended Link Layer II (8 bytes: CC, ACC, SN[4], CRC[2])
    FormatII,
    /// Extended Link Layer III (16 bytes: CC, ACC, MFR[2], ADDR[6], SN[4], CRC[2])
    FormatIII,
}

impl ExtendedLinkLayer {
    /// Parse Extended Link Layer from data, returns (ELL, bytes_consumed)
    pub fn parse(data: &[u8], format: EllFormat) -> Result<(Self, usize), ApplicationLayerError> {
        match format {
            EllFormat::FormatI => Self::parse_format_i(data),
            EllFormat::FormatII => Self::parse_format_ii(data),
            EllFormat::FormatIII => Self::parse_format_iii(data),
        }
    }

    fn parse_format_i(data: &[u8]) -> Result<(Self, usize), ApplicationLayerError> {
        if data.len() < 2 {
            return Err(ApplicationLayerError::InsufficientData);
        }

        Ok((
            ExtendedLinkLayer {
                communication_control: data[0],
                access_number: data[1],
                receiver_address: None,
                encryption: None,
            },
            2,
        ))
    }

    fn parse_format_ii(data: &[u8]) -> Result<(Self, usize), ApplicationLayerError> {
        if data.len() < 8 {
            return Err(ApplicationLayerError::InsufficientData);
        }

        Ok((
            ExtendedLinkLayer {
                communication_control: data[0],
                access_number: data[1],
                receiver_address: None,
                encryption: Some(EncryptionFields {
                    session_number: [data[2], data[3], data[4], data[5]],
                    payload_crc: u16::from_le_bytes([data[6], data[7]]),
                }),
            },
            8,
        ))
    }

    fn parse_format_iii(data: &[u8]) -> Result<(Self, usize), ApplicationLayerError> {
        if data.len() < 16 {
            return Err(ApplicationLayerError::InsufficientData);
        }

        Ok((
            ExtendedLinkLayer {
                communication_control: data[0],
                access_number: data[1],
                receiver_address: Some(ReceiverAddress {
                    manufacturer: u16::from_le_bytes([data[2], data[3]]),
                    address: [data[4], data[5], data[6], data[7], data[8], data[9]],
                }),
                encryption: Some(EncryptionFields {
                    session_number: [data[10], data[11], data[12], data[13]],
                    payload_crc: u16::from_le_bytes([data[14], data[15]]),
                }),
            },
            16,
        ))
    }
}
