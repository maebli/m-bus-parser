use wasm_bindgen::prelude::*;

mod highlight;
mod parser;

fn load_syntaxes() -> syntect::parsing::SyntaxSet {
    syntect::parsing::SyntaxSet::load_defaults_newlines()
}

pub use highlight::m_bus_highlight;
pub use parser::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
