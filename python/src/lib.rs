use hex;
use m_bus_parser::serialize_mbus_data;
use m_bus_parser::user_data::DataRecords;
use pyo3::prelude::*;
use serde_json;

#[pyfunction]
fn parse_application_layer(data_record: &str) -> PyResult<String> {
    // Decode the hex string into bytes
    match hex::decode(data_record) {
        Ok(bytes) => {
            // Try to parse the bytes into DataRecords
            match DataRecords::try_from(bytes.as_slice()) {
                Ok(records) => {
                    // Serialize the records to JSON using Serde
                    match serde_json::to_string(&records) {
                        Ok(json) => Ok(json),
                        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Failed to serialize records to JSON: {}",
                            e
                        ))),
                    }
                }
                Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Failed to parse data record",
                )),
            }
        }
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to decode hex: {}",
            e
        ))),
    }
}

#[pyfunction]
pub fn m_bus_parse(data: &str, format: &str) -> String {
    serialize_mbus_data(data, format)
}

#[pymodule]
fn pymbusparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_application_layer, m)?)?;
    m.add_function(wrap_pyfunction!(m_bus_parse, m)?)?;
    Ok(())
}
