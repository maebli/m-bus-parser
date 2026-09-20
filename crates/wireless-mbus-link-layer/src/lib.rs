#![cfg_attr(not(any(feature = "std", test)), no_std)]

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

/// Return the start offset of a trailing frame CRC when the final two bytes
/// validate against every preceding byte.
pub fn trailing_frame_crc_start(data: &[u8]) -> Option<usize> {
    let crc_start = data.len().checked_sub(2)?;
    if crc_start < 10 {
        return None;
    }

    let expected = u16::from_be_bytes([data[crc_start], data[crc_start + 1]]);
    (crc16_en13757(&data[..crc_start]) == expected).then_some(crc_start)
}

fn validate_format_a_header(data: &[u8]) -> Option<()> {
    let header = data.get(..10)?;
    let crc = data.get(10..12)?;
    (crc16_en13757(header) == u16::from_be_bytes([crc[0], crc[1]])).then_some(())
}

/// A borrowed view of a Format A frame with its interleaved CRCs omitted.
///
/// The source is never rewritten or copied. [`Self::bytes`] yields the corrected
/// length byte followed by the original non-CRC bytes. As with
/// [`strip_format_a_crcs`], a valid first-block CRC is required; an unrecognized
/// trailing block is retained verbatim for compatibility.
///
/// Construction scans CRC boundaries to determine the corrected length. No
/// destination buffer is needed to subsequently consume the bytes:
///
/// ```
/// use wireless_mbus_link_layer::FormatAFrame;
/// let raw = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];
/// let view = FormatAFrame::new(&raw).unwrap();
/// let mut bytes = view.bytes();
/// assert_eq!(bytes.next(), Some(9));
/// assert_eq!(bytes.count(), 9);
/// ```
#[derive(Clone, Debug)]
pub struct FormatAFrame<'a> {
    data: &'a [u8],
    length: usize,
}

impl<'a> FormatAFrame<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Option<Self> {
        validate_format_a_header(data)?;
        let length = FormatAChunks {
            remaining: &data[12..],
        }
        .fold(10, |length, chunk| length + chunk.len());
        Some(Self { data, length })
    }

    /// Number of bytes after removing the recognized CRCs.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// A Format A frame always contains its ten-byte link header.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Iterates over normalized bytes without a destination buffer.
    #[must_use]
    pub fn bytes(&self) -> FormatABytes<'a> {
        FormatABytes {
            length_byte: Some((self.length - 1) as u8),
            current: self.data[1..10].iter(),
            chunks: FormatAChunks {
                remaining: &self.data[12..],
            },
            remaining: self.length,
        }
    }
}

#[derive(Clone, Debug)]
struct FormatAChunks<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for FormatAChunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let data = self.remaining;
        if data.is_empty() {
            return None;
        }
        if data.len() >= 3 {
            for length in (1..=16.min(data.len() - 2)).rev() {
                let crc = u16::from_be_bytes([data[length], data[length + 1]]);
                if crc16_en13757(&data[..length]) == crc {
                    self.remaining = &data[length + 2..];
                    return Some(&data[..length]);
                }
            }
        }
        self.remaining = &[];
        Some(data)
    }
}

/// Cloneable iterator over a borrowed Format A frame's normalized bytes.
///
/// Normalized data can be consumed incrementally. APIs accepting a contiguous
/// slice still need caller-provided storage, filled with [`strip_format_a_crcs`].
#[derive(Clone, Debug)]
pub struct FormatABytes<'a> {
    length_byte: Option<u8>,
    current: core::slice::Iter<'a, u8>,
    chunks: FormatAChunks<'a>,
    remaining: usize,
}

impl Iterator for FormatABytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let byte = if let Some(length) = self.length_byte.take() {
            length
        } else {
            loop {
                if let Some(byte) = self.current.next() {
                    break *byte;
                }
                self.current = self.chunks.next()?.iter();
            }
        };
        self.remaining -= 1;
        Some(byte)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for FormatABytes<'_> {}
impl core::iter::FusedIterator for FormatABytes<'_> {}

