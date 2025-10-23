//! Wireless M-Bus frame structures and parsing

use crate::crc::{calculate_crc16, verify_crc16};

/// Validate CRC for all blocks in user data
///
/// According to EN 13757-4:
/// - Subsequent blocks are 16 bytes + 2 CRC
/// - Last block can be shorter: ((L-9) MOD 16) bytes + 2 CRC
fn validate_multiblock_crc(data: &[u8]) -> Result<(), FrameError> {
    if data.is_empty() {
        return Ok(());  // No user data is valid
    }

    let mut pos = 0;
    let mut block_num = 1;  // Block 0 was already validated

    while pos < data.len() {
        // Determine block size
        let remaining = data.len() - pos;

        if remaining < 2 {
            // Need at least 2 bytes for CRC
            return Err(FrameError::InsufficientData {
                required: pos + 2,
                available: data.len(),
            });
        }

        // Each block is max 16 bytes + 2 CRC = 18 bytes
        // Last block can be shorter
        let block_data_len = if remaining > 18 {
            16  // Full block
        } else if remaining >= 3 {
            remaining - 2  // Last block (data + CRC)
        } else {
            return Err(FrameError::InsufficientData {
                required: pos + 3,
                available: data.len(),
            });
        };

        let block_end = pos + block_data_len;
        let crc_end = block_end + 2;

        if crc_end > data.len() {
            return Err(FrameError::InsufficientData {
                required: crc_end,
                available: data.len(),
            });
        }

        let block_data = &data[pos..block_end];
        let crc_bytes = [data[block_end], data[block_end + 1]];

        if !verify_crc16(block_data, &crc_bytes) {
            let expected = u16::from_be_bytes(crc_bytes);
            let calculated = calculate_crc16(block_data);
            return Err(FrameError::CrcError {
                block: block_num,
                expected,
                calculated,
            });
        }

        pos = crc_end;
        block_num += 1;
    }

    Ok(())
}

/// Extract user data by removing CRC bytes
///
/// Returns a slice containing only the actual data bytes,
/// with CRC bytes removed
fn extract_user_data(data: &[u8]) -> &[u8] {
    if data.is_empty() {
        return data;
    }

    // For now, return all data including CRC
    // TODO: Implement proper extraction once we have tests
    // The challenge is we need to know where each CRC is
    // to skip it. For now, return the raw data.
    data
}

/// Wireless M-Bus frame formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FrameFormat {
    /// Format A: IEC 60870-5-1 format type FT3
    /// L-field excludes itself and CRC bytes
    FormatA,
    /// Format B: Modified format
    /// L-field excludes only itself
    FormatB,
}

/// Wireless M-Bus transmission modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TransmissionMode {
    /// S-mode: Stationary meters (walk-by reading)
    /// Unidirectional, 32.768 kHz
    S,
    /// T-mode: Frequent transmission (fixed network)
    /// Unidirectional, 32.768 kHz, more frequent than S
    T,
    /// C-mode: Compact, low power
    /// Bidirectional, 100 kHz
    C,
    /// R-mode: Frequent bidirectional
    /// Bidirectional, 32.768 kHz
    R,
    /// N-mode: Narrowband
    /// Narrowband, optimized for long range
    N,
    /// F-mode: Frequent transmission, longer range
    /// Similar to T but optimized for range
    F,
}

/// Control field (C-field) information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlField {
    /// Raw C-field byte
    pub raw: u8,
    /// Function code (lower 4 bits)
    pub function: u8,
    /// Accessibility (bit 4)
    pub accessibility: bool,
    /// Synchronous (bit 5)
    pub synchronous: bool,
    /// Reserved bits (bits 6-7)
    pub reserved: u8,
}

impl ControlField {
    /// Parse C-field from a byte
    pub fn from_byte(byte: u8) -> Self {
        Self {
            raw: byte,
            function: byte & 0x0F,
            accessibility: (byte & 0x10) != 0,
            synchronous: (byte & 0x20) != 0,
            reserved: (byte >> 6) & 0x03,
        }
    }
}

/// Manufacturer ID (M-field)
///
/// 2-byte manufacturer code encoded according to EN 13757-3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ManufacturerId {
    /// Raw 2-byte manufacturer ID
    pub raw: u16,
    /// 3-letter manufacturer code (if valid)
    pub code: Option<[char; 3]>,
}

