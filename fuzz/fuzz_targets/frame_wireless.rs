#![no_main]

use libfuzzer_sys::fuzz_target;
use m_bus_parser::mbus_data::MbusData;
use m_bus_parser::WirelessFrame;

fuzz_target!(|data: &[u8]| {
    let Ok(parsed) = MbusData::<WirelessFrame>::try_from(data) else {
        return;
    };
    std::hint::black_box(&parsed.user_data);
    for record in parsed.data_records.into_iter().flatten().flatten() {
        std::hint::black_box(record.value());
    }
});
