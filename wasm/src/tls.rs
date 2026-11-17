use super::*;
use m_bus_parser::output::ImportedTlsSession;

#[wasm_bindgen]
pub fn m_bus_decode_tls(data: &str, session_json: &str) -> Result<JsValue, JsValue> {
    let imported = ImportedTlsSession::from_json(session_json).map_err(error_to_js)?;
    let decoded = m_bus_parser::output::decode_hex_with_tls_session(
        data,
        &DecodeOptions::default(),
        &imported.session(),
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
pub fn m_bus_render_tls(
    data: &str,
    format: &str,
    session_json: &str,
    width: Option<usize>,
    application_only: bool,
) -> Result<String, JsValue> {
    let imported = ImportedTlsSession::from_json(session_json).map_err(error_to_js)?;
    let format = OutputFormat::from_str(format).map_err(error_to_js)?;
    let options = RenderOptions {
        table_width: width,
        ..RenderOptions::default()
    };
    let session = imported.session();
    if application_only {
        m_bus_parser::output::render_application_hex_with_tls_session(
            data, format, &options, &session,
        )
    } else {
        m_bus_parser::output::render_hex_with_tls_session(data, format, &options, &session)
    }
    .map_err(error_to_js)
}
