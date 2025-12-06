#[cfg(feature = "plaintext-before-extension")]
use m_bus_parser::user_data::UserDataBlock;
#[cfg(feature = "plaintext-before-extension")]
use std::fs;
#[cfg(feature = "plaintext-before-extension")]
use walkdir::WalkDir;

//  This is an example of how to use the library to parse a frame.
#[cfg(feature = "plaintext-before-extension")]
fn main() {
    for entry in WalkDir::new("./tests/rscada/test-frames")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
    {
        use m_bus_parser::WiredFrame;

        #[allow(clippy::expect_used)]
        let contents =
            fs::read_to_string(entry.path()).expect("Something went wrong reading the file");
        println!("Path: {}", entry.path().display());
        println!("Input:\n{}", contents);

        let contents = contents.trim().replace(" ", "");
        #[allow(clippy::unwrap_used)]
        let bytes = hex::decode(contents).unwrap();
        #[allow(clippy::unwrap_used)]
        let frame = wired_mbus_link_layer::WiredFrame::try_from(bytes.as_slice()).unwrap();

        if let WiredFrame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            if let Ok(parsed) = UserDataBlock::try_from(data) {
                println!("user data: {:?}", parsed);
                if let Ok(m_bus_parser::user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block,
                }) = m_bus_parser::user_data::UserDataBlock::try_from(data)
                {
                    println!("long_tpl_header: {:#?}", long_tpl_header);
                    println!("variable_data_block: {:?}", variable_data_block);
                    let data_records =
                        m_bus_parser::user_data::DataRecords::try_from(variable_data_block);
                    println!("data_records: {:#?}", data_records.unwrap());
                }
            }
        }
    }
}

#[cfg(not(feature = "plaintext-before-extension"))]
fn main() {
    println!("This example requires the `plaintext-before-extension` feature to be enabled.");
}
