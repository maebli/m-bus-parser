use core::slice;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ParseStatus {
    ParseOk = 0,
    ParseError = 1,
}

#[no_mangle]
pub extern "C" fn parse_mbus(data: *const u8, length: u32) -> ParseStatus {
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
