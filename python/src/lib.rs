use std::str::FromStr;

use m_bus_parser::{
    decode_bytes, decode_data_records, decode_hex_bytes, render_bytes, render_hex, DecodeOptions,
    OutputError, OutputFormat, RenderOptions,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

pyo3::create_exception!(
    pymbusparser,
    MbusParserError,
    pyo3::exceptions::PyValueError
);

fn parser_error(error: OutputError) -> PyErr {
    MbusParserError::new_err(format!("[{}] {error}", error.code()))
}

fn binding_error(code: &'static str, message: impl std::fmt::Display) -> PyErr {
    MbusParserError::new_err(format!("[{code}] {message}"))
}

fn extract_bytes(value: &Bound<'_, PyAny>, label: &str) -> PyResult<Vec<u8>> {
    if let Ok(text) = value.extract::<String>() {
        return decode_hex_bytes(&text).map_err(parser_error);
    }

    if let Ok(bytes) = value.extract::<Vec<u8>>() {
        if bytes.is_empty() {
            return Err(parser_error(OutputError::EmptyInput));
        }
        return Ok(bytes);
    }

    Err(PyTypeError::new_err(format!(
        "{label} must be a hexadecimal string or bytes-like object"
    )))
}

fn extract_key(key: Option<&Bound<'_, PyAny>>) -> PyResult<Option<[u8; 16]>> {
    let Some(value) = key else {
        return Ok(None);
    };

    let bytes = extract_bytes(value, "key")?;
    let length = bytes.len();
    bytes.try_into().map(Some).map_err(|_| {
        binding_error(
            "option.invalid",
            format!("key must contain exactly 16 bytes, received {length}"),
        )
    })
}

fn json_to_python(py: Python<'_>, json: &str) -> PyResult<Py<PyAny>> {
    PyModule::import(py, "json")?
        .call_method1("loads", (json,))
        .map(Bound::unbind)
}

fn records_json(data: &[u8]) -> PyResult<String> {
    let records = decode_data_records(data).map_err(parser_error)?;

    serde_json::to_string(&records).map_err(|error| {
        binding_error(
            "render.serialization",
            format!("failed to serialize application-layer records: {error}"),
        )
    })
}

/// Parse a complete wired or wireless M-Bus frame into the canonical schema.
///
/// ``data`` may be strict hexadecimal text or a bytes-like object. Pass a
/// 16-byte AES key as bytes or hexadecimal text for supported encrypted frames.
#[pyfunction]
#[pyo3(signature = (data, *, key=None, include_enrichment=true))]
fn parse(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    key: Option<&Bound<'_, PyAny>>,
    include_enrichment: bool,
) -> PyResult<Py<PyAny>> {
    let data = extract_bytes(data, "data")?;
    let decoded = decode_bytes(
        &data,
        &DecodeOptions {
            key: extract_key(key)?,
            include_enrichment,
        },
    )
    .map_err(parser_error)?;
    let json = serde_json::to_string(&decoded)
        .map_err(|error| binding_error("render.serialization", error))?;
    json_to_python(py, &json)
}

/// Parse application-layer data records into a native Python list.
#[pyfunction]
fn parse_records(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let data = extract_bytes(data, "data")?;
    json_to_python(py, &records_json(&data)?)
}

/// Render a complete M-Bus frame in one of the canonical text formats.
#[pyfunction]
#[pyo3(signature = (data, format="json", *, key=None, width=None, include_enrichment=true))]
fn render(
    data: &Bound<'_, PyAny>,
    format: &str,
    key: Option<&Bound<'_, PyAny>>,
    width: Option<usize>,
    include_enrichment: bool,
) -> PyResult<String> {
    let data = extract_bytes(data, "data")?;
    let format = OutputFormat::from_str(format).map_err(parser_error)?;
    render_bytes(
        &data,
        format,
        &RenderOptions {
            decode: DecodeOptions {
                key: extract_key(key)?,
                include_enrichment,
            },
            table_width: width,
        },
    )
    .map_err(parser_error)
}

/// Legacy JSON-string API. Prefer :func:`parse_records` for native objects.
#[pyfunction]
fn parse_application_layer(data_record: &str) -> PyResult<String> {
    let bytes = decode_hex_bytes(data_record).map_err(parser_error)?;
    records_json(&bytes)
}

/// Compatibility rendering API. Prefer :func:`parse` or :func:`render`.
#[pyfunction]
#[pyo3(signature = (data, format, key=None))]
fn m_bus_parse(data: &str, format: &str, key: Option<&str>) -> PyResult<String> {
    let key = key
        .map(decode_hex_bytes)
        .transpose()
        .map_err(parser_error)?
        .map(|bytes| {
            let length = bytes.len();
            bytes.try_into().map_err(|_| {
                binding_error(
                    "option.invalid",
                    format!("key must contain exactly 16 bytes, received {length}"),
                )
            })
        })
        .transpose()?;
    let format = OutputFormat::from_str(format).map_err(parser_error)?;
    render_hex(
        data,
        format,
        &RenderOptions {
            decode: DecodeOptions {
                key,
                include_enrichment: true,
            },
            ..RenderOptions::default()
        },
    )
    .map_err(parser_error)
}

/// Fast Python bindings for parsing wired and wireless M-Bus frames.
#[pymodule]
fn pymbusparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("MbusParserError", m.py().get_type::<MbusParserError>())?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_records, m)?)?;
    m.add_function(wrap_pyfunction!(render, m)?)?;
    m.add_function(wrap_pyfunction!(parse_application_layer, m)?)?;
    m.add_function(wrap_pyfunction!(m_bus_parse, m)?)?;
    m.add(
        "__all__",
        vec![
            "MbusParserError",
            "parse",
            "parse_records",
            "render",
            "parse_application_layer",
            "m_bus_parse",
            "__version__",
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_supported_readable_hex() {
        let decoded = decode_hex_bytes("0x68: 3d-0X16").unwrap();
        assert_eq!(decoded, [0x68, 0x3D, 0x16]);
    }

    #[test]
    fn rejects_ambiguous_hex() {
        assert!(decode_hex_bytes("68,3D").is_err());
        assert!(decode_hex_bytes("123").is_err());
    }

    #[test]
    fn validates_output_formats() {
        assert_eq!(OutputFormat::from_str("yml").unwrap(), OutputFormat::Yaml);
        assert_eq!(OutputFormat::from_str("xml").unwrap(), OutputFormat::Xml);
        assert!(OutputFormat::from_str("not-a-format").is_err());
    }
}
