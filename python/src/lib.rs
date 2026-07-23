use m_bus_parser::serialize_mbus_data;
use m_bus_parser::user_data::DataRecords;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

const FORMATS: &[&str] = &[
    "json",
    "yaml",
    "csv",
    "table",
    "mermaid",
    "xml",
    "annotated",
    "annotated-text",
    "hexview",
];

fn decode_hex(value: &str, label: &str) -> PyResult<Vec<u8>> {
    let without_prefix = value.replace("0x", "").replace("0X", "");
    let compact: String = without_prefix
        .chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && !matches!(character, ',' | ':' | '-' | '_')
        })
        .collect();

    if compact.is_empty() {
        return Err(PyValueError::new_err(format!("{label} must not be empty")));
    }

    hex::decode(&compact)
        .map_err(|error| PyValueError::new_err(format!("invalid hexadecimal {label}: {error}")))
}

fn extract_bytes(value: &Bound<'_, PyAny>, label: &str) -> PyResult<Vec<u8>> {
    if let Ok(text) = value.extract::<String>() {
        return decode_hex(&text, label);
    }

    if let Ok(bytes) = value.extract::<Vec<u8>>() {
        if bytes.is_empty() {
            return Err(PyValueError::new_err(format!("{label} must not be empty")));
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
    let key = bytes.try_into().map_err(|_| {
        PyValueError::new_err(format!(
            "key must contain exactly 16 bytes, received {length}"
        ))
    })?;

    Ok(Some(key))
}

fn normalized_data(value: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(hex::encode_upper(extract_bytes(value, "data")?))
}

fn normalize_format(format: &str) -> PyResult<&str> {
    let format = match format {
        "yml" => "yaml",
        value => value,
    };

    if FORMATS.contains(&format) {
        Ok(format)
    } else {
        Err(PyValueError::new_err(format!(
            "unsupported format {format:?}; expected one of: {}",
            FORMATS.join(", ")
        )))
    }
}

fn json_to_python(py: Python<'_>, json: &str) -> PyResult<Py<PyAny>> {
    PyModule::import(py, "json")?
        .call_method1("loads", (json,))
        .map(Bound::unbind)
}

fn parse_frame_json(data: &str, key: Option<&[u8; 16]>) -> PyResult<String> {
    let json = serialize_mbus_data(data, "json", key);
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|error| PyValueError::new_err(format!("parser returned invalid JSON: {error}")))?;

    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        return Err(PyValueError::new_err(
            "data is not a valid wired or wireless M-Bus frame",
        ));
    }

    Ok(json)
}

fn records_json(data: &[u8]) -> PyResult<String> {
    let records = DataRecords::from(data)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            PyValueError::new_err(format!(
                "failed to parse application-layer records: {error}"
            ))
        })?;

    serde_json::to_string(&records).map_err(|error| {
        PyValueError::new_err(format!(
            "failed to serialize application-layer records: {error}"
        ))
    })
}

/// Parse a complete wired or wireless M-Bus frame into native Python objects.
///
/// ``data`` may be a hexadecimal string or a bytes-like object. Pass a 16-byte
/// AES key as bytes or hexadecimal text to decrypt supported encrypted frames.
#[pyfunction]
#[pyo3(signature = (data, *, key=None))]
fn parse(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    key: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let data = normalized_data(data)?;
    let key = extract_key(key)?;
    let json = parse_frame_json(&data, key.as_ref())?;
    json_to_python(py, &json)
}

/// Parse application-layer data records into a native Python list.
#[pyfunction]
fn parse_records(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let data = extract_bytes(data, "data")?;
    json_to_python(py, &records_json(&data)?)
}

/// Render a complete M-Bus frame in one of the parser's text formats.
#[pyfunction]
#[pyo3(signature = (data, format="json", *, key=None))]
fn render(
    data: &Bound<'_, PyAny>,
    format: &str,
    key: Option<&Bound<'_, PyAny>>,
) -> PyResult<String> {
    let data = normalized_data(data)?;
    let format = normalize_format(format)?;
    let key = extract_key(key)?;
    let json = parse_frame_json(&data, key.as_ref())?;
    if format == "json" {
        Ok(json)
    } else {
        Ok(serialize_mbus_data(&data, format, key.as_ref()))
    }
}

/// Legacy JSON-string API. Prefer :func:`parse_records` for native objects.
#[pyfunction]
fn parse_application_layer(data_record: &str) -> PyResult<String> {
    let bytes = decode_hex(data_record, "data")?;
    records_json(&bytes)
}

/// Legacy rendering API. Prefer :func:`parse` or :func:`render`.
#[pyfunction]
#[pyo3(signature = (data, format, key=None))]
fn m_bus_parse(data: &str, format: &str, key: Option<&str>) -> PyResult<String> {
    let data = decode_hex(data, "data")?;
    let data = hex::encode_upper(data);
    let format = normalize_format(format)?;
    let key = key
        .map(|value| decode_hex(value, "key"))
        .transpose()?
        .map(|bytes| {
            let length = bytes.len();
            bytes.try_into().map_err(|_| {
                PyValueError::new_err(format!(
                    "key must contain exactly 16 bytes, received {length}"
                ))
            })
        })
        .transpose()?;

    Ok(serialize_mbus_data(&data, format, key.as_ref()))
}

/// Fast Python bindings for parsing wired and wireless M-Bus frames.
#[pymodule]
fn pymbusparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_records, m)?)?;
    m.add_function(wrap_pyfunction!(render, m)?)?;
    m.add_function(wrap_pyfunction!(parse_application_layer, m)?)?;
    m.add_function(wrap_pyfunction!(m_bus_parse, m)?)?;
    m.add(
        "__all__",
        vec![
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
    fn decodes_readable_hex() {
        let decoded = decode_hex("0x68: 3D-3d,68", "data").unwrap();
        assert_eq!(decoded, [0x68, 0x3D, 0x3D, 0x68]);
    }

    #[test]
    fn rejects_invalid_hex() {
        let error = decode_hex("123", "data").unwrap_err();
        assert!(error.to_string().contains("invalid hexadecimal data"));
    }

    #[test]
    fn validates_output_formats() {
        assert_eq!(normalize_format("yml").unwrap(), "yaml");
        assert!(normalize_format("xml").is_err());
    }
}
