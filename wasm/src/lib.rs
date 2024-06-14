use m_bus_parser::{self, clean_and_convert, MbusData};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn m_bus_parse(s: &str) -> String {
    let s = clean_and_convert(s);
    if let Ok(mbus_data) = MbusData::try_from(s.as_slice()) {
        serde_json::to_string_pretty(&mbus_data).unwrap()
    } else {
        "Failed to parse".to_string()
    }
}
