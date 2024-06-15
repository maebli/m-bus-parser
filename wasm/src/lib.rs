use m_bus_parser::serialize_mbus_data;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn m_bus_parse(data: &str, format: &str) -> String {
    serialize_mbus_data(data, format)
}
