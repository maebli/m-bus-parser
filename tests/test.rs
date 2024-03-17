use hex;
use m_bus_parser::user_data::Medium;
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

fn medium_to_str(medium: Medium) -> &'static str {
    match medium {
        Medium::Other => "Other",
        Medium::Oil => "Oil",
        Medium::Electricity => "Electricity",
        Medium::Gas => "Gas",
        Medium::Heat => "Heat: Outlet",
        Medium::Steam => "Steam",
        Medium::HotWater => "Warm water (30-90Â°C)",
        Medium::Water => "Water",
        Medium::HeatCostAllocator => "Heat Cost Allocator",
        Medium::Unknown => "Unknown",
        Medium::Reserved => "Breaker: Electricity",
        Medium::GasMode2 => "GasMode2",
        Medium::HeatMode2 => "HeatMode2",
        Medium::HotWaterMode2 => "Heat: Inlet",
        Medium::WaterMode2 => "Heat / Cooling load meter",
        Medium::HeatCostAllocator2 => "Bus/System",
        Medium::ReservedMode2 => "ReservedMode2",
        Medium::ColdWater => "Cold water",
        Medium::DualWater => "DualWater",
        Medium::Pressure => "Pressure",
        Medium::ADConverter => "ADConverter",
    }
}
#[cfg(test)]
mod tests {

    use m_bus_parser::{frames::Frame, user_data::UserDataBlock};

    use super::*;

    #[test]
    fn test_valid_control_frame_parsing() {
        /* parses all the good examples, shouldn't throw any errors. */
        for entry in WalkDir::new("./tests/rscada/test-frames")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "hex"))
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

            let contents = contents.trim().replace(" ", "");
            let bytes = hex::decode(contents).unwrap();
            let frame = Frame::try_from(bytes.as_slice()).unwrap();
            if let Frame::LongFrame {
                function: _,
                address: _,
                data,
            } = frame
            {
                let user_data = UserDataBlock::try_from(data).unwrap();
                if let UserDataBlock::VariableDataStructure {
                    fixed_data_header,
                    variable_data_block: _,
                } = user_data
                {
                    assert!(
                        Into::<u32>::into(fixed_data_header.identification_number)
                            == mbus_data.slave_information.id.parse::<u32>().unwrap()
                    );
                    let expected_manufacturer = mbus_data
                        .slave_information
                        .manufacturer
                        .unwrap()
                        .into_bytes();
                    assert_eq!(
                        fixed_data_header.manufacturer.code[0],
                        expected_manufacturer[0] as char
                    );
                    assert_eq!(
                        fixed_data_header.manufacturer.code[1],
                        expected_manufacturer[1] as char
                    );
                    assert_eq!(
                        fixed_data_header.manufacturer.code[2],
                        expected_manufacturer[2] as char
                    );
                    assert_eq!(
                        fixed_data_header.access_number,
                        mbus_data.slave_information.access_number as u8
                    );
                    assert_eq!(
                        fixed_data_header.status.bits(),
                        u8::from_str_radix(&mbus_data.slave_information.status, 16).unwrap()
                    );
                    assert_eq!(
                        fixed_data_header.signature,
                        u16::from_str_radix(
                            mbus_data.slave_information.signature.unwrap().as_str(),
                            16
                        )
                        .unwrap()
                    );
                    assert_eq!(
                        fixed_data_header.version,
                        mbus_data.slave_information.version.unwrap()
                    );
                    assert_eq!(
                        medium_to_str(fixed_data_header.medium),
                        mbus_data.slave_information._medium
                    );
                }
            } else {
                panic!("Frame is not a long frame");
            }
        }
    }
}
