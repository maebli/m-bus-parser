#[cfg(feature = "std")]
use prettytable::{format, row, Table};

use crate::user_data;
use crate::MbusError;
use wired_mbus_link_layer as frames;
use wireless_mbus_link_layer;

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(bound(deserialize = "'de: 'a"))
)]
#[derive(Debug)]
pub struct MbusData<'a, F> {
    pub frame: F,
    pub user_data: Option<user_data::UserDataBlock<'a>>,
    pub data_records: Option<user_data::DataRecords<'a>>,
}

impl<'a> TryFrom<&'a [u8]> for MbusData<'a, frames::WiredFrame<'a>> {
    type Error = MbusError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let frame = frames::WiredFrame::try_from(data)?;
        let mut user_data = None;
        let mut data_records = None;
        match &frame {
            frames::WiredFrame::LongFrame { data, .. } => {
                if let Ok(x) = user_data::UserDataBlock::try_from(*data) {
                    user_data = Some(x);
                    if let Ok(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                        long_tpl_header,
                        variable_data_block,
                    }) = user_data::UserDataBlock::try_from(*data)
                    {
                        data_records = Some(variable_data_block.into());
                    }
                }
            }
            frames::WiredFrame::SingleCharacter { .. } => (),
            frames::WiredFrame::ShortFrame { .. } => (),
            frames::WiredFrame::ControlFrame { .. } => (),
            _ => (),
        };

        Ok(MbusData {
            frame,
            user_data,
            data_records,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for MbusData<'a, wireless_mbus_link_layer::WirelessFrame<'a>> {
    type Error = MbusError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let frame = wireless_mbus_link_layer::WirelessFrame::try_from(data)?;
        let mut user_data = None;
        let mut data_records = None;

        // Extract application layer data from wireless frame
        match &frame {
            wireless_mbus_link_layer::WirelessFrame::FormatA { data, .. } => {
                if let Ok(x) = user_data::UserDataBlock::try_from(*data) {
                    user_data = Some(x);
                    match user_data::UserDataBlock::try_from(*data) {
                        Ok(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                            long_tpl_header,
                            variable_data_block,
                        }) => {
                            data_records = Some(variable_data_block.into());
                        }
                        Ok(user_data::UserDataBlock::VariableDataStructureWithShortTplHeader {
                            short_tpl_header,
                            variable_data_block,
                        }) => {
                            data_records = Some(variable_data_block.into());
                        }
                        _ => (),
                    }
                }
            }
            wireless_mbus_link_layer::WirelessFrame::FormatB { data, .. } => {
                if let Ok(x) = user_data::UserDataBlock::try_from(*data) {
                    user_data = Some(x);
                    if let Ok(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                        long_tpl_header,
                        variable_data_block,
                    }) = user_data::UserDataBlock::try_from(*data)
                    {
                        data_records = Some(variable_data_block.into());
                    }
                }
            }
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
#[must_use]
pub fn serialize_mbus_data(data: &str, format: &str) -> String {
    match format {
        "json" => parse_to_json(data),
        "yaml" => parse_to_yaml(data),
        "csv" => parse_to_csv(data).to_string(),
        _ => parse_to_table(data).to_string(),
    }
}

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_json(input: &str) -> String {
    let data = clean_and_convert(input);

    // Try wired first
    if let Ok(parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        return serde_json::to_string_pretty(&parsed_data)
            .unwrap_or_default()
            .to_string();
    }

    // If wired fails, try wireless
    let parsed_data =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(data.as_slice());
    serde_json::to_string_pretty(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_yaml(input: &str) -> String {
    let data = clean_and_convert(input);

    // Try wired first
    if let Ok(parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        return serde_yaml::to_string(&parsed_data)
            .unwrap_or_default()
            .to_string();
    }

    // If wired fails, try wireless
    let parsed_data =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(data.as_slice());
    serde_yaml::to_string(&parsed_data)
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_table(input: &str) -> String {
    use user_data::UserDataBlock;

    let data = clean_and_convert(input);

    let mut table_output = String::new();

    // Try wired first
    if let Ok(parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        match parsed_data.frame {
            frames::WiredFrame::LongFrame {
                function,
                address,
                data: _,
            } => {
                table_output.push_str("Long Frame \n");

                table.set_titles(row!["Function", "Address"]);
                table.add_row(row![function, address]);

                table_output.push_str(&table.to_string());

                if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block: _,
                }) = &parsed_data.user_data
                {
                    let mut info_table = Table::new();
                    info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                    info_table.set_titles(row!["Field", "Value"]);
                    info_table.add_row(row![
                        "Identification Number",
                        long_tpl_header.identification_number
                    ]);
                    info_table.add_row(row![
                        "Manufacturer",
                        long_tpl_header
                            .manufacturer
                            .as_ref()
                            .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m))
                    ]);
                    info_table.add_row(row![
                        "Access Number",
                        long_tpl_header.short_tpl_header.access_number
                    ]);
                    info_table.add_row(row!["Status", long_tpl_header.short_tpl_header.status]);
                    info_table.add_row(row![
                        "Security Mode",
                        long_tpl_header
                            .short_tpl_header
                            .configuration_field
                            .security_mode()
                    ]);
                    info_table.add_row(row!["Version", long_tpl_header.version]);
                    info_table.add_row(row!["DeviceType", long_tpl_header.device_type]);
                    table_output.push_str(&info_table.to_string());
                }

                let mut value_table = Table::new();
                value_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                value_table.set_titles(row!["Value", "Data Information", "Header Hex", "Data Hex"]);
                if let Some(data_records) = parsed_data.data_records {
                    for record in data_records.flatten() {
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        value_table.add_row(row![
                            format!("({}{}", record.data, value_information),
                            data_information,
                            record.data_record_header_hex(),
                            record.data_hex()
                        ]);
                    }
                }
                table_output.push_str(&value_table.to_string());
            }
            frames::WiredFrame::ShortFrame { .. } => {
                table_output.push_str("Short Frame\n");
            }
            frames::WiredFrame::SingleCharacter { .. } => {
                table_output.push_str("Single Character Frame\n");
            }
            frames::WiredFrame::ControlFrame { .. } => {
                table_output.push_str("Control Frame\n");
            }
            _ => {
                table_output.push_str("Unknown Frame\n");
            }
        }
        return table_output;
    }

    // If wired fails, try wireless
    if let Ok(parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(data.as_slice())
    {
        match &parsed_data.frame {
            wireless_mbus_link_layer::WirelessFrame::FormatA {
                function,
                manufacturer_id,
                data: _,
            } => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_BOX_CHARS);
                table.set_titles(row!["Field", "Value"]);
                table.add_row(row!["Function", format!("{:?}", function)]);
                table.add_row(row![
                    "Manufacturer Code",
                    format!("{:?}", manufacturer_id.manufacturer_code)
                ]);
                table.add_row(row![
                    "Identification Number",
                    format!("{:?}", manufacturer_id.identification_number)
                ]);
                table.add_row(row![
                    "Device Type",
                    format!("{:?}", manufacturer_id.device_type)
                ]);
                table.add_row(row!["Version", format!("{:?}", manufacturer_id.version)]);
                table.add_row(row![
                    "Is globally Unique Id",
                    format!("{:?}", manufacturer_id.is_unique_globally)
                ]);
                table_output.push_str(&table.to_string());

                match &parsed_data.user_data {
                    Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                        long_tpl_header,
                        variable_data_block: _,
                    }) => {
                        let mut info_table = Table::new();
                        info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                        info_table.set_titles(row!["Field", "Value"]);
                        info_table.add_row(row![
                            "Identification Number",
                            long_tpl_header.identification_number
                        ]);
                        info_table.add_row(row![
                            "Manufacturer",
                            long_tpl_header
                                .manufacturer
                                .as_ref()
                                .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m))
                        ]);
                        info_table.add_row(row![
                            "Access Number",
                            long_tpl_header.short_tpl_header.access_number
                        ]);
                        info_table.add_row(row!["Status", long_tpl_header.short_tpl_header.status]);
                        info_table.add_row(row![
                            "Security Mode",
                            long_tpl_header
                                .short_tpl_header
                                .configuration_field
                                .security_mode()
                        ]);
                        info_table.add_row(row!["Version", long_tpl_header.version]);
                        info_table.add_row(row!["Device Type", long_tpl_header.device_type]);
                        table_output.push_str(&info_table.to_string());
                    }
                    Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
                        short_tpl_header,
                        variable_data_block,
                    }) => {
                        let mut info_table = Table::new();
                        info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                        info_table.set_titles(row!["Field", "Value"]);
                        info_table.add_row(row!["Access Number", short_tpl_header.access_number]);
                        info_table.add_row(row!["Status", short_tpl_header.status]);
                        info_table.add_row(row![
                            "Security Mode",
                            short_tpl_header.configuration_field.security_mode()
                        ]);
                        table_output.push_str(&info_table.to_string());
                    }
                    _ => (),
                }

                let mut value_table = Table::new();
                value_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                value_table.set_titles(row!["Value", "Data Information", "Header Hex", "Data Hex"]);
                if let Some(data_records) = &parsed_data.data_records {
                    for record in data_records.clone().flatten() {
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        value_table.add_row(row![
                            format!("({}{}", record.data, value_information),
                            data_information,
                            record.data_record_header_hex(),
                            record.data_hex()
                        ]);
                    }
                }
                table_output.push_str(&value_table.to_string());
            }
            wireless_mbus_link_layer::WirelessFrame::FormatB {
                function,
                manufacturer_id,
                data,
            } => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_BOX_CHARS);
                table.set_titles(row!["Field", "Value"]);
                table.add_row(row!["Function", format!("{:?}", function)]);
                table_output.push_str(&table.to_string());

                if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block: _,
                }) = &parsed_data.user_data
                {
                    let mut info_table = Table::new();
                    info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                    info_table.set_titles(row!["Field", "Value"]);
                    info_table.add_row(row![
                        "Identification Number",
                        long_tpl_header.identification_number
                    ]);
                    info_table.add_row(row![
                        "Manufacturer",
                        long_tpl_header
                            .manufacturer
                            .as_ref()
                            .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m))
                    ]);
                    info_table.add_row(row![
                        "Access Number",
                        long_tpl_header.short_tpl_header.access_number
                    ]);
                    info_table.add_row(row!["Status", long_tpl_header.short_tpl_header.status]);
                    info_table.add_row(row![
                        "Security Mode",
                        long_tpl_header
                            .short_tpl_header
                            .configuration_field
                            .security_mode()
                    ]);
                    info_table.add_row(row!["Version", long_tpl_header.version]);
                    info_table.add_row(row!["Device Type", long_tpl_header.device_type]);
                    table_output.push_str(&info_table.to_string());
                }

                let mut value_table = Table::new();
                value_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                value_table.set_titles(row!["Value", "Data Information", "Header Hex", "Data Hex"]);
                if let Some(data_records) = &parsed_data.data_records {
                    for record in data_records.clone().flatten() {
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => "None".to_string(),
                        };
                        value_table.add_row(row![
                            format!("({}{}", record.data, value_information),
                            data_information,
                            record.data_record_header_hex(),
                            record.data_hex()
                        ]);
                    }
                }
                table_output.push_str(&value_table.to_string());
            }
        }
        return table_output;
    }

    // If both fail, return error
    "Error: Could not parse data as wired or wireless M-Bus".to_string()
}

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_csv(input: &str) -> String {
    use crate::user_data::UserDataBlock;
    use prettytable::csv;

    let data = clean_and_convert(input);

    let mut writer = csv::Writer::from_writer(vec![]);

    // Try wired first
    if let Ok(parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        match parsed_data.frame {
            frames::WiredFrame::LongFrame {
                function, address, ..
            } => {
                let data_point_count = parsed_data
                    .data_records
                    .as_ref()
                    .map(|records| records.clone().flatten().count())
                    .unwrap_or(0);

                let mut headers = vec![
                    "FrameType".to_string(),
                    "Function".to_string(),
                    "Address".to_string(),
                    "Identification Number".to_string(),
                    "Manufacturer".to_string(),
                    "Access Number".to_string(),
                    "Status".to_string(),
                    "Security Mode".to_string(),
                    "Version".to_string(),
                    "Device Type".to_string(),
                ];

                for i in 1..=data_point_count {
                    headers.push(format!("DataPoint{}_Value", i));
                    headers.push(format!("DataPoint{}_Info", i));
                }

                let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
                writer
                    .write_record(header_refs)
                    .map_err(|_| ())
                    .unwrap_or_default();

                let mut row = vec![
                    "LongFrame".to_string(),
                    function.to_string(),
                    address.to_string(),
                ];

                match &parsed_data.user_data {
                    Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                        long_tpl_header,
                        ..
                    }) => {
                        row.extend_from_slice(&[
                            long_tpl_header.identification_number.to_string(),
                            long_tpl_header
                                .manufacturer
                                .as_ref()
                                .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m)),
                            long_tpl_header.short_tpl_header.access_number.to_string(),
                            long_tpl_header.short_tpl_header.status.to_string(),
                            long_tpl_header
                                .short_tpl_header
                                .configuration_field
                                .security_mode()
                                .to_string(),
                            long_tpl_header.version.to_string(),
                            long_tpl_header.device_type.to_string(),
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
                            "".to_string(), // Security Mode
                            "".to_string(), // Version
                            "".to_string(), // Device Type
                        ]);
                    }
                    _ => {
                        // Fill with empty strings for header info
                        for _ in 0..7 {
                            row.push("".to_string());
                        }
                    }
                }

                if let Some(data_records) = parsed_data.data_records {
                    for record in data_records.flatten() {
                        // Format the value with units to match the table output
                        let parsed_value = format!("{}", record.data);

                        // Get value information including units
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(x) => format!("{}", x),
                            None => "None".to_string(),
                        };

                        // Format the value similar to the table output with units
                        let formatted_value = format!("({}{}", parsed_value, value_information);

                        let data_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .data_information
                        {
                            Some(x) => format!("{}", x),
                            None => "None".to_string(),
                        };

                        row.push(formatted_value);
                        row.push(data_information);
                    }
                }

                let row_refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
                writer
                    .write_record(row_refs)
                    .map_err(|_| ())
                    .unwrap_or_default();
            }
            _ => {
                writer
                    .write_record(["FrameType"])
                    .map_err(|_| ())
                    .unwrap_or_default();
                writer
                    .write_record([format!("{:?}", parsed_data.frame).as_str()])
                    .map_err(|_| ())
                    .unwrap_or_default();
            }
        }

        let csv_data = writer.into_inner().unwrap_or_default();
        return String::from_utf8(csv_data)
            .unwrap_or_else(|_| "Error converting CSV data to string".to_string());
    }

    // If wired fails, try wireless
    if let Ok(parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(data.as_slice())
    {
        let frame_type = match &parsed_data.frame {
            wireless_mbus_link_layer::WirelessFrame::FormatA { .. } => "WirelessFormatA",
            wireless_mbus_link_layer::WirelessFrame::FormatB { .. } => "WirelessFormatB",
        };

        let data_point_count = parsed_data
            .data_records
            .as_ref()
            .map(|records| records.clone().flatten().count())
            .unwrap_or(0);

        let mut headers = vec![
            "FrameType".to_string(),
            "Identification Number".to_string(),
            "Manufacturer".to_string(),
            "Access Number".to_string(),
            "Status".to_string(),
            "Security Mode".to_string(),
            "Version".to_string(),
            "Device Type".to_string(),
        ];

        for i in 1..=data_point_count {
            headers.push(format!("DataPoint{}_Value", i));
            headers.push(format!("DataPoint{}_Info", i));
        }

        let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
        writer
            .write_record(header_refs)
            .map_err(|_| ())
            .unwrap_or_default();

        let mut row = vec![frame_type.to_string()];

        match &parsed_data.user_data {
            Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                variable_data_block,
            }) => {
                row.extend_from_slice(&[
                    long_tpl_header.identification_number.to_string(),
                    long_tpl_header
                        .manufacturer
                        .as_ref()
                        .map_or_else(|e| format!("Err({:?})", e), |m| format!("{:?}", m)),
                    long_tpl_header.short_tpl_header.access_number.to_string(),
                    long_tpl_header.short_tpl_header.status.to_string(),
                    long_tpl_header
                        .short_tpl_header
                        .configuration_field
                        .security_mode()
                        .to_string(),
                    long_tpl_header.version.to_string(),
                    long_tpl_header.device_type.to_string(),
                ]);
            }
            _ => {
                // Fill with empty strings for header info
                for _ in 0..7 {
                    row.push("".to_string());
                }
            }
        }

        if let Some(data_records) = &parsed_data.data_records {
            for record in data_records.clone().flatten() {
                let parsed_value = format!("{}", record.data);
                let value_information = match record
                    .data_record_header
                    .processed_data_record_header
                    .value_information
                {
                    Some(x) => format!("{}", x),
                    None => "None".to_string(),
                };
                let formatted_value = format!("({}{}", parsed_value, value_information);
                let data_information = match record
                    .data_record_header
                    .processed_data_record_header
                    .data_information
                {
                    Some(x) => format!("{}", x),
                    None => "None".to_string(),
                };
                row.push(formatted_value);
                row.push(data_information);
            }
        }

        let row_refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        writer
            .write_record(row_refs)
            .map_err(|_| ())
            .unwrap_or_default();

        let csv_data = writer.into_inner().unwrap_or_default();
        return String::from_utf8(csv_data)
            .unwrap_or_else(|_| "Error converting CSV data to string".to_string());
    }

    // If both fail, return error
    writer
        .write_record(["Error"])
        .map_err(|_| ())
        .unwrap_or_default();
    writer
        .write_record(["Error parsing data as wired or wireless M-Bus"])
        .map_err(|_| ())
        .unwrap_or_default();

    let csv_data = writer.into_inner().unwrap_or_default();
    String::from_utf8(csv_data)
        .unwrap_or_else(|_| "Error converting CSV data to string".to_string())
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_converter() {
        use super::parse_to_csv;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let csv_output: String = parse_to_csv(input);
        println!("{}", csv_output);
        let yaml_output: String = super::parse_to_yaml(input);
        println!("{}", yaml_output);
        let json_output: String = super::parse_to_json(input);
        println!("{}", json_output);
        let table_output: String = super::parse_to_table(input);
        println!("{}", table_output);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_expected_output() {
        use super::parse_to_csv;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let csv_output = parse_to_csv(input);

        let expected = "FrameType,Function,Address,Identification Number,Manufacturer,Access Number,Status,Security Mode,Version,Device Type,Device Type,DataPoint1_Value,DataPoint1_Info,DataPoint2_Value,DataPoint2_Info,DataPoint3_Value,DataPoint3_Info,DataPoint4_Value,DataPoint4_Info,DataPoint5_Value,DataPoint5_Info,DataPoint6_Value,DataPoint6_Info,DataPoint7_Value,DataPoint7_Info,DataPoint8_Value,DataPoint8_Info,DataPoint9_Value,DataPoint9_Info,DataPoint10_Value,DataPoint10_Info\nLongFrame,\"RspUd (ACD: false, DFC: false)\",Primary (1),02205100,\"ManufacturerCode { code: ['S', 'L', 'B'] }\",0,\"Permanent error, Manufacturer specific 3\",No encryption used,2,Heat,(0))e4[Wh],\"0,Inst,32-bit Integer\",(3))e-1[m³](Volume),\"0,Inst,BCD 8-digit\",(0))e3[W],\"0,Inst,BCD 6-digit\",(0))e-3[m³h⁻¹],\"0,Inst,BCD 6-digit\",(1288))e-1[°C],\"0,Inst,BCD 4-digit\",(516))e-1[°C],\"0,Inst,BCD 4-digit\",(7723))e-2[°K],\"0,Inst,BCD 6-digit\",(12/Jan/12))(Date),\"0,Inst,Date Type G\",(3383))[day],\"0,Inst,16-bit Integer\",\"(Manufacturer Specific: [15, 96, 0])None\",None\n";

        assert_eq!(csv_output, expected);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_yaml_expected_output() {
        use super::parse_to_yaml;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let yaml_output = parse_to_yaml(input);

        // First line of YAML output to test against - we'll test just the beginning to avoid a massive string
        let expected_start = "frame: !LongFrame\n  function: !RspUd\n    acd: false\n    dfc: false\n  address: !Primary 1\nuser_data: !VariableDataStructure\n";

        assert!(yaml_output.starts_with(expected_start));
        // Additional checks for specific content in the YAML
        assert!(yaml_output.contains("device_type: Heat Meter"));
        assert!(yaml_output.contains("identification_number:"));
        assert!(yaml_output.contains("status: PERMANENT_ERROR | MANUFACTURER_SPECIFIC_3"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_expected_output() {
        use super::parse_to_json;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let json_output = parse_to_json(input);

        // Testing specific content in JSON
        assert!(json_output.contains("\"Ok\""));
        assert!(json_output.contains("\"LongFrame\""));
        assert!(json_output.contains("\"RspUd\""));
        assert!(json_output.contains("\"number\": 2205100"));
        assert!(json_output.contains("\"device type\": \"Heat\""));
        assert!(json_output.contains("\"status\": \"PERMANENT_ERROR | MANUFACTURER_SPECIFIC_3\""));

        // Verify JSON structure is valid
        let json_parsed = serde_json::from_str::<serde_json::Value>(&json_output);
        assert!(json_parsed.is_ok());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_table_expected_output() {
        use super::parse_to_table;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let table_output = parse_to_table(input);

        // First section of the table output
        assert!(table_output.starts_with("Long Frame"));

        // Key content pieces to verify
        assert!(table_output.contains("RspUd (ACD: false, DFC: false)"));
        assert!(table_output.contains("Primary (1)"));
        assert!(table_output.contains("Identification Number"));
        assert!(table_output.contains("02205100"));
        assert!(table_output.contains("ManufacturerCode { code: ['S', 'L', 'B'] }"));

        // Data point verifications
        assert!(table_output.contains("(0)e4[Wh]"));
        assert!(table_output.contains("(3)e-1[m³](Volume)"));
        assert!(table_output.contains("(1288)e-1[°C]"));
        assert!(table_output.contains("(12/Jan/12)(Date)"));
        assert!(table_output.contains("(3383)[day]"));
    }
}
