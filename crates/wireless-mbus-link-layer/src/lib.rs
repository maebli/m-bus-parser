use m_bus_core::{DeviceType, Function, IdentificationNumber, ManufacturerCode};

/// CRC-16/EN13757 used in wireless M-Bus Format A frames.
/// Polynomial: 0x3D65, Init: 0x0000, XorOut: 0xFFFF, RefIn: false, RefOut: false.
fn crc16_en13757(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x3D65;
            } else {
                crc <<= 1;
            }
        }
    }
    crc ^ 0xFFFF
}

/// Strip Format A CRCs from a wireless M-Bus frame.
///
/// Format A frames have CRC-16 checksums embedded in the data:
/// - Block 1: first 10 bytes (L, C, M, M, ID, ID, ID, ID, Ver, Type) + 2 CRC bytes
/// - Block 2+: up to 16 bytes of data + 2 CRC bytes each
///
/// Writes the stripped frame into `output` and returns the resulting slice,
/// or `None` if the frame doesn't have valid Format A CRCs.
/// The L-field is corrected to reflect the stripped payload size.
pub fn strip_format_a_crcs<'a>(data: &[u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    if data.len() < 12 || output.len() < data.len() {
        return None;
    }

    // Check block 1: first 10 bytes + 2 CRC
    if crc16_en13757(&data[0..10]) != u16::from_be_bytes([data[10], data[11]]) {
        return None;
    }

    let mut out_pos = 10;
    output[..10].copy_from_slice(&data[..10]);

    let mut pos = 12;
    while pos < data.len() {
        let remaining = data.len() - pos;
        if remaining < 3 {
            output[out_pos..out_pos + remaining].copy_from_slice(&data[pos..pos + remaining]);
            out_pos += remaining;
            break;
        }

        let max_data_len = 16.min(remaining - 2);
        let mut found = false;

        for data_len in (1..=max_data_len).rev() {
            let crc_start = pos + data_len;
            if crc_start + 2 > data.len() {
                continue;
            }
            if crc16_en13757(&data[pos..crc_start])
                == u16::from_be_bytes([data[crc_start], data[crc_start + 1]])
            {
                output[out_pos..out_pos + data_len].copy_from_slice(&data[pos..crc_start]);
                out_pos += data_len;
                pos = crc_start + 2;
                found = true;
                break;
            }
        }

        if !found {
            let remaining = data.len() - pos;
            output[out_pos..out_pos + remaining].copy_from_slice(&data[pos..]);
            out_pos += remaining;
            break;
        }
    }

    output[0] = (out_pos - 1) as u8;
    Some(&output[..out_pos])
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WirelessFrame<'a> {
    pub function: Function,
    pub manufacturer_id: ManufacturerId,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ManufacturerId {
    pub manufacturer_code: ManufacturerCode,
    pub identification_number: IdentificationNumber,
    pub device_type: DeviceType,
    pub version: u8,
    pub is_unique_globally: bool,
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
            .map_err(|_| FrameError::TooShort)?,
            identification_number: IdentificationNumber::from_bcd_hex_digits([
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
                *iter.next().ok_or(FrameError::TooShort)?,
            ])
            .map_err(|_| FrameError::TooShort)?,
            version: *iter.next().ok_or(FrameError::TooShort)?,
            // In wireless M-Bus, device type encoding depends on the CI (Control Information) field:
            // - For unencrypted frames (CI=0x7A): use full device type byte
            // - For encrypted frames (CI=0xA0-0xAF): device type is in upper nibble,
            //   lower nibble contains encryption mode information
            device_type: {
                let device_byte = *iter.next().ok_or(FrameError::TooShort)?;
                // Peek ahead at the CI field (at offset 8 from start of ManufacturerId data)
                let ci_byte = *data.get(8).ok_or(FrameError::TooShort)?;
                let device_type_code = if (0xA0..=0xAF).contains(&ci_byte) {
                    // Encrypted frame: extract upper nibble only
                    (device_byte >> 4) & 0x0F
                } else {
                    // Unencrypted frame: use full byte
                    device_byte
                };
                DeviceType::from(device_type_code)
            },
            is_unique_globally: false, /*todo not sure about this field*/
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FrameError {
    EmptyData,
    TooShort,
    WrongLength { expected: usize, actual: usize },
}

impl<'a> TryFrom<&'a [u8]> for WirelessFrame<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, FrameError> {
        let length = data.len();
        let length_byte = *data.first().ok_or(FrameError::EmptyData)? as usize;
        let _c_field = *data.get(1).ok_or(FrameError::TooShort)? as usize;
        let manufacturer_id = ManufacturerId::try_from(&data[2..])?;

        // In wireless M-Bus, the L-field contains the number of bytes following the L-field
        if length_byte + 1 == length {
            return Ok(WirelessFrame {
                function: Function::SndNk { prm: false },
                manufacturer_id,
                data: &data[10..],
            });
        }

        Err(FrameError::WrongLength {
            expected: length_byte + 1,
            actual: data.len(),
        })
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
        let parsed = WirelessFrame::try_from(frame);
        println!("{:#?}", parsed);
    }
}
