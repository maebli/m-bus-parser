#![no_std]

use arrayvec::ArrayVec;
use core::{hint::black_box, mem::size_of};

/// Deliberately mirrors the capacity from the original M-Bus design.
/// The accepted *values* are still only integers from 1 through 10.
pub const CAPACITY: usize = 117;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    EmptyToken,
    NotANumber,
    OutsideOneToTen,
    TooManyNumbers,
}

fn parse_one(token: &str) -> Result<u8, ParseError> {
    if token.is_empty() {
        return Err(ParseError::EmptyToken);
    }

    let mut value = 0_u8;
    for byte in token.bytes() {
        if !byte.is_ascii_digit() {
            return Err(ParseError::NotANumber);
        }

        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(byte - b'0'))
            .ok_or(ParseError::NotANumber)?;
    }

    if (1..=10).contains(&value) {
        Ok(value)
    } else {
        Err(ParseError::OutsideOneToTen)
    }
}

// ---------------------------------------------------------------------------
// Eager variant: all numbers are parsed immediately and stored inline.
// ---------------------------------------------------------------------------

pub type ParsedNumbers = ArrayVec<u8, CAPACITY>;

pub fn parse_arrayvec(input: &str) -> Result<ParsedNumbers, ParseError> {
    let mut numbers = ParsedNumbers::new();

    for token in input.split_ascii_whitespace() {
        numbers
            .try_push(parse_one(token)?)
            .map_err(|_| ParseError::TooManyNumbers)?;
    }

    Ok(numbers)
}

// ---------------------------------------------------------------------------
// Lazy variant: only a borrowed input slice and its progress are retained.
// ---------------------------------------------------------------------------

pub struct Numbers<'a> {
    tokens: core::str::SplitAsciiWhitespace<'a>,
}

pub fn parse_iter(input: &str) -> Numbers<'_> {
    Numbers {
        tokens: input.split_ascii_whitespace(),
    }
}

impl Iterator for Numbers<'_> {
    type Item = Result<u8, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.next().map(parse_one)
    }
}

// Export symbols whose byte sizes can be inspected in a Cortex-M object file.
// This lets llvm-nm report target-specific type sizes without executing code.
#[used]
#[no_mangle]
pub static TYPE_SIZE_ARRAYVEC_U8_117: [u8; size_of::<ParsedNumbers>()] =
    [0; size_of::<ParsedNumbers>()];

#[used]
#[no_mangle]
pub static TYPE_SIZE_ITERATOR: [u8; size_of::<Numbers<'static>>()] =
    [0; size_of::<Numbers<'static>>()];

/// A simplified stand-in for a large decoded protocol record.
#[repr(C)]
pub struct DemoRecord {
    pub bytes: [u8; 280],
}

pub type DemoRecordBuffer = ArrayVec<DemoRecord, CAPACITY>;

#[used]
#[no_mangle]
pub static TYPE_SIZE_DEMO_RECORD_ARRAYVEC: [u8; size_of::<DemoRecordBuffer>()] =
    [0; size_of::<DemoRecordBuffer>()];

/// Forces the eager result to become a real local object so stack use remains
/// visible in generated assembly.
#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn eager_probe(input: *const u8, len: usize) -> u32 {
    let bytes = core::slice::from_raw_parts(input, len);
    let text = core::str::from_utf8_unchecked(bytes);

    let numbers = match parse_arrayvec(text) {
        Ok(numbers) => numbers,
        Err(_) => return u32::MAX,
    };

    black_box(&numbers);
    numbers.iter().map(|value| u32::from(*value)).sum()
}

/// Forces the iterator state to become observable while consuming records one
/// at a time.
#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn iterator_probe(input: *const u8, len: usize) -> u32 {
    let bytes = core::slice::from_raw_parts(input, len);
    let text = core::str::from_utf8_unchecked(bytes);
    let mut numbers = parse_iter(text);
    let mut sum = 0_u32;

    for result in numbers.by_ref() {
        match result {
            Ok(value) => sum += u32::from(value),
            Err(_) => return u32::MAX,
        }
    }

    black_box(&numbers);
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_variants_parse_the_same_numbers() {
        let input = "1 2 3 4 5 6 7 8 9 10";
        let eager = parse_arrayvec(input).unwrap();
        let mut lazy = parse_iter(input);

        for expected in eager {
            assert_eq!(lazy.next(), Some(Ok(expected)));
        }
        assert_eq!(lazy.next(), None);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            parse_iter("1 11").nth(1),
            Some(Err(ParseError::OutsideOneToTen))
        );
    }
}
