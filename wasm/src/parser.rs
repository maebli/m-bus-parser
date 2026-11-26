use std::str::FromStr;

use js_sys::{Error as JsError, Reflect};
use m_bus_parser::{
    decode_capture_hex, decode_hex_bytes, render_hex, DecodeOptions, OutputError, OutputFormat,
    RenderOptions,
};
use wasm_bindgen::prelude::*;

#[cfg(feature = "decryption-mode-13")]
#[path = "tls.rs"]
mod tls;
#[cfg(feature = "decryption-mode-13")]
pub use tls::{m_bus_decode_tls, m_bus_render_tls};

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
    let decoded = decode_capture_hex(
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

#[wasm_bindgen]
pub fn m_bus_decode_fragments(
    frames: Vec<String>,
    key_hex: Option<String>,
    include_enrichment: Option<bool>,
) -> Result<JsValue, JsValue> {
    let options = decode_options(key_hex.as_deref(), include_enrichment).map_err(error_to_js)?;
    if !(1..=255).contains(&frames.len()) {
        return Err(error_to_js(OutputError::Rendering {
            code: "fragment.invalid",
            message: "expected 1 to 255 ordered packets".into(),
        }));
    }
    let bytes = frames
        .iter()
        .map(|frame| decode_hex_bytes(frame))
        .collect::<Result<Vec<_>, _>>()
        .map_err(error_to_js)?;
    let slices = bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let decoded = m_bus_parser::decode_fragment_frames(&slices, &options).map_err(error_to_js)?;
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

#[cfg(feature = "decryption-mode-13")]
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ReportSettings {
    capture: m_bus_parser::output::OmsCaptureOptions,
    #[cfg(feature = "decryption-mode-10")]
    lpwan_mac_key: Option<String>,
    #[cfg(feature = "decryption-mode-10")]
    lpwan_derivation_counter: Option<u8>,
    #[cfg(feature = "decryption-mode-7")]
    application_key: Option<String>,
}

#[cfg(feature = "decryption-mode-13")]
#[wasm_bindgen]
pub fn m_bus_render_oms_capture(
    input: &str,
    selected: usize,
    format: &str,
    consecutive: bool,
    key_hex: Option<String>,
    session_json: Option<String>,
    context_json: Option<String>,
) -> Result<String, JsValue> {
    use m_bus_parser::output::{
        render_application_oms_with_keys, render_oms_capture_with_options, FragmentOutput,
        ImportedTlsSession,
    };
    let OutputFormat::Oms(format) = OutputFormat::from_str(format).map_err(error_to_js)? else {
        return Err(JsError::new("Expected an OMS report format.").into());
    };
    let settings: ReportSettings = context_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| JsError::new(&error.to_string()))?
        .unwrap_or_default();
    let mut capture = settings.capture;
    capture.consecutive = consecutive;
    let options = decode_options(key_hex.as_deref(), None).map_err(error_to_js)?;
    #[cfg(feature = "decryption-mode-7")]
    let application_key = parse_key(settings.application_key.as_deref()).map_err(error_to_js)?;
    #[cfg(not(feature = "decryption-mode-7"))]
    let application_key: Option<[u8; 16]> = None;
    let imported = session_json
        .as_deref()
        .map(ImportedTlsSession::from_json)
        .transpose()
        .map_err(error_to_js)?;
    let session = imported.as_ref().map(ImportedTlsSession::session);
    let mut frames = match decode_capture_hex(input, &DecodeOptions::default()) {
        Ok(FragmentOutput::Multiple(frames)) => frames,
        Ok(FragmentOutput::Single(frame)) => vec![*frame],
        Err(frame_error) if selected != 0 => return Err(error_to_js(frame_error)),
        Err(_) => {
            let options = RenderOptions {
                decode: options,
                ..RenderOptions::default()
            };
            return render_application_oms_with_keys(
                input,
                format,
                &options,
                capture,
                session.as_ref(),
                application_key.as_ref(),
            )
            .map_err(error_to_js);
        }
    };
    let frame = frames
        .get_mut(selected)
        .ok_or_else(|| JsError::new("Select a telegram in this capture."))?;
    if let Some(session) = session {
        *frame = m_bus_parser::output::decode_hex_with_tls_session(
            &frame.raw.original_frame_hex,
            &options,
            &session,
        )
        .map_err(error_to_js)?;
    } else if options.key.is_some() {
        let source = if frame.raw.fragment_frames_hex.len() > 1 {
            frame.raw.fragment_frames_hex.join("\n")
        } else {
            frame.raw.original_frame_hex.clone()
        };
        *frame = m_bus_parser::decode_hex(&source, &options).map_err(error_to_js)?;
    }
    #[cfg(feature = "decryption-mode-10")]
    if let Some(key) = parse_key(settings.lpwan_mac_key.as_deref()).map_err(error_to_js)? {
        frame
            .authenticate_lpwan_mac(&key, settings.lpwan_derivation_counter)
            .map_err(error_to_js)?;
    }
    #[cfg(feature = "decryption-mode-7")]
    if let Some(key) = application_key {
        frame.authenticate_application(&key).map_err(error_to_js)?;
    }
    render_oms_capture_with_options(&frames, format, Some(selected), capture).map_err(error_to_js)
}

/// Render CI-prefixed application data or bare DIF/VIF records without a link frame.
#[wasm_bindgen]
pub fn m_bus_render_application(
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
    m_bus_parser::output::render_application_hex(data, format, &options).map_err(error_to_js)
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

    #[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
    fn strict_key_validation_does_not_panic_on_odd_input() {
        assert!(parse_key(Some("123")).is_err());
        assert!(parse_key(Some("00")).is_err());
        assert!(parse_key(Some("00112233445566778899AABBCCDDEEFF")).is_ok());
    }
}
