#![allow(clippy::unwrap_used)]

use m_bus_parser::user_data::parse_data_records;

fn main() {
    /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    let data = vec![0x03, 0x13, 0x15, 0x31, 0x00, 0x03, 0x13, 0x15, 0x31, 0x00];
    let records: Result<Vec<_>, _> = parse_data_records(&data).collect();
    assert_eq!(records.unwrap().len(), 2);
}