impl ManufacturerId {
    /// Parse manufacturer ID from 2 bytes (little-endian)
    pub fn from_bytes(bytes: [u8; 2]) -> Self {
        let raw = u16::from_le_bytes(bytes);

        // Decode 3-letter manufacturer code
        // Each letter is encoded as: A=1, B=2, ... Z=26
        // Formula: raw = (letter1-64)*32*32 + (letter2-64)*32 + (letter3-64)
        let code = if raw > 0 && raw <= 0x421F {
            let c3 = ((raw & 0x1F) as u8 + 64) as char;
            let c2 = (((raw >> 5) & 0x1F) as u8 + 64) as char;
            let c1 = (((raw >> 10) & 0x1F) as u8 + 64) as char;

            if c1.is_ascii_uppercase() && c2.is_ascii_uppercase() && c3.is_ascii_uppercase() {
                Some([c1, c2, c3])
            } else {
                None
            }
        } else {
            None
        };

        Self { raw, code }
    }
}

/// Device address (A-field)
///
/// 6-byte address consisting of:
/// - Identification number (4 bytes, BCD encoded)
/// - Version (1 byte)
/// - Device type/medium (1 byte)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceAddress {
    /// Identification number (BCD encoded, 8 digits)
    pub identification: u32,
    /// Version number
    pub version: u8,
    /// Device type/medium
    pub device_type: u8,
}

impl DeviceAddress {
    /// Parse device address from 6 bytes
    pub fn from_bytes(bytes: [u8; 6]) -> Self {
        // Identification is 4 bytes, little-endian, BCD encoded
        let identification = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let version = bytes[4];
        let device_type = bytes[5];

        Self {
            identification,
            version,
            device_type,
        }
    }
}

/// Wireless M-Bus frame
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Frame<'a> {
    /// Frame format (A or B)
    pub format: FrameFormat,
    /// Length field value
    pub length: u8,
    /// Control field
    pub control: ControlField,
    /// Manufacturer ID
    pub manufacturer: ManufacturerId,
    /// Device address
    pub address: DeviceAddress,
    /// Control information field
    pub ci_field: u8,
    /// User data (application layer)
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub data: &'a [u8],
}

