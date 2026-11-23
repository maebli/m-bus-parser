use wasm_bindgen::prelude::*;

mod highlight;
mod parser;

pub use highlight::m_bus_highlight;
pub use parser::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
