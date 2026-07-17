use m_bus_application_layer::{parse_data_records, DataRecordError};

fn main() -> Result<(), DataRecordError> {
    // Application-layer records only: no wired/wireless frame or CI/TPL header.
    let data = [
        0x03, 0x13, 0x15, 0x31, 0x00, // Volume: 12_565 x 10^-3 m³
        0x02, 0x5A, 0xD7, 0x04, // Flow temperature: 1_239 x 10^-1 °C
    ];

    for (index, record) in parse_data_records(&data).enumerate() {
        let record = record?;

        println!("Record {}", index + 1);
        println!("  value: {:?}", record.value());
        if let Some(value_information) = record.value_information() {
            println!("  labels: {:?}", value_information.labels);
            println!("  scale: 10^{}", value_information.decimal_scale_exponent);
            println!("  units: {:?}", value_information.units);
        }
        println!("  raw bytes: {:02X?}", record.raw_bytes());
    }

    Ok(())
}
