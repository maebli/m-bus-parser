//! Allocation-free extension point for DIF 0x0F/0x1F manufacturer tails.
//!
//! See the crate README and `examples/decoder.rs` for a complete synthetic decoder.
//! Decoders are ordinary trusted Rust: callers must review their termination and
//! bounds handling before using them in firmware. No runtime sandbox is provided.
#![no_std]
#![forbid(unsafe_code)]

use core::{fmt, ops::Range};
pub use m_bus_application_layer::data_information::{Month, SingleEveryOrInvalid};
pub use m_bus_application_layer::value_information::{Unit, UnitName};
pub use m_bus_core::DeviceType;

mod cursor;
mod dispatch;
pub use cursor::{Cursor, Reading};
pub use dispatch::{decode, DecodeSummary};

/// Transport identity; missing values are never inferred from a decoder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeterInfo {
    pub manufacturer: Option<[u8; 3]>,
    pub version: Option<u8>,
    pub device: Option<DeviceType>,
}

/// One stateless decoder entry. Version ranges are inclusive.
#[derive(Debug, Clone, Copy)]
pub struct Decoder {
    pub name: &'static str,
    /// Vendor specification URL and section.
    pub source: &'static str,
    pub manufacturer: [u8; 3],
    pub versions: Option<(u8, u8)>,
    pub device: Option<DeviceType>,
    pub decode: DecodeFn,
}

/// Emit borrowed fields synchronously and return the number of tail bytes consumed.
/// Fields must be consumed in the callback. Errors use tail-relative offsets;
/// earlier fields are retained. Functions must terminate without panicking or allocating.
pub type DecodeFn = fn(&MeterInfo, &[u8], &mut dyn FnMut(Field<'_>)) -> Result<usize, DecodeError>;

impl Decoder {
    pub fn is_valid(&self) -> bool {
        self.manufacturer.iter().all(u8::is_ascii_uppercase)
            && self.versions.is_none_or(|(min, max)| min <= max)
    }

    pub fn matches(&self, meter: &MeterInfo) -> bool {
        self.is_valid()
            && meter.manufacturer == Some(self.manufacturer)
            && self
                .versions
                .is_none_or(|(min, max)| meter.version.is_some_and(|v| (min..=max).contains(&v)))
            && self
                .device
                .is_none_or(|device| meter.device == Some(device))
    }
}

/// Integer keys never pass through floating point (including values above 2^53).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integer {
    Signed(i64),
    Unsigned(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DateValue {
    Date {
        day: SingleEveryOrInvalid<u8>,
        month: SingleEveryOrInvalid<Month>,
        year: SingleEveryOrInvalid<u16>,
    },
    DateTime {
        day: SingleEveryOrInvalid<u8>,
        month: SingleEveryOrInvalid<Month>,
        year: SingleEveryOrInvalid<u16>,
        hour: SingleEveryOrInvalid<u8>,
        minute: SingleEveryOrInvalid<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Integer(Integer),
    Date(DateValue),
    Bytes(&'a [u8]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Labels<'a> {
    None,
    /// Bit indices, not masks; only applies to unsigned integers.
    Flags(&'a [(u8, &'a str)]),
    /// Exact typed integer keys, not bit indices.
    Enum(&'a [(Integer, &'a str)]),
}

/// One reading. `range` is relative to the first byte of the manufacturer tail.
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub value: Value<'a>,
    pub range: Range<usize>,
    pub exponent: i8,
    pub units: &'a [Unit],
    pub labels: Labels<'a>,
}

impl<'a> Field<'a> {
    pub fn new(name: &'a str, value: Value<'a>, range: Range<usize>) -> Self {
        Self {
            name,
            value,
            range,
            exponent: 0,
            units: &[],
            labels: Labels::None,
        }
    }
    pub fn unsigned(name: &'a str, value: u64, range: Range<usize>) -> Self {
        Self::new(name, Value::Integer(Integer::Unsigned(value)), range)
    }
    pub fn signed(name: &'a str, value: i64, range: Range<usize>) -> Self {
        Self::new(name, Value::Integer(Integer::Signed(value)), range)
    }
    pub fn bytes(name: &'a str, bytes: &'a [u8], range: Range<usize>) -> Self {
        Self::new(name, Value::Bytes(bytes), range)
    }
    pub fn exponent(mut self, exponent: i8) -> Self {
        self.exponent = exponent;
        self
    }
    pub fn units(mut self, units: &'a [Unit]) -> Self {
        self.units = units;
        self
    }
    pub fn flags(mut self, labels: &'a [(u8, &'a str)]) -> Self {
        self.labels = Labels::Flags(labels);
        self
    }
    pub fn enumeration(mut self, labels: &'a [(Integer, &'a str)]) -> Self {
        self.labels = Labels::Enum(labels);
        self
    }
    /// Iterate active flag names or matching enum names without allocating.
    pub fn active_labels(&self) -> impl Iterator<Item = &'a str> + '_ {
        let flags = match self.labels {
            Labels::Flags(v) => v,
            _ => &[],
        };
        let enums = match self.labels {
            Labels::Enum(v) => v,
            _ => &[],
        };
        flags
            .iter()
            .filter_map(|(bit, name)| match self.value {
                Value::Integer(Integer::Unsigned(v))
                    if 1u64
                        .checked_shl(u32::from(*bit))
                        .is_some_and(|mask| v & mask != 0) =>
                {
                    Some(*name)
                }
                _ => None,
            })
            .chain(
                enums.iter().filter_map(|(key, name)| {
                    (self.value == Value::Integer(*key)).then_some(*name)
                }),
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    InsufficientData { needed: usize, remaining: usize },
    InvalidWidth,
    InvalidBcd,
    InvalidDate,
    InvalidConsumedLength,
    InvalidField,
    InvalidValue(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError {
    pub offset: usize,
    pub kind: ErrorKind,
}
impl DecodeError {
    pub const fn new(offset: usize, kind: ErrorKind) -> Self {
        Self { offset, kind }
    }
}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "manufacturer tail byte {}: ", self.offset)?;
        match self.kind {
            ErrorKind::InsufficientData { needed, remaining } => {
                write!(f, "need {needed} bytes, have {remaining}")
            }
            ErrorKind::InvalidWidth => f.write_str("unsupported integer or BCD width"),
            ErrorKind::InvalidBcd => f.write_str("invalid BCD digit"),
            ErrorKind::InvalidDate => f.write_str("invalid date encoding"),
            ErrorKind::InvalidConsumedLength => {
                f.write_str("decoder consumed length is inconsistent with tail or fields")
            }
            ErrorKind::InvalidField => f.write_str("invalid field range or labels"),
            ErrorKind::InvalidValue(reason) => f.write_str(reason),
        }
    }
}
impl core::error::Error for DecodeError {}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
