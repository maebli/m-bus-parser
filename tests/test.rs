use std::fs;
use walkdir::WalkDir;
use hex;

#[cfg(test)]
mod tests {
    use m_bus_parser::parse_frame;

    use super::*;

    #[test]
    fn test_print_hex_files() {
        for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
        {

            let xml_path = entry.path().with_extension("xml");
            let contents = fs::read_to_string(entry.path())
                .expect("Something went wrong reading the file");
            println!("Input:\n{}", contents);
            // contents to bytes array
            let contents = contents.trim().replace(" ", "");
            let bytes = hex::decode(contents).unwrap();
            let frame = parse_frame(bytes.as_slice()).unwrap();
            println!("{:?}", frame);
            
        }
    }
}
