//! Brief summary
//! * is a library for parsing M-Bus frames and user data.
//! * aims to be a modern, open source decoder for wired m-bus protocol decoder for EN 13757-2 physical and link layer, EN 13757-3 application layer of m-bus
//! * was implemented using the publicly available documentation available at <https://m-bus.com/>
//! # Example
//! ```rust
//! use m_bus_parser::frames::{ Address, Frame, Function };
//! use m_bus_parser::user_data::{ DataRecords, UserDataBlock };
//! use std::io;
//!
//!     let example = vec![
//!         0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01,
//!         0x00, 0x00, 0x00, 0x96, 0x15, 0x01, 0x00, 0x18,
//!         0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00,
//!         0x00, 0x01, 0xFD, 0x1B, 0x00, 0x02, 0xFC, 0x03,
//!         0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC,
//!         0x03, 0x48, 0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12,
//!         0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11,
//!         0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09,
//!         0x12, 0x65, 0xB7, 0x09, 0x01, 0x72, 0x00, 0x72,
//!         0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00,
//!         0x1F, 0xB3, 0x16
//!     ];
//!
//!     // Parse the frame
//!     let frame = Frame::try_from(example.as_slice()).unwrap();
//!
//!     if let Frame::LongFrame { function, address, data } = frame {
//!         assert_eq!(function, Function::RspUd { acd: false, dfc: false });
//!         assert_eq!(address, Address::Primary(1));
//!         if let Ok(UserDataBlock::VariableDataStructure { fixed_data_header, variable_data_block }) = UserDataBlock::try_from(data) {
//!             let data_records = DataRecords::try_from(variable_data_block).unwrap();
//!             println!("data_records: {:#?}", data_records);
//!             let data_records = DataRecords::try_from(variable_data_block).unwrap();
//!         }
//!     }
//!
//!     // Parse everything at once
//!     let parsed_data = m_bus_parser::MbusData::try_from(example.as_slice()).unwrap();
//!     println!("parsed_data: {:#?}", parsed_data);
//!
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use frames::FrameError;
use user_data::ApplicationLayerError;

#[cfg(feature = "std")]
use prettytable::{format, row, Table};

#[cfg(feature = "std")]
use std::str;

pub mod frames;
pub mod user_data;

#[derive(Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(bound(deserialize = "'de: 'a"))
)]
pub struct MbusData<'a> {
    pub frame: frames::Frame<'a>,
    pub user_data: Option<user_data::UserDataBlock<'a>>,
    pub data_records: Option<user_data::DataRecords<'a>>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MbusError {
    FrameError(FrameError),
    ApplicationLayerError(ApplicationLayerError),
}

impl From<FrameError> for MbusError {
    fn from(error: FrameError) -> Self {
        Self::FrameError(error)
    }
}

impl From<ApplicationLayerError> for MbusError {
    fn from(error: ApplicationLayerError) -> Self {
        Self::ApplicationLayerError(error)
    }
}

impl<'a> TryFrom<&'a [u8]> for MbusData<'a> {
    type Error = MbusError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let frame = frames::Frame::try_from(data)?;
        let mut user_data = None;
        let mut data_records = None;
        match &frame {
            frames::Frame::LongFrame { data, .. } => {
                if let Ok(x) = user_data::UserDataBlock::try_from(*data) {
                    user_data = Some(x);
                    if let Ok(user_data::UserDataBlock::VariableDataStructure {
                        fixed_data_header: _,
                        variable_data_block,
                    }) = user_data::UserDataBlock::try_from(*data)
                    {
                        data_records = Some(variable_data_block.into());
                    }
                }
            }
            frames::Frame::SingleCharacter { .. } => (),
            frames::Frame::ShortFrame { .. } => (),
            frames::Frame::ControlFrame { .. } => (),
        };

        Ok(MbusData {
            frame,
            user_data,
            data_records,
        })
    }
}

