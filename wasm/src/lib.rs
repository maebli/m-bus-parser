use std::str::FromStr;

use js_sys::{Error as JsError, Reflect};
use m_bus_parser::{
    decode_hex, decode_hex_bytes, render_hex, DecodeOptions, OutputError, OutputFormat,
    RenderOptions,
};
use wasm_bindgen::prelude::*;

mod highlight;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Highlight JSON, YAML, CSV, XML, or plaintext with the bundled Rust grammar set.
#[wasm_bindgen]
pub fn m_bus_highlight(source: &str, language: &str) -> Result<String, JsValue> {
    highlight::highlight_source(source, language)
        .map_err(|error| JsError::new(&format!("syntax highlighting failed: {error}")).into())
}

fn error_to_js(error: OutputError) -> JsValue {
    let js_error = JsError::new(&error.to_string());
    js_error.set_name("MbusParserError");
    let value: JsValue = js_error.into();
    let _ = Reflect::set(
        &value,
        &JsValue::from_str("code"),
        &JsValue::from_str(error.code()),
    );
    let _ = Reflect::set(
        &value,
        &JsValue::from_str("layer"),
        &JsValue::from_str(error.layer()),
    );
    if let Some(offset) = error.byte_offset() {
        let _ = Reflect::set(
            &value,
            &JsValue::from_str("byteOffset"),
            &JsValue::from_f64(offset as f64),
        );
    }
    value
}

fn parse_key(key_hex: Option<&str>) -> Result<Option<[u8; 16]>, OutputError> {
    let Some(key_hex) = key_hex.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let bytes = decode_hex_bytes(key_hex)?;
    let length = bytes.len();
    bytes
        .try_into()
        .map(Some)
        .map_err(|_| OutputError::InvalidOption {
            option: "key",
            message: format!("must contain exactly 16 bytes, received {length}"),
        })
}

fn decode_options(
    key_hex: Option<&str>,
    include_enrichment: Option<bool>,
) -> Result<DecodeOptions, OutputError> {
    Ok(DecodeOptions {
        key: parse_key(key_hex)?,
        include_enrichment: include_enrichment.unwrap_or(true),
    })
}

/// Decode into the canonical schema as a native JavaScript object.
#[wasm_bindgen]
pub fn m_bus_decode(
    data: &str,
    key_hex: Option<String>,
    include_enrichment: Option<bool>,
) -> Result<JsValue, JsValue> {
    let decoded = decode_hex(
        data,
        &decode_options(key_hex.as_deref(), include_enrichment).map_err(error_to_js)?,
    )
    .map_err(error_to_js)?;
    serde_wasm_bindgen::to_value(&decoded).map_err(|error| {
        error_to_js(OutputError::Serialization {
            format: "javascript",
            message: error.to_string(),
        })
    })
}

/// Render with the shared Rust/Python/CLI output contract.
#[wasm_bindgen]
pub fn m_bus_render(
    data: &str,
    format: &str,
    key_hex: Option<String>,
    width: Option<usize>,
    include_enrichment: Option<bool>,
) -> Result<String, JsValue> {
    let format = OutputFormat::from_str(format).map_err(error_to_js)?;
    let options = RenderOptions {
        decode: decode_options(key_hex.as_deref(), include_enrichment).map_err(error_to_js)?,
        table_width: width,
    };
    render_hex(data, format, &options).map_err(error_to_js)
}

/// Compatibility wrapper returning errors as text. Prefer `m_bus_render`.
#[wasm_bindgen]
pub fn m_bus_parse(data: &str, format: &str) -> String {
    m_bus_render(data, format, None, None, None).unwrap_or_else(|value| {
        js_sys::JSON::stringify(&value)
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_else(|| "M-Bus rendering failed".to_string())
    })
}

/// Compatibility wrapper returning errors as text. Prefer `m_bus_render`.
#[wasm_bindgen]
pub fn m_bus_parse_with_key(data: &str, format: &str, key_hex: &str) -> String {
    m_bus_render(data, format, Some(key_hex.to_string()), None, None).unwrap_or_else(|value| {
        js_sys::JSON::stringify(&value)
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_else(|| "M-Bus rendering failed".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_key_validation_does_not_panic_on_odd_input() {
        assert!(parse_key(Some("123")).is_err());
        assert!(parse_key(Some("00")).is_err());
        assert!(parse_key(Some("00112233445566778899AABBCCDDEEFF")).is_ok());
    }
}
