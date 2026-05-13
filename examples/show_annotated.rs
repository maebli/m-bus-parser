use m_bus_parser::mbus_data::serialize_mbus_data;
use m_bus_parser::user_data::data_record::DataRecord;

fn main() {
    // The data records portion of the example frame (bytes 19..81)
    let data: Vec<u8> = vec![
        0x0C, 0x78, 0x56, 0x00, 0x00, 0x00,  // record 0: DIF=0x0C VIF=0x78 data=4 bytes
        0x01, 0xFD, 0x1B, 0x00,              // record 1: DIF=0x01 VIF=0xFD VIFE=0x1B data=1 byte
        0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, // record 2: DIF=0x02 VIF=0xFC ...
    ];
    
    // Try parsing from byte 10 (where record 2 starts)
    let remaining = &data[10..];
    eprintln!("Trying to parse from offset 10: {:02X?}", remaining);
    match DataRecord::try_from(remaining) {
        Ok(record) => {
            eprintln!("Record parsed OK, total size: {}", record.get_size());
            eprintln!("Header size: {}", record.data_record_header.get_size());
            eprintln!("Data: {:?}", record.data);
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
        }
    }
    
    // Also show annotated output
    let input = "68 4D 4D 68 08 01 72 01 00 00 00 96 15 01 00 18 00 00 00 0C 78 56 00 00 00 01 FD 1B 00 02 FC 03 48 52 25 74 44 0D 22 FC 03 48 52 25 74 F1 0C 12 FC 03 48 52 25 74 63 11 02 65 B4 09 22 65 86 09 12 65 B7 09 01 72 00 72 65 00 00 B2 01 65 00 00 1F B3 16";
    let output = serialize_mbus_data(input, "annotated", None);
    println!("{}", output);
}
