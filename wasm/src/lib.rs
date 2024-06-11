mod utils;
use m_bus_parser::{self, MbusData};

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

fn clean_and_convert(input: &str) -> Vec<u8> {
    let input = input.trim();
    let cleaned_data: String = input.replace("0x", "").replace([' ', ','], "");

    // Convert pairs of characters into bytes
    cleaned_data
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let byte_str = std::str::from_utf8(chunk).expect("Invalid UTF-8 sequence");
            u8::from_str_radix(byte_str, 16).expect("Invalid byte value")
        })
        .collect()
}

#[wasm_bindgen]
pub fn m_bus_parse(s: &str) -> String {
    let s = clean_and_convert(s);
    if let Ok(mbus_data) = MbusData::try_from(s.as_slice()) {
        serde_json::to_string_pretty(&mbus_data).unwrap()
    } else {
        format!("Failed to parse")
    }
}
