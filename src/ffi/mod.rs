extern crate libc;
use libc::{uint8_t, size_t};

// Representing the C enum in Rust
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ParseStatus {
    ParseOk = 0,
    ParseError = 1,
}

extern "C" {
    pub fn parse_mbus(data: *const uint8_t, length: size_t) -> ParseStatus;
}


#[no_mangle]
pub extern "C" fn parse_mbus(data: *const uint8_t, length: size_t) -> ParseStatus {

    if data.is_null() || length == 0 {
        return ParseStatus::ParseError;
    }

    let slice = unsafe {
        std::slice::from_raw_parts(data, length as usize)
    };

    /* dummy code */
    if slice.len() == 5 {
        ParseStatus::ParseOk
    } else {
        ParseStatus::ParseError
    }
}
