#[cfg(feature = "std")]
use prettytable::{csv::Writer, format, row, Table};

use crate::frames;
use crate::user_data;
use crate::MbusError;

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(bound(deserialize = "'de: 'a"))
)]
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
                            fixed_data_header
                                .manufacturer
                                .as_ref()
                                .map_or_else(|e| format!("Err({:?})", e), |m| format!("{:?}", m)),
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
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(x) => format!("{}", x),
                            None => "None".to_string(),
                        };

                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(x) => format!("{}", x),
                            None => "None".to_string(),
                        };

                        table.add_row(row![
                            format!("({}{}", record.data, value_information),
                            format!("{}", data_information)
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
    use crate::user_data::UserDataBlock;
    use prettytable::csv;

    let data = clean_and_convert(input);
    let parsed_data = MbusData::try_from(data.as_slice());

    // CSV writer using a vector as an intermediate buffer
    let mut writer = csv::Writer::from_writer(vec![]);

    if let Ok(parsed_data) = parsed_data {
        match parsed_data.frame {
            frames::Frame::LongFrame {
                function, address, ..
            } => {
                // Count how many data points we have
                let data_point_count = parsed_data
                    .data_records
                    .as_ref()
                    .map(|records| records.clone().flatten().count())
                    .unwrap_or(0);

                // Create headers as owned strings
                let mut headers = vec![
                    "FrameType".to_string(),
                    "Function".to_string(),
                    "Address".to_string(),
                    "IdentificationNumber".to_string(),
                    "Manufacturer".to_string(),
                    "AccessNumber".to_string(),
                    "Status".to_string(),
                    "Signature".to_string(),
                    "Version".to_string(),
                    "Medium".to_string(),
                ];

                // Add headers for each data point
                for i in 1..=data_point_count {
                    headers.push(format!("DataPoint{}_Value", i));
                    headers.push(format!("DataPoint{}_Info", i));
                }

                // Convert Vec<String> to Vec<&str> for write_record
                let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
                writer.write_record(&header_refs).unwrap();

                // Create data row
                let mut row = vec![
                    "LongFrame".to_string(),
                    function.to_string(),
                    address.to_string(),
                ];

                // Add header info
                match &parsed_data.user_data {
                    Some(UserDataBlock::VariableDataStructure {
                        fixed_data_header, ..
                    }) => {
                        row.extend_from_slice(&[
                            fixed_data_header.identification_number.to_string(),
                            fixed_data_header
                                .manufacturer
                                .as_ref()
                                .map_or_else(|e| format!("Err({:?})", e), |m| format!("{:?}", m)),
                            fixed_data_header.access_number.to_string(),
                            fixed_data_header.status.to_string(),
                            fixed_data_header.signature.to_string(),
                            fixed_data_header.version.to_string(),
                            fixed_data_header.medium.to_string(),
                        ]);
                    }
                    Some(UserDataBlock::FixedDataStructure {
                        identification_number,
                        access_number,
                        status,
                        ..
                    }) => {
                        row.extend_from_slice(&[
                            identification_number.to_string(),
                            "".to_string(), // Manufacturer
                            access_number.to_string(),
                            status.to_string(),
                            "".to_string(), // Signature
                            "".to_string(), // Version
                            "".to_string(), // Medium
                        ]);
                    }
                    _ => {
                        // Fill with empty strings for header info
                        for _ in 0..7 {
                            row.push("".to_string());
                        }
                    }
                }

                // Add data points
                if let Some(data_records) = parsed_data.data_records {
                    for record in data_records.flatten() {
                        // Get the parsed value
                        let parsed_value = format!("{}", record.data);

                        // Get data information
                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(x) => format!("{}", x),
                            None => "None".to_string(),
                        };

                        // Add value and info to the single row
                        row.push(parsed_value);
                        row.push(data_information);
                    }
                }

                // Convert Vec<String> to Vec<&str> for write_record
                let row_refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
                writer.write_record(&row_refs).unwrap();
            }
            _ => {
                // For other frame types, just output a simple header and row
                writer.write_record(&["FrameType"]).unwrap();
                writer
                    .write_record(&[format!("{:?}", parsed_data.frame).as_str()])
                    .unwrap();
            }
        }
    } else {
        // Error case
        writer.write_record(&["Error"]).unwrap();
        writer.write_record(&["Error parsing data"]).unwrap();
    }

    // Convert CSV to a string and return it
    let csv_data = writer.into_inner().unwrap();
    String::from_utf8(csv_data).unwrap()
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_converter() {
        use super::parse_to_csv;
        let x = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let y = parse_to_csv(x);
        println!("{}", y);
    }
}
