#[derive(Debug,PartialEq)]
struct LongFrame {
    start1: u8,
    length1: u8,
    length2: u8,
    start2: u8,
    control: u8,
    address: u8,
    control_information: u8,
    // data field varies in length, hence a Vec<u8>
    data: Vec<u8>,
    checksum: u8,
    stop: u8,
}
