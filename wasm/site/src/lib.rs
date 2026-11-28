#[cfg(feature = "parser")]
#[path = "../../src/parser.rs"]
pub mod parser;

#[cfg(feature = "syntax")]
#[path = "../../src/highlight.rs"]
pub mod highlight;

#[cfg(feature = "syntax")]
fn load_syntaxes() -> syntect::parsing::SyntaxSet {
    syntect::dumps::from_uncompressed_data(include_bytes!("../grammars/syntax.packdump"))
        .expect("bundled web grammars")
}

#[cfg(feature = "parser")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn version() -> String {
    m_bus_parser::VERSION.to_string()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "parser")]
    #[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
    fn website_version_matches_the_parser() {
        assert_eq!(super::version(), m_bus_parser::VERSION);
    }
}