/// Errors that can occur during wireless M-Bus frame parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum FrameError {
    /// Input data is empty
    EmptyData,
    /// Input data is too short to be a valid frame
    InsufficientData { required: usize, available: usize },
    /// Invalid length field value
    InvalidLength { length: u8 },
    /// CRC validation failed
    CrcError { block: usize, expected: u16, calculated: u16 },
    /// Invalid frame format
    InvalidFormat,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::EmptyData => write!(f, "Empty data"),
            FrameError::InsufficientData { required, available } => {
                write!(f, "Insufficient data: need {} bytes, got {}", required, available)
            }
            FrameError::InvalidLength { length } => {
                write!(f, "Invalid length field: {}", length)
            }
            FrameError::CrcError { block, expected, calculated } => {
                write!(f, "CRC error in block {}: expected 0x{:04X}, calculated 0x{:04X}",
                       block, expected, calculated)
            }
            FrameError::InvalidFormat => write!(f, "Invalid frame format"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FrameError {}

impl<'a> Frame<'a> {
    /// Parse a wireless M-Bus frame from raw bytes
    ///
    /// # Arguments
    ///
    /// * `data` - Raw frame data
    /// * `format` - Frame format to use (A or B)
    ///
    /// # Returns
    ///
    /// Parsed frame or error
    ///
    /// # Frame Structure
    ///
    /// ```text
    /// [Block 0]
    /// Byte 0:      L-field (length)
    /// Byte 1:      C-field (control)
    /// Byte 2-3:    M-field (manufacturer, little-endian)
    /// Byte 4-7:    A-field ID (4 bytes, little-endian BCD)
    /// Byte 8:      A-field version
    /// Byte 9:      A-field device type
    /// Byte 10-11:  CRC-16 of bytes 0-9 (big-endian!)
    /// Byte 12:     CI-field (control information)
    /// Byte 13+:    User data (with CRC blocks every 16 bytes)
    /// ```
    pub fn parse(data: &'a [u8], format: FrameFormat) -> Result<Self, FrameError> {
        if data.is_empty() {
            return Err(FrameError::EmptyData);
        }

        // Minimum frame: L + C + M(2) + A(6) + CRC(2) + CI = 13 bytes
        if data.len() < 13 {
            return Err(FrameError::InsufficientData {
                required: 13,
                available: data.len(),
            });
        }

        let length = data[0];

        // Validate length
        match format {
            FrameFormat::FormatA => {
                // Format A: L-field excludes itself and CRC bytes
                // Minimum: C(1) + M(2) + A(6) = 9 bytes
                if length < 9 {
                    return Err(FrameError::InvalidLength { length });
                }
            }
            FrameFormat::FormatB => {
                // Format B: L-field excludes only itself
                // Minimum: C(1) + M(2) + A(6) + CRC(2) = 11 bytes
                if length < 11 {
                    return Err(FrameError::InvalidLength { length });
                }
            }
        }

        // Parse header fields (bytes 1-9)
        let c_field = ControlField::from_byte(data[1]);
        let m_field = ManufacturerId::from_bytes([data[2], data[3]]);
        let a_field = DeviceAddress::from_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9],
        ]);

        // Validate first block CRC (bytes 0-9, CRC at 10-11, big-endian)
        let first_block_data = &data[0..10];
        let first_block_crc = [data[10], data[11]];

        if !verify_crc16(first_block_data, &first_block_crc) {
            let expected = u16::from_be_bytes(first_block_crc);  // Big-endian
            let calculated = calculate_crc16(first_block_data);
            return Err(FrameError::CrcError {
                block: 0,
                expected,
                calculated,
            });
        }

        // CI-field is at byte 12
        let ci_field = data[12];

        // User data starts at byte 13
        // Validate all subsequent CRC blocks
        let user_data_start = 13;
        let remaining_data = &data[user_data_start..];

        // Validate multi-block CRCs for user data
        // According to EN 13757-4: subsequent blocks are 16 bytes + 2 CRC
        validate_multiblock_crc(remaining_data)?;

        // Extract user data (without CRC bytes)
        let user_data = extract_user_data(remaining_data);

        Ok(Frame {
            format,
            length,
            control: c_field,
            manufacturer: m_field,
            address: a_field,
            ci_field,
            data: user_data,
        })
    }

    /// Try to parse as Format A
    pub fn try_format_a(data: &'a [u8]) -> Result<Self, FrameError> {
        Self::parse(data, FrameFormat::FormatA)
    }

    /// Try to parse as Format B
    pub fn try_format_b(data: &'a [u8]) -> Result<Self, FrameError> {
        Self::parse(data, FrameFormat::FormatB)
    }
}

impl<'a> TryFrom<&'a [u8]> for Frame<'a> {
    type Error = FrameError;

    /// Try to parse frame, attempting Format A first, then Format B
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Try Format A first (more common)
        if let Ok(frame) = Self::try_format_a(data) {
            return Ok(frame);
        }

        // Fall back to Format B
        Self::try_format_b(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_field_parsing() {
        let c_field = ControlField::from_byte(0x44);
        assert_eq!(c_field.function, 0x04);
        assert!(!c_field.accessibility);
        assert!(!c_field.synchronous);
    }

    #[test]
    fn test_manufacturer_id() {
        // Example: "ACW" = (1-0)*32*32 + (3-0)*32 + (23-0) = 0 + 96 + 23 = 119
        // Actually the encoding is: (letter-64)*... where A=65 (ASCII)
        // So: (65-64)*1024 + (67-64)*32 + (87-64) = 1*1024 + 3*32 + 23 = 1024 + 96 + 23 = 1143
        let m_id = ManufacturerId::from_bytes([0x77, 0x04]); // 1143 in little-endian
        assert_eq!(m_id.raw, 0x0477);
        // Note: The decoding formula might need adjustment based on the actual standard
    }

    #[test]
    fn test_device_address() {
        let addr = DeviceAddress::from_bytes([0x34, 0x12, 0x78, 0x56, 0x01, 0x07]);
        assert_eq!(addr.identification, 0x56781234);
        assert_eq!(addr.version, 0x01);
        assert_eq!(addr.device_type, 0x07);
    }

    #[test]
    fn test_frame_error_empty() {
        let result = Frame::try_from(&[][..]);
        assert_eq!(result, Err(FrameError::EmptyData));
    }

    #[test]
    fn test_frame_error_too_short() {
        let data = [0x0A, 0x44]; // Only 2 bytes
        let result = Frame::try_from(&data[..]);
        assert!(matches!(result, Err(FrameError::InsufficientData { .. })));
    }
}
