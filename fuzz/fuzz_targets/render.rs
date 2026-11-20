#![no_main]

use libfuzzer_sys::fuzz_target;
use m_bus_parser::serialize_mbus_data;

const OUTPUT_FORMATS: &[&str] = &[
    "json-legacy",
    "yaml-legacy",
    "csv-wide",
    "annotated-legacy",
    "hexview-legacy",
    "table",
    "json",
    "yaml",
    "csv",
    "mermaid",
    "xml",
    "annotated",
    "annotated-text",
];

const USE_KEY_FLAG: u8 = 0x80;
const FUZZ_KEY: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
];

fuzz_target!(|data: &[u8]| {
    let Some((&selector, frame)) = data.split_first() else {
        return;
    };
    let format = OUTPUT_FORMATS[usize::from(selector & !USE_KEY_FLAG) % OUTPUT_FORMATS.len()];
    let key = (selector & USE_KEY_FLAG != 0).then_some(&FUZZ_KEY);
    let frame_as_hex: String = frame.iter().map(|byte| format!("{byte:02X}")).collect();

    let output = serialize_mbus_data(&frame_as_hex, format, key);
    std::hint::black_box(output);
});
