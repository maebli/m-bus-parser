mod utils;
use m_bus_parser;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse(s: &str) -> String {
    // remove  white space
    let s = s.trim().replace([' ', "0x", '\\'], "");
    m_bus_parser.parse(s);
}
