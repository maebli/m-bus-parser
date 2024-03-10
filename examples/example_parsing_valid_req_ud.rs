use m_bus_parser::{frames::Frame, user_data::UserDataBlock};
use std::fs;
use walkdir::WalkDir;

//  This is an example of how to use the library to parse a frame.
fn main() {
    for entry in WalkDir::new("./tests/rscada/test-frames")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
    {
        let contents =
            fs::read_to_string(entry.path()).expect("Something went wrong reading the file");
        println!("Path: {}", entry.path().display());
        println!("Input:\n{}", contents);

        let contents = contents.trim().replace(" ", "");
        let bytes = hex::decode(contents).unwrap();
        let frame = Frame::try_from(bytes.as_slice()).unwrap();

        if let Frame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            if let Ok(parsed) = UserDataBlock::try_from(data) {
                println!("user data: {:?}", parsed);
            }
        }
    }
}
