#![no_std]

use libc::{uint8_t, size_t};
use core::slice;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ParseStatus {
    ParseOk = 0,
    ParseError = 1,
}

#[no_mangle]
pub extern "C" fn parse_mbus(data: *const uint8_t, length: size_t) -> ParseStatus {
    if data.is_null() || length == 0 {
        return ParseStatus::ParseError;
    }

    let slice = unsafe {
        slice::from_raw_parts(data, length as usize)
    };

    // Implement your parsing logic here
    if slice.len() == 5 {
        ParseStatus::ParseOk
    } else {
        ParseStatus::ParseError
    }
}
