use m_bus_parser::serialize_mbus_data;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn m_bus_parse(data: &str, format: &str) -> String {
    serialize_mbus_data(data, format, None)
}

#[wasm_bindgen]
pub fn m_bus_parse_with_key(data: &str, format: &str, key_hex: &str) -> String {
    let key = parse_key(key_hex);
    serialize_mbus_data(data, format, key.as_ref())
}

fn parse_key(key_hex: &str) -> Option<[u8; 16]> {
    if key_hex.is_empty() {
        return None;
    }
    let bytes: Vec<u8> = (0..key_hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&key_hex[i..i + 2], 16).ok())
        .collect();
    if bytes.len() == 16 {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Some(arr)
    } else {
        None
    }
}
