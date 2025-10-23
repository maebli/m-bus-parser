//! CRC-16 implementation for Wireless M-Bus (EN 13757-4)
//!
//! The CRC-16/EN-13757 algorithm uses:
//! - Polynomial: 0x3D65 (x^16+x^13+x^12+x^11+x^10+x^8+x^6+x^5+x^2+1)
//! - Init: 0x0000
//! - RefIn: false (no bit reflection on input)
//! - RefOut: false (no bit reflection on output)
//! - XorOut: 0xFFFF
//! - Check value: 0xC2B7 (for "123456789")

/// CRC-16 polynomial for Wireless M-Bus (EN 13757-4)
const CRC16_POLY: u16 = 0x3D65;

/// Calculate CRC-16 for Wireless M-Bus
///
/// This implements the CRC-16/EN-13757 algorithm as specified in EN 13757-4.
///
/// # Arguments
///
/// * `data` - The data bytes to calculate CRC over
///
/// # Returns
///
/// The calculated CRC-16 value (already XORed with 0xFFFF)
///
/// # Example
///
/// ```
/// use m_bus_wireless_frame::calculate_crc16;
///
/// let data = b"123456789";
/// let crc = calculate_crc16(data);
/// assert_eq!(crc, 0xC2B7); // Expected check value
/// ```
pub fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000; // Init value

    for &byte in data {
        crc ^= (byte as u16) << 8;

        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ CRC16_POLY;
            } else {
                crc <<= 1;
            }
        }
    }

    crc ^ 0xFFFF // XorOut
}

/// Verify CRC-16 for a block of data
///
/// # Arguments
///
/// * `data` - The data bytes (without CRC)
/// * `expected_crc` - The expected CRC value (2 bytes, **big-endian** - MSB first)
///
/// # Returns
///
/// `true` if the CRC matches, `false` otherwise
///
/// # Note
///
/// Wireless M-Bus stores CRC in big-endian format (MSB, LSB),
/// unlike wired M-Bus which uses little-endian.
pub fn verify_crc16(data: &[u8], expected_crc: &[u8; 2]) -> bool {
    let calculated = calculate_crc16(data);
    let expected = u16::from_be_bytes(*expected_crc);  // Big-endian!
    calculated == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_check_value() {
        // Standard check value for "123456789"
        let data = b"123456789";
        let crc = calculate_crc16(data);
        assert_eq!(crc, 0xC2B7, "CRC-16/EN-13757 check value should be 0xC2B7");
    }

    #[test]
    fn test_crc16_empty() {
        let data = b"";
        let crc = calculate_crc16(data);
        assert_eq!(crc, 0xFFFF, "CRC of empty data should be 0xFFFF (init XOR xorout)");
    }

    #[test]
    fn test_verify_crc16() {
        let data = b"123456789";
        let crc_bytes = 0xC2B7u16.to_be_bytes();  // Big-endian
        assert!(verify_crc16(data, &crc_bytes));
    }

    #[test]
    fn test_verify_crc16_invalid() {
        let data = b"123456789";
        let wrong_crc = [0x00, 0x00];
        assert!(!verify_crc16(data, &wrong_crc));
    }
}
