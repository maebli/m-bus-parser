use std::ffi::c_char;
use std::panic::{catch_unwind, UnwindSafe};
use std::ptr;
use std::slice;

use m_bus_parser::{
    decode_data_records, render_bytes, DecodeOptions, OutputError, OutputFormat, RenderOptions,
};

pub struct MbusResult {
    text: Vec<u8>,
    error_code: Vec<u8>,
}

fn terminated(text: String) -> Vec<u8> {
    let mut bytes = text.into_bytes();
    bytes.push(0);
    bytes
}

fn result_pointer(
    operation: impl FnOnce() -> Result<String, OutputError> + UnwindSafe,
) -> *mut MbusResult {
    let (text, code) = match catch_unwind(operation) {
        Ok(Ok(text)) => (text, ""),
        Ok(Err(error)) => (error.to_string(), error.code()),
        Err(_) => (
            "The parser encountered an internal error".to_owned(),
            "internal.panic",
        ),
    };
    Box::into_raw(Box::new(MbusResult {
        text: terminated(text),
        error_code: terminated(code.to_owned()),
    }))
}

unsafe fn input_bytes<'a>(
    data: *const u8,
    length: usize,
    option: &'static str,
) -> Result<&'a [u8], OutputError> {
    if length == 0 {
        return Ok(&[]);
    }
    if data.is_null() || length > isize::MAX as usize {
        return Err(OutputError::InvalidOption {
            option,
            message: "a nonempty buffer requires a valid pointer and a length no greater than PTRDIFF_MAX".to_owned(),
        });
    }
    Ok(unsafe { slice::from_raw_parts(data, length) })
}

unsafe fn parse_key(key: *const u8, length: usize) -> Result<Option<[u8; 16]>, OutputError> {
    match length {
        0 => Ok(None),
        16 => {
            let bytes = unsafe { input_bytes(key, length, "key")? };
            let mut key = [0; 16];
            key.copy_from_slice(bytes);
            Ok(Some(key))
        }
        _ => Err(OutputError::InvalidOption {
            option: "key",
            message: "the key must contain exactly 16 bytes, or be omitted with length zero".into(),
        }),
    }
}

fn output_format(format: i32) -> Result<OutputFormat, OutputError> {
    match format {
        0 => Ok(OutputFormat::Json),
        1 => Ok(OutputFormat::Yaml),
        2 => Ok(OutputFormat::Csv),
        3 => Ok(OutputFormat::Xml),
        4 => Ok(OutputFormat::Table),
        5 => Ok(OutputFormat::Mermaid),
        6 => Ok(OutputFormat::Annotated),
        7 => Ok(OutputFormat::AnnotatedText),
        _ => Err(OutputError::UnsupportedFormat {
            format: format.to_string(),
        }),
    }
}

#[no_mangle]
pub extern "C" fn mbus_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn mbus_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn mbus_parse(
    data: *const u8,
    data_length: usize,
    key: *const u8,
    key_length: usize,
) -> *mut MbusResult {
    unsafe { mbus_render(data, data_length, key, key_length, 0) }
}

#[no_mangle]
pub unsafe extern "C" fn mbus_render(
    data: *const u8,
    data_length: usize,
    key: *const u8,
    key_length: usize,
    format: i32,
) -> *mut MbusResult {
    result_pointer(|| {
        let format = output_format(format)?;
        let data = unsafe { input_bytes(data, data_length, "data")? };
        let key = unsafe { parse_key(key, key_length)? };
        render_bytes(
            data,
            format,
            &RenderOptions {
                decode: DecodeOptions {
                    key,
                    include_enrichment: true,
                },
                ..RenderOptions::default()
            },
        )
    })
}

#[repr(C)]
pub struct MbusFrame {
    pub data: *const u8,
    pub length: usize,
}

#[no_mangle]
pub unsafe extern "C" fn mbus_parse_fragments(
    frames: *const MbusFrame,
    count: usize,
    key: *const u8,
    key_length: usize,
) -> *mut MbusResult {
    result_pointer(|| {
        if frames.is_null() || !(1..=255).contains(&count) {
            return Err(OutputError::Rendering {
                code: "fragment.invalid",
                message: "expected 1 to 255 ordered packets".into(),
            });
        }
        let frames = unsafe { slice::from_raw_parts(frames, count) };
        let inputs = frames
            .iter()
            .map(|f| unsafe { input_bytes(f.data, f.length, "frame") })
            .collect::<Result<Vec<_>, _>>()?;
        let options = DecodeOptions {
            key: unsafe { parse_key(key, key_length)? },
            ..DecodeOptions::default()
        };
        let decoded = m_bus_parser::decode_fragment_frames(&inputs, &options)?;
        serde_json::to_string(&decoded).map_err(|e| OutputError::Serialization {
            format: "json",
            message: e.to_string(),
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn mbus_parse_records(data: *const u8, length: usize) -> *mut MbusResult {
    result_pointer(|| {
        let records = decode_data_records(unsafe { input_bytes(data, length, "data")? })?;
        serde_json::to_string(&records).map_err(|error| OutputError::Serialization {
            format: "json",
            message: error.to_string(),
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn mbus_result_text(result: *const MbusResult) -> *const c_char {
    unsafe { result.as_ref() }.map_or(ptr::null(), |result| result.text.as_ptr().cast())
}

#[no_mangle]
pub unsafe extern "C" fn mbus_result_text_length(result: *const MbusResult) -> usize {
    unsafe { result.as_ref() }.map_or(0, |result| result.text.len() - 1)
}

#[no_mangle]
pub unsafe extern "C" fn mbus_result_error_code(result: *const MbusResult) -> *const c_char {
    unsafe { result.as_ref() }.map_or(c"option.invalid".as_ptr(), |result| {
        if result.error_code.len() == 1 {
            ptr::null()
        } else {
            result.error_code.as_ptr().cast()
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mbus_result_error_code_length(result: *const MbusResult) -> usize {
    unsafe { result.as_ref() }.map_or("option.invalid".len(), |result| result.error_code.len() - 1)
}

#[no_mangle]
pub unsafe extern "C" fn mbus_result_free(result: *mut MbusResult) {
    if !result.is_null() {
        drop(unsafe { Box::from_raw(result) });
    }
}