/// Strip Format A CRCs into caller-provided contiguous storage.
///
/// Use [`FormatAFrame::bytes`] to consume normalized bytes lazily instead.
pub fn strip_format_a_crcs<'a>(data: &[u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    // Preserve the historical destination-size requirement.
    if output.len() < data.len() {
        return None;
    }
    validate_format_a_header(data)?;
    output[..10].copy_from_slice(&data[..10]);
    let mut length = 10;
    for chunk in (FormatAChunks {
        remaining: &data[12..],
    }) {
        output[length..length + chunk.len()].copy_from_slice(chunk);
        length += chunk.len();
    }
    output[0] = (length - 1) as u8;
    Some(&output[..length])
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WirelessFrame<'a> {
    /// Raw wireless M-Bus C-field.
    pub control_field: u8,
    /// Decoded C-field when it maps to a function currently known by the
    /// shared M-Bus core. Unknown but otherwise valid C-fields are preserved
    /// through `control_field` instead of making the frame unparseable.
    pub function: Option<Function>,
    pub manufacturer_id: ManufacturerId,
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "m_bus_core::serde_hex::serialize")
    )]
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ManufacturerId {
    pub manufacturer_code: ManufacturerCode,
    pub identification_number: IdentificationNumber,
    pub device_type: DeviceType,
    pub version: u8,
    /// Bit 15 of the manufacturer field, which sits outside the three-letter
    /// code: meters that set it flag the address as not globally unique.
    pub is_unique_globally: bool,
}

