use serde::Deserialize;
use serde_xml_rs::from_str;
use std::fs;
use walkdir::WalkDir;
#[derive(Debug, Deserialize)]
pub struct MBusData {
    #[serde(rename = "SlaveInformation")]
    slave_information: SlaveInformation,

    #[serde(rename = "DataRecord", default)]
    _data_records: Vec<DataRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SlaveInformation {
    #[serde(rename = "Id")]
    id: String,

    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,

    #[serde(rename = "Version")]
    version: Option<u8>,

    #[serde(rename = "ProductName")]
    _product_name: Option<String>,

    #[serde(rename = "Medium")]
    _medium: String,

    #[serde(rename = "AccessNumber")]
    access_number: u32,

    #[serde(rename = "Status")]
    status: String,

    #[serde(rename = "Signature")]
    signature: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DataRecord {
    #[serde(rename = "id")]
    _id: String,

    #[serde(rename = "Function")]
    _function: String,

    #[serde(rename = "StorageNumber")]
    _storage_number: Option<u32>,

    #[serde(rename = "Tariff", default)]
    _tariff: Option<u8>,

    #[serde(rename = "Device", default)]
    _device: Option<u8>,

    #[serde(rename = "Unit")]
    _unit: Option<String>,

    #[serde(rename = "Value")]
    _value: Option<String>,
}

#[cfg(test)]
mod tests {

    use m_bus_parser::{user_data::UserDataBlock, WiredFrame};

    use super::*;

    #[test]
    fn test_valid_control_frame_parsing() {
        /* parses all the good examples, shouldn't throw any errors. */
        for entry in WalkDir::new("./tests/rscada/test-frames")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "hex"))
        {
            let contents =
                fs::read_to_string(entry.path()).expect("Something went wrong reading the file");
            println!("Path: {}", entry.path().display());
            let xml_path = entry.path().with_extension("xml");
            let xml_content =
                fs::read_to_string(xml_path).expect("Something went wrong reading the file");
            let mbus_data: MBusData = from_str(&xml_content).unwrap();

            println!("{:?}", mbus_data);
            println!("Input:\n{}", contents);

            let contents = contents.trim().replace(' ', "");
            let bytes = hex::decode(contents).unwrap();
            let frame = WiredFrame::try_from(bytes.as_slice()).unwrap();
            if let WiredFrame::LongFrame {
                function: _,
                address: _,
                data,
            } = frame
            {
                let user_data = UserDataBlock::try_from(data).unwrap();
                if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block: _variable_user_data,
                    ..
                } = user_data
                {
                    assert!(
                        Into::<u32>::into(long_tpl_header.identification_number)
                            == mbus_data.slave_information.id.parse::<u32>().unwrap()
                    );
                    let expected_manufacturer = mbus_data
                        .slave_information
                        .manufacturer
                        .unwrap()
                        .into_bytes();
                    let manufacturer = long_tpl_header.manufacturer.unwrap();
                    assert_eq!(manufacturer.code[0], expected_manufacturer[0] as char);
                    assert_eq!(manufacturer.code[1], expected_manufacturer[1] as char);
                    assert_eq!(manufacturer.code[2], expected_manufacturer[2] as char);
                    assert_eq!(
                        long_tpl_header.short_tpl_header.access_number,
                        mbus_data.slave_information.access_number as u8
                    );
                    assert_eq!(
                        long_tpl_header.short_tpl_header.status.bits(),
                        u8::from_str_radix(&mbus_data.slave_information.status, 16).unwrap()
                    );
                    assert_eq!(
                        long_tpl_header.short_tpl_header.configuration_field.raw(),
                        u16::from_str_radix(
                            mbus_data.slave_information.signature.unwrap().as_str(),
                            16
                        )
                        .unwrap()
                    );
                    assert_eq!(
                        long_tpl_header.version,
                        mbus_data.slave_information.version.unwrap()
                    );
                    //  TODO: fix this:
                    //let data_record = DataRecords::try_from(variable_user_data);
                    //println!("{:#?}", data_record);
                }
            } else {
                panic!("Frame is not a long frame");
            }
        }
    }
}
