use m_bus_parser::user_data::variable_user_data::parse_variable_data;

fn main() {
    /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
    let data = vec![0x03, 0x13, 0x15, 0x31, 0x00];
    let  mut records = [None; 117];
    let result = parse_variable_data(&data, &mut records);
    println!("{:?}", result);
}
