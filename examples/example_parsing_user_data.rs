#![allow(clippy::unwrap_used, clippy::unnecessary_fallible_conversions)]

use m_bus_parser::user_data::DataRecords;

fn main() {
    /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    let data = vec![0x03, 0x13, 0x15, 0x31, 0x00, 0x03, 0x13, 0x15, 0x31, 0x00];
    let result = DataRecords::try_from(data.as_slice());
    assert!(result.is_ok());
    assert!(result.unwrap().count() == 2);
}
