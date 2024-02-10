use std::fs;
use walkdir::WalkDir;
use hex;

#[cfg(test)]
mod tests {
    use m_bus_parser::parse_frame;

    use super::*;

    #[test]
    fn test_print_hex_files() {
        /* parses all the good examples, shouldn't throw any errors. */
        for entry in WalkDir::new("./test-frames")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
        {

            let contents = fs::read_to_string(entry.path())
                .expect("Something went wrong reading the file");
            println!("Path: {}", entry.path().display());
            println!("Input:\n{}", contents);

            let contents = contents.trim().replace(" ", "");
            let bytes = hex::decode(contents).unwrap();
            let frame = parse_frame(bytes.as_slice()).unwrap();
            println!("{:?}", frame);
            
        }
    }
}
