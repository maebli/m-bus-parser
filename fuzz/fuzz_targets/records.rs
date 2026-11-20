#![no_main]

use libfuzzer_sys::fuzz_target;
use m_bus_application_layer::parse_data_records;

fuzz_target!(|data: &[u8]| {
    for (index, record) in parse_data_records(data).enumerate() {
        assert!(index < data.len());
        if let Ok(record) = record {
            let raw = record.raw_bytes();
            assert!(!raw.is_empty());
            assert!(record.data_record_header.get_size() <= raw.len());
            std::hint::black_box(record.value());
        }
    }
});
