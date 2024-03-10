use m_bus_parser::user_data::variable_user_data::DataRecord;

fn main() {
    /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    let data = vec![0x03, 0x13, 0x15, 0x31, 0x00];
    let result = DataRecord::try_from(data.as_slice());
    println!("{:?}", result);
}
