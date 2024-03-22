use m_bus_parser::user_data::DataRecords;

fn main() {
    /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    /* Data block 2: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    let data = vec![0x03, 0x13, 0x15, 0x31, 0x00, 0x03, 0x13, 0x15, 0x31, 0x00];
    let result = DataRecords::try_from(data.as_slice());
    assert!(result.is_ok());
    assert!(result.unwrap().len() == 2);

    let data = vec![
        21, 1, 0, 24, 0, 0, 0, 12, 120, 86, 0, 0, 0, 1, 253, 27, 0, 2, 252, 3, 72, 82, 37, 116, 68,
        13, 34, 252, 3, 72, 82, 37, 116, 241, 12, 18, 252, 3, 72, 82, 37, 116, 99, 17, 2, 101, 180,
        9, 34, 101, 134, 9, 18, 101, 183, 9, 1, 114, 0, 114, 101, 0, 0, 178, 1, 101,
    ];
    let result = DataRecords::try_from(data.as_slice());
    println!("{:?}", result);
}
