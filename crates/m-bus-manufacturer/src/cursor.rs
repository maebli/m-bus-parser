use crate::{DateValue, DecodeError, ErrorKind};
use m_bus_application_layer::data_information::{DataFieldCoding, DataType};

/// Checked sequential reader. Failed reads leave the position unchanged.
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

macro_rules! integer_reads {
    ($(($unsigned:ident, $signed:ident, $width:expr, $big:expr)),* $(,)?) => {
        $(pub fn $unsigned(&mut self) -> Result<u64, DecodeError> { self.unsigned($width, $big) }
        pub fn $signed(&mut self) -> Result<i64, DecodeError> { self.signed($width, $big) })*
    };
}

impl<'a> Cursor<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    pub const fn position(&self) -> usize {
        self.position
    }
    pub fn remaining(&self) -> &'a [u8] {
        self.data.get(self.position..).unwrap_or(&[])
    }
    pub fn take(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
        let bytes = self.remaining().get(..length).ok_or_else(|| {
            DecodeError::new(
                self.position,
                ErrorKind::InsufficientData {
                    needed: length,
                    remaining: self.remaining().len(),
                },
            )
        })?;
        self.position += length;
        Ok(bytes)
    }
    pub fn skip(&mut self, length: usize) -> Result<(), DecodeError> {
        self.take(length).map(|_| ())
    }
    pub fn rest(&mut self) -> &'a [u8] {
        let bytes = self.remaining();
        self.position = self.data.len();
        bytes
    }
    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        self.unsigned(1, false).map(|v| v as u8)
    }
    pub fn i8(&mut self) -> Result<i8, DecodeError> {
        self.signed(1, false).map(|v| v as i8)
    }

    /// Read 1..=8 bytes. `big_endian = false` means little-endian.
    pub fn unsigned(&mut self, width: usize, big_endian: bool) -> Result<u64, DecodeError> {
        if !(1..=8).contains(&width) {
            return Err(DecodeError::new(self.position, ErrorKind::InvalidWidth));
        }
        let bytes = self.take(width)?;
        let mut value = 0u64;
        if big_endian {
            for &byte in bytes {
                value = (value << 8) | u64::from(byte);
            }
        } else {
            for (index, &byte) in bytes.iter().enumerate() {
                value |= u64::from(byte) << (index * 8);
            }
        }
        Ok(value)
    }
    pub fn signed(&mut self, width: usize, big_endian: bool) -> Result<i64, DecodeError> {
        let value = self.unsigned(width, big_endian)?;
        let shift = (8 - width) * 8;
        Ok(((value << shift) as i64) >> shift)
    }
    integer_reads! {
        (u16_le, i16_le, 2, false), (u16_be, i16_be, 2, true),
        (u24_le, i24_le, 3, false), (u24_be, i24_be, 3, true),
        (u32_le, i32_le, 4, false), (u32_be, i32_be, 4, true),
        (u48_le, i48_le, 6, false), (u48_be, i48_be, 6, true),
        (u64_le, i64_le, 8, false), (u64_be, i64_be, 8, true),
    }
    /// Read 2, 4, 6, 8, 10 or 12 packed decimal digits, exactly and unsigned.
    /// Reject nondecimal nibbles; vendor sign/sentinel conventions belong in the decoder.
    pub fn bcd(&mut self, digits: usize, big_endian: bool) -> Result<u64, DecodeError> {
        if !(2..=12).contains(&digits) || !digits.is_multiple_of(2) {
            return Err(DecodeError::new(self.position, ErrorKind::InvalidWidth));
        }
        let start = self.position;
        let bytes = self.take(digits / 2)?;
        if bytes.iter().any(|b| b & 15 > 9 || b >> 4 > 9) {
            self.position = start;
            return Err(DecodeError::new(start, ErrorKind::InvalidBcd));
        }
        let mut value = 0u64;
        let mut factor = 1u64;
        for &byte in bytes {
            let pair = u64::from(byte >> 4) * 10 + u64::from(byte & 15);
            if big_endian {
                value = value * 100 + pair;
            } else {
                value += pair * factor;
                factor *= 100;
            }
        }
        Ok(value)
    }
    pub fn date_g(&mut self) -> Result<DateValue, DecodeError> {
        self.date(DataFieldCoding::DateTypeG, 2)
    }
    pub fn datetime_f(&mut self) -> Result<DateValue, DecodeError> {
        self.date(DataFieldCoding::DateTimeTypeF, 4)
    }
    fn date(&mut self, coding: DataFieldCoding, length: usize) -> Result<DateValue, DecodeError> {
        let start = self.position;
        let bytes = self.take(length)?;
        // Preserve the protocol parser's Every/Invalid components rather than inventing dates.
        let value = match coding.parse(bytes, None).ok().and_then(|data| data.value) {
            Some(DataType::Date(day, month, year)) => Some(DateValue::Date { day, month, year }),
            Some(DataType::DateTime(day, month, year, hour, minute)) => Some(DateValue::DateTime {
                day,
                month,
                year,
                hour,
                minute,
            }),
            _ => None,
        };
        value.ok_or_else(|| {
            self.position = start;
            DecodeError::new(start, ErrorKind::InvalidDate)
        })
    }
}
