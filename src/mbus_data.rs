#[cfg(feature = "std")]
use prettytable::{csv::Writer, format, row, Table};

use crate::frames;
use crate::user_data;
use crate::MbusError;
use serde::Deserialize;
#[derive(Debug)]
pub struct MbusData<'a> {
    pub frame: frames::Frame<'a>,
    pub user_data: Option<user_data::UserDataBlock<'a>>,
    pub data_records: Option<user_data::DataRecords<'a>>,
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
    use core::str;
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
        "csv" => parse_to_csv(data).to_string(),
        _ => parse_to_table(data).to_string(),
    }
}

#[cfg(feature = "std")]
pub fn parse_to_json(input: &str) -> String {
    let data = clean_and_convert(input);
    let parsed_data = MbusData::try_from(data.as_slice());

    serde_json::to_string_pretty(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
fn parse_to_yaml(input: &str) -> String {
    let data = clean_and_convert(input);
    let parsed_data = MbusData::try_from(data.as_slice());

    serde_yaml::to_string(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
fn parse_to_table(input: &str) -> String {
    use user_data::UserDataBlock;

    let data = clean_and_convert(input);

    let mut table_output = String::new();
    let parsed_data_result = MbusData::try_from(data.as_slice());
    if let Ok(parsed_data) = parsed_data_result {
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
        format!("Error {:?} parsing data", parsed_data_result)
    }
}

#[cfg(feature = "std")]
pub fn parse_to_csv(input: &str) -> String {
    let data = clean_and_convert(input);
    let _parsed_data = MbusData::try_from(data.as_slice());
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row![
        "FrameType",
        "Function",
        "Address",
        "IdentificationNumber",
        "Manufacturer",
        "AccessNumber",
        "Status",
        "Signature",
        "Version",
        "Medium",
        "MediumAdUnit",
        "Counter1",
        "Counter2",
        "Subcode",
        "Value",
        "DataInformation"
    ]);

    let writer = Writer::from_writer(vec![]);
    let x = table.to_csv_writer(writer).unwrap();
    String::from_utf8(x.into_inner().unwrap()).unwrap()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_converter() {
        use super::parse_to_csv;
        let x = "0000";
        let y = parse_to_csv(x);
        println!("{}", y);
    }
}