#[cfg(feature = "std")]
fn clean_and_convert(input: &str) -> Vec<u8> {
    let input = input.trim();
    let cleaned_data: String = input.replace("0x", "").replace([' ', ',', 'x'], "");

    cleaned_data
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let byte_str = str::from_utf8(chunk).unwrap_or_default();
            u8::from_str_radix(byte_str, 16).unwrap_or_default()
        })
        .collect()
}

#[cfg(feature = "std")]
pub fn serialize_mbus_data(data: &str, format: &str) -> String {
    match format {
        "json" => parse_to_json(data),
        "yaml" => parse_to_yaml(data),
        _ => parse_to_table(data).to_string(),
    }
}

#[cfg(feature = "std")]
fn parse_to_json(input: &str) -> std::string::String {
    let data = clean_and_convert(input);
    let parsed_data = MbusData::try_from(data.as_slice());

    serde_json::to_string_pretty(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
fn parse_to_yaml(input: &str) -> std::string::String {
    let data = clean_and_convert(input);
    let parsed_data = MbusData::try_from(data.as_slice());

    serde_yaml::to_string(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
fn parse_to_table(input: &str) -> std::string::String {
    use user_data::UserDataBlock;

    let data = clean_and_convert(input);

    let mut table_output = String::new();
    if let Ok(parsed_data) = MbusData::try_from(data.as_slice()) {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

        match parsed_data.frame {
            frames::Frame::LongFrame {
                function,
                address,
                data: _,
            } => {
                table_output.push_str("Long Frame \n");

                table.set_titles(row!["Function", "Address"]);
                table.add_row(row![function, address]);

                table_output.push_str(&table.to_string());
                table = Table::new();

                match parsed_data.user_data {
                    Some(UserDataBlock::VariableDataStructure {
                        fixed_data_header,
                        variable_data_block: _,
                    }) => {
                        table.add_row(row![
                            fixed_data_header.identification_number,
                            fixed_data_header.manufacturer,
                            fixed_data_header.access_number,
                            fixed_data_header.status,
                            fixed_data_header.signature,
                            fixed_data_header.version,
                            fixed_data_header.medium,
                        ]);

                        table.set_titles(row![
                            "Identification Number",
                            "Manufacturer",
                            "Access Number",
                            "Status",
                            "Signature",
                            "Version",
                            "Medium",
                        ]);
                    }
                    Some(UserDataBlock::FixedDataStructure {
                        identification_number,
                        access_number,
                        status,
                        medium_ad_unit,
                        counter1,
                        counter2,
                    }) => {
                        table.set_titles(row![
                            "Identification Number",
                            "Access Number",
                            "Status",
                            "Medium Ad Unit",
                            "Counter 1",
                            "Counter 2",
                        ]);
                        table.add_row(row![
                            identification_number,
                            access_number,
                            status,
                            medium_ad_unit,
                            counter1,
                            counter2,
                        ]);
                    }
                    Some(UserDataBlock::ResetAtApplicationLevel { subcode }) => {
                        table.set_titles(row!["Function", "Address", "Subcode"]);
                        table.add_row(row![function, address, subcode]);
                    }
                    None => {
                        table.set_titles(row!["Function", "Address"]);
                        table.add_row(row![function, address]);
                    }
                }

                table_output.push_str(&table.to_string());
                table = Table::new();

                table.set_titles(row!["Value", "Data Information",]);

                if let Some(data_records) = parsed_data.data_records {
                    for record in data_records.flatten() {
                        table.add_row(row![
                            format!(
                                "({}{}",
                                record.data,
                                record
                                    .data_record_header
                                    .processed_data_record_header
                                    .value_information
                            ),
                            record
                                .data_record_header
                                .processed_data_record_header
                                .data_information
                        ]);
                    }
                }
            }
            frames::Frame::ShortFrame { .. } => {
                table_output.push_str("Short Frame\n");
            }
            frames::Frame::SingleCharacter { .. } => {
                table_output.push_str("Single Character Frame\n");
            }
            frames::Frame::ControlFrame { .. } => {
                table_output.push_str("Control Frame\n");
            }
        }

        table_output.push_str(&table.to_string());
        table_output
    } else {
        "Error parsing data".to_string()
    }
}
