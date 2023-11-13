#[derive(Debug,PartialEq)]
struct ShortFrame {
    control: u8,
    address: u8,
    checksum: u8,
    stop: u8,
}