impl TryFrom<&[u8]> for ManufacturerId {
    type Error = FrameError;
    fn try_from(data: &[u8]) -> Result<Self, FrameError> {
        let mut iter = data.iter();
        let manufacturer_field = u16::from_le_bytes([
            *iter.next().ok_or(FrameError::TooShort)?,
            *iter.next().ok_or(FrameError::TooShort)?,
        ]);
        Ok(ManufacturerId {
            manufacturer_code: ManufacturerCode::from_id(manufacturer_field).map_err(|_| {
                FrameError::InvalidManufacturerCode {
                    code: manufacturer_field,
                }
            })?,
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
            is_unique_globally: (manufacturer_field & !ManufacturerCode::CODE_MASK) == 0,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FrameError {
    EmptyData,
    TooShort,
    /// The manufacturer field's low 15 bits are not three uppercase letters.
    InvalidManufacturerCode {
        code: u16,
    },
    WrongLength {
        expected: usize,
        actual: usize,
    },
}

impl<'a> TryFrom<&'a [u8]> for WirelessFrame<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, FrameError> {
        let length = data.len();
        let length_byte = *data.first().ok_or(FrameError::EmptyData)? as usize;
        let control_field = *data.get(1).ok_or(FrameError::TooShort)?;
        let manufacturer_id = ManufacturerId::try_from(&data[2..])?;

        // In wireless M-Bus, the L-field contains the number of bytes following the L-field
        if length_byte + 1 == length {
            let data_end = trailing_frame_crc_start(data).unwrap_or(length);
            return Ok(WirelessFrame {
                control_field,
                function: Function::try_from(control_field).ok(),
                manufacturer_id,
                data: &data[10..data_end],
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

    #[test]
    fn test_trailing_frame_crc_is_not_payload() {
        let frame = [
            0x14, 0x44, 0xAE, 0x0C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x07, 0x8C, 0x20, 0x27, 0x78,
            0x0B, 0x13, 0x43, 0x65, 0x87, 0x7A, 0xC5,
        ];

        assert_eq!(trailing_frame_crc_start(&frame), Some(19));

        let parsed = WirelessFrame::try_from(frame.as_slice()).expect("valid wireless frame");
        assert_eq!(
            parsed.data,
            &[0x8C, 0x20, 0x27, 0x78, 0x0B, 0x13, 0x43, 0x65, 0x87]
        );
    }

    #[test]
    fn c_field_is_decoded_and_preserved() {
        let frame = [
            0x18, 0x44, 0xAE, 0x4C, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00,
        ];
        let parsed = WirelessFrame::try_from(frame.as_slice()).expect("valid wireless frame");
        assert_eq!(parsed.control_field, 0x44);
        assert_eq!(parsed.function, Some(Function::SndNr));
    }

    #[test]
    fn manufacturer_field_with_top_bit_set_is_decoded() {
        let frame = [
            0x18, 0x44, 0x97, 0xA6, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00,
        ];
        let parsed = WirelessFrame::try_from(frame.as_slice()).expect("valid wireless frame");
        assert_eq!(
            parsed.manufacturer_id.manufacturer_code.code,
            ['I', 'T', 'W']
        );
        assert!(!parsed.manufacturer_id.is_unique_globally);

        // Same code without the flag, which does mark a globally unique address.
        let mut frame = frame;
        frame[3] = 0x26;
        let parsed = WirelessFrame::try_from(frame.as_slice()).expect("valid wireless frame");
        assert_eq!(
            parsed.manufacturer_id.manufacturer_code.code,
            ['I', 'T', 'W']
        );
        assert!(parsed.manufacturer_id.is_unique_globally);
    }

    #[test]
    fn undecodable_manufacturer_field_is_not_reported_as_too_short() {
        let frame = [
            0x18, 0x44, 0x00, 0x00, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00,
        ];
        assert_eq!(
            WirelessFrame::try_from(frame.as_slice()),
            Err(FrameError::InvalidManufacturerCode { code: 0x0000 })
        );
    }

    #[test]
    fn unknown_c_field_does_not_invalidate_frame() {
        let frame = [
            0x18, 0x45, 0xAE, 0x4C, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00,
        ];
        let parsed = WirelessFrame::try_from(frame.as_slice()).expect("valid wireless frame");
        assert_eq!(parsed.control_field, 0x45);
        assert_eq!(parsed.function, None);
    }
}

#[cfg(test)]
mod format_a_tests {
    use super::*;

    fn encode(payload: &[u8]) -> Vec<u8> {
        let header = [0, 0x44, 0x49, 0x6A, 0x31, 0, 1, 0x55, 0x14, 0x37];
        let mut frame = header.to_vec();
        frame.extend(crc16_en13757(&header).to_be_bytes());
        for chunk in payload.chunks(16) {
            frame.extend(chunk);
            frame.extend(crc16_en13757(chunk).to_be_bytes());
        }
        frame
    }

    #[test]
    fn normalized_bytes_skip_crcs_across_block_boundaries() {
        for length in [0, 1, 15, 16, 17, 31, 32, 33, 80, 240] {
            let payload: Vec<u8> = (0..length).map(|n| (n * 17) as u8).collect();
            let encoded = encode(&payload);
            let original = encoded.clone();
            let view = FormatAFrame::new(&encoded).unwrap();
            let mut expected = vec![(length + 9) as u8];
            expected.extend(&encoded[1..10]);
            expected.extend(&payload);
            assert_eq!(view.len(), expected.len());
            assert!(!view.is_empty());
            assert!(view.bytes().eq(expected.iter().copied()));
            let mut bytes = view.bytes();
            for (index, want) in expected.iter().enumerate() {
                assert_eq!(bytes.len(), view.len() - index);
                assert_eq!(bytes.size_hint(), (bytes.len(), Some(bytes.len())));
                assert_eq!(bytes.next(), Some(*want));
                assert!(bytes.clone().eq(expected.iter().skip(index + 1).copied()));
            }
            assert_eq!(bytes.next(), None);
            assert_eq!(bytes.next(), None);
            assert_eq!(bytes.len(), 0);
            assert_eq!(encoded, original);
            let mut output = vec![0; encoded.len()];
            assert_eq!(
                strip_format_a_crcs(&encoded, &mut output),
                Some(expected.as_slice())
            );
        }
    }

    #[test]
    fn tail_and_validation_match_the_buffer_api() {
        for tail in [
            &[][..],
            &[0x33][..],
            &[0x33, 0x44][..],
            &[1, 2, 3, 4, 5][..],
        ] {
            let mut encoded = encode(&[]);
            encoded.extend(tail);
            let frame = FormatAFrame::new(&encoded).unwrap();
            assert!(frame.bytes().skip(10).eq(tail.iter().copied()));
            let mut too_small = vec![0xAA; encoded.len() - 1];
            assert!(strip_format_a_crcs(&encoded, &mut too_small).is_none());
            assert!(too_small.iter().all(|byte| *byte == 0xAA));
        }
        let encoded = encode(&[]);
        for length in 0..12 {
            assert!(FormatAFrame::new(&encoded[..length]).is_none());
        }
        let mut corrupt = encoded;
        corrupt[10] ^= 1;
        assert!(FormatAFrame::new(&corrupt).is_none());
    }

    #[test]
    fn iterator_borrows_source_not_temporary_view() {
        let encoded = encode(&[1, 2, 3]);
        let bytes = {
            let view = FormatAFrame::new(&encoded).unwrap();
            view.bytes()
        };
        assert_eq!(bytes.skip(10).collect::<Vec<_>>(), [1, 2, 3]);
    }
}
