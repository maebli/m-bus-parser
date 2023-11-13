#[derive(Debug,PartialEq)]
struct ControlFrame {
    start: u8,
    length: u8,
    control: u8,
    address: u8,
    control_information: u8,
    checksum: u8,
    stop: u8,
}
