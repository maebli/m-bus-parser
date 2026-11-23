#[cfg(feature = "parser")]
#[path = "../../src/parser.rs"]
pub mod parser;

#[cfg(feature = "syntax")]
#[path = "../../src/highlight.rs"]
pub mod highlight;

#[cfg(feature = "parser")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn version() -> String {
    m_bus_parser::VERSION.to_string()
}
