mod utils;
use m_bus_parser;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn parse(s: &str) -> String {
    // remove  white space
    let s = s.trim().replace([' ', "0x", '\\'], "");
    m_bus_parser.parse(s);
}
