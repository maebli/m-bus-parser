use std::fs;
use walkdir::WalkDir;
use hex;
use serde::Deserialize;
use serde_xml_rs::from_str;
#
[derive(Debug, Deserialize)]
pub struct MBusData {
    #[serde(rename = "SlaveInformation")]
    slave_information: SlaveInformation,

    #[serde(rename = "DataRecord", default)]
    data_records: Vec<DataRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SlaveInformation {
    #[serde(rename = "Id")]
    id: String,

    #[serde(rename = "Manufacturer")]
    manufacturer: String,

    #[serde(rename = "Version")]
    version: u8,

    #[serde(rename = "ProductName")]
    product_name: String,

    #[serde(rename = "Medium")]
    medium: String,

    #[serde(rename = "AccessNumber")]
    access_number: u32,

    #[serde(rename = "Status")]
    status: String,

    #[serde(rename = "Signature")]
    signature: String,
}

#[derive(Debug, Deserialize)]
pub struct DataRecord {
    #[serde(rename = "id")]
    id: String,

    #[serde(rename = "Function")]
    function: String,

    #[serde(rename = "StorageNumber")]
    storage_number: Option<u32>,

    #[serde(rename = "Tariff", default)]
    tariff: Option<u8>,

    #[serde(rename = "Device", default)]
    device: Option<u8>,

    #[serde(rename = "Unit")]
    unit: Option<String>,

    #[serde(rename = "Value")]
    value: Option<String>,
}
#[cfg(test)]
mod tests {

    use m_bus_parser::{frames::{parse_frame, FrameType}, user_data::{parse_user_data, UserDataBlock}};

    use super::*;

    #[test]
    fn test_print_hex_files() {
        /* parses all the good examples, shouldn't throw any errors. */
        for entry in WalkDir::new("./tests/rscada/test-frames")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
        {

            let contents = fs::read_to_string(entry.path())
                .expect("Something went wrong reading the file");
            println!("Path: {}", entry.path().display());
            let xml_path = entry.path().with_extension("xml");
            let xml_content = fs::read_to_string(xml_path)
                .expect("Something went wrong reading the file");
            let mbus_data: MBusData = from_str(&xml_content).unwrap();
            
            println!("{:?}", mbus_data);
            println!("Input:\n{}", contents);

            let contents = contents.trim().replace(" ", "");
            let bytes = hex::decode(contents).unwrap();
            let frame = parse_frame(bytes.as_slice()).unwrap();

            if let FrameType::LongFrame { function, address, data } = frame {
                let user_data = parse_user_data(data).unwrap();
                if let UserDataBlock::VariableDataStructure { 
                    fixed_data_header, 
                    variable_data_block,
                     mdh, 
                     manufacturer_specific_data } = user_data {
                        assert!( Into::<u32>::into(fixed_data_header.identification_number) == mbus_data.slave_information.id.parse::<u32>().unwrap());
                        
                     }
            }
        }
    }
}
