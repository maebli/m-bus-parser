use m_bus_parser::frames::{Address, Frame, Function};

//  This is an example of how to use the library to parse a frame.
fn main() {
    let example = vec![
        0x68, 0x1B, 0x1B, 0x68, 0x08, 0x01, 0x72, 0x07, 0x20, 0x18, 0x00, 0xE6, 0x1E, 0x35, 0x07,
        0x4C, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x07, 0x20, 0x18, 0x00, 0x0C, 0x16, 0x69, 0x02, 0x00,
        0x00, 0x96, 0x16,
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
            println!("fixed_data_header: {:?}", fixed_data_header);
            println!("variable_data_block: {:?}", variable_data_block);
            if let Ok(data) = m_bus_parser::user_data::DataRecords::try_from(variable_data_block) {
                println!("user data: {:?}", data);
            }
        }
    }
}
