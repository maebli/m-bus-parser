use m_bus_parser::frames::{Address, Frame, Function};
///       68 4d 4d 68 08 01 72 01 00 00 00 96 15 01 00 18 00 00 00 0c 78 56 00 00 00 01
///       fd 1b 00 02 fc 03 48 52 25 74 44 0d 22 fc 03 48 52 25 74 f1 0c 12 fc 03 48 52
///       25 74 63 11 02 65 b4 09 22 65 86 09 12 65 b7 09 01 72 00 72 65 00 00 b2 01 65
///       00 00 1f b3 16
///         
///       parsed with https://dev-lab.github.io/tmbus/tmbus.htm
///
///       Output:
///       Type    Data
///       A       1
///       Errors  
///       ID      1
///       ManId   ELV
///       Version 1
///       DeviceCode      0
///       DeviceType      Other
///       AccessN 24
///       Fabrication No  56
///       Digital Input   0 binary
///       %RH     33.96
///       %RH (Minimum)   33.13
///       %RH (Maximum)   44.51
///       External Temperature    24.84 °C
///       External Temperature (Minimum)  24.38 °C
///       External Temperature (Maximum)  24.87 °C
///       Averaging Duration      0 hours
///
//  This is an example of how to use the library to parse a frame.

#[cfg(feature = "plaintext-before-extension")]
fn main() {
    let example = vec![
        0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01, 0x00,
        0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B, 0x00, 0x02,
        0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74,
        0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11, 0x02, 0x65, 0xB4, 0x09,
        0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72, 0x00, 0x72, 0x65, 0x00, 0x00,
        0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
    ];

    let frame = Frame::try_from(example.as_slice()).unwrap();

    if let Frame::LongFrame {
        function,
        address,
        data,
    } = frame
    {
        assert_eq!(
            function,
            Function::RspUd {
                acd: false,
                dfc: false
            }
        );
        assert_eq!(address, Address::Primary(1));

        if let Ok(m_bus_parser::user_data::UserDataBlock::VariableDataStructure {
            fixed_data_header,
            variable_data_block,
        }) = m_bus_parser::user_data::UserDataBlock::try_from(data)
        {
            println!("fixed_data_header: {:#?}", fixed_data_header);
            println!("variable_data_block: {:?}", variable_data_block);
            let data_records = m_bus_parser::user_data::DataRecords::try_from(variable_data_block);
            println!("data_records: {:#?}", data_records.unwrap());
        }
    }
}

// todo: adjust  this example
#[cfg(not(feature = "plaintext-before-extension"))]
fn main() {
    let example = vec![
        0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01, 0x00,
        0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B, 0x00, 0x02,
        0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74,
        0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11, 0x02, 0x65, 0xB4, 0x09,
        0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72, 0x00, 0x72, 0x65, 0x00, 0x00,
        0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
    ];

    let frame = Frame::try_from(example.as_slice()).unwrap();

    if let Frame::LongFrame {
        function,
        address,
        data,
    } = frame
    {
        assert_eq!(
            function,
            Function::RspUd {
                acd: false,
                dfc: false
            }
        );
        assert_eq!(address, Address::Primary(1));

        if let Ok(m_bus_parser::user_data::UserDataBlock::VariableDataStructure {
            fixed_data_header,
            variable_data_block,
        }) = m_bus_parser::user_data::UserDataBlock::try_from(data)
        {
            println!("fixed_data_header: {:#?}", fixed_data_header);
            println!("variable_data_block: {:?}", variable_data_block);
            let data_records = m_bus_parser::user_data::DataRecords::try_from(variable_data_block);
            println!("data_records: {:#?}", data_records.unwrap());
        }
    }
}
