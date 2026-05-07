#[cfg(feature = "std")]
use prettytable::{format, row, Table};
use wireless_mbus_link_layer::WirelessFrame;

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
                        long_tpl_header: _,
                        variable_data_block,
                        ..
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

impl<'a> TryFrom<&'a [u8]> for MbusData<'a, WirelessFrame<'a>> {
    type Error = MbusError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let frame = wireless_mbus_link_layer::WirelessFrame::try_from(data)?;
        let mut user_data = None;
        let mut data_records = None;
        // Extract application layer data from wireless frame
        let wireless_mbus_link_layer::WirelessFrame { data, .. } = &frame;

        if let Ok(user_data_block) = user_data::UserDataBlock::try_from(*data) {
            match &user_data_block {
                user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                    variable_data_block,
                    ..
                } => {
                    data_records = Some((*variable_data_block).into());
                }
                user_data::UserDataBlock::VariableDataStructureWithShortTplHeader {
                    variable_data_block,
                    ..
                } => {
                    data_records = Some((*variable_data_block).into());
                }
                _ => {}
            }
            user_data = Some(user_data_block);
        }

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
pub fn serialize_mbus_data(data: &str, format: &str, key: Option<&[u8; 16]>) -> String {
    match format {
        "json" => parse_to_json(data, key),
        "yaml" => parse_to_yaml(data, key),
        "csv" => parse_to_csv(data, key).to_string(),
        "mermaid" => parse_to_mermaid(data, key),
        _ => parse_to_table(data, key).to_string(),
    }
}

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_json(input: &str, key: Option<&[u8; 16]>) -> String {
    use user_data::UserDataBlock;

    let data = clean_and_convert(input);
    // Buffer for decrypted data - M-Bus user data max ~252 bytes, 256 is safe
    let mut decrypted_buffer = [0u8; 256];
    let mut decrypted_len = 0usize;

    // Try wired first
    if let Ok(mut parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        #[cfg(feature = "decryption")]
        if let Some(key_bytes) = key {
            if let Some(user_data) = &parsed_data.user_data {
                if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    ..
                } = user_data
                {
                    if long_tpl_header.is_encrypted() {
                        if let Ok(mfr) = &long_tpl_header.manufacturer {
                            let mut provider = crate::decryption::StaticKeyProvider::<1>::new();
                            let mfr_id = mfr.to_id();
                            let id_num = long_tpl_header.identification_number.number;
                            let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                            if let Ok(len) =
                                user_data.decrypt_variable_data(&provider, &mut decrypted_buffer)
                            {
                                decrypted_len = len;
                            }
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "decryption"))]
        let _ = key;

        // Apply decrypted data records if decryption succeeded
        #[cfg(feature = "decryption")]
        if decrypted_len > 0 {
            if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                ..
            }) = &parsed_data.user_data
            {
                if let Some(decrypted_data) = decrypted_buffer.get(..decrypted_len) {
                    parsed_data.data_records = Some(user_data::DataRecords::new(
                        decrypted_data,
                        Some(long_tpl_header),
                    ));
                }
            }
        }

        let mfr_code_str = parsed_data.user_data.as_ref().and_then(|ud| {
            if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header, ..
            } = ud
            {
                long_tpl_header
                    .manufacturer
                    .as_ref()
                    .ok()
                    .map(|m| format!("{}", m))
            } else {
                None
            }
        });
        let mut json_val = serde_json::to_value(&parsed_data).unwrap_or_default();
        if let (Some(code), serde_json::Value::Object(ref mut map)) = (mfr_code_str, &mut json_val)
        {
            if let Some(info) = crate::manufacturers::lookup_manufacturer(&code) {
                map.insert(
                    "manufacturer_info".to_string(),
                    serde_json::json!({
                        "name": info.name,
                        "website": info.website,
                        "description": info.description,
                    }),
                );
            }
        }
        return serde_json::to_string_pretty(&json_val)
            .unwrap_or_default()
            .to_string();
    }

    // If wired fails, try wireless - strip Format A CRCs if present
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(mut parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        #[cfg(feature = "decryption")]
        {
            let mut long_header_for_records: Option<&user_data::LongTplHeader> = None;
            if let Some(key_bytes) = key {
                let manufacturer_id = &parsed_data.frame.manufacturer_id;
                if let Some(user_data) = &parsed_data.user_data {
                    let (is_encrypted, long_header) = match user_data {
                        UserDataBlock::VariableDataStructureWithLongTplHeader {
                            long_tpl_header,
                            ..
                        } => (long_tpl_header.is_encrypted(), Some(long_tpl_header)),
                        UserDataBlock::VariableDataStructureWithShortTplHeader {
                            short_tpl_header,
                            ..
                        } => (short_tpl_header.is_encrypted(), None),
                        _ => (false, None),
                    };
                    long_header_for_records = long_header;

                    if is_encrypted {
                        let mut provider = crate::decryption::StaticKeyProvider::<1>::new();

                        let decrypt_result = match user_data {
                            UserDataBlock::VariableDataStructureWithLongTplHeader {
                                long_tpl_header,
                                ..
                            } => {
                                if let Ok(mfr) = &long_tpl_header.manufacturer {
                                    let mfr_id = mfr.to_id();
                                    let id_num = long_tpl_header.identification_number.number;
                                    let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                    user_data
                                        .decrypt_variable_data(&provider, &mut decrypted_buffer)
                                } else {
                                    Err(crate::decryption::DecryptionError::DecryptionFailed)
                                }
                            }
                            UserDataBlock::VariableDataStructureWithShortTplHeader { .. } => {
                                let mfr_id = manufacturer_id.manufacturer_code.to_id();
                                let id_num = manufacturer_id.identification_number.number;
                                let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                user_data.decrypt_variable_data_with_context(
                                    &provider,
                                    manufacturer_id.manufacturer_code,
                                    id_num,
                                    manufacturer_id.version,
                                    manufacturer_id.device_type,
                                    &mut decrypted_buffer,
                                )
                            }
                            _ => Err(crate::decryption::DecryptionError::UnknownEncryptionState),
                        };

                        if let Ok(len) = decrypt_result {
                            decrypted_len = len;
                        }
                    }
                }
            }

            // Apply decrypted data records if decryption succeeded
            if let Some(decrypted_data) = decrypted_buffer.get(..decrypted_len) {
                if !decrypted_data.is_empty() {
                    parsed_data.data_records = Some(user_data::DataRecords::new(
                        decrypted_data,
                        long_header_for_records,
                    ));
                }
            }
        }
        #[cfg(not(feature = "decryption"))]
        let _ = key;

        let mfr_code_str = format!("{}", parsed_data.frame.manufacturer_id.manufacturer_code);
        let mut json_val = serde_json::to_value(&parsed_data).unwrap_or_default();
        if let serde_json::Value::Object(ref mut map) = json_val {
            if let Some(info) = crate::manufacturers::lookup_manufacturer(&mfr_code_str) {
                map.insert(
                    "manufacturer_info".to_string(),
                    serde_json::json!({
                        "name": info.name,
                        "website": info.website,
                        "description": info.description,
                    }),
                );
            }
        }
        return serde_json::to_string_pretty(&json_val)
            .unwrap_or_default()
            .to_string();
    }

    // If both fail, return error
    "{}".to_string()
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_yaml(input: &str, key: Option<&[u8; 16]>) -> String {
    use user_data::UserDataBlock;

    let data = clean_and_convert(input);
    // Buffer for decrypted data - must live as long as data_records
    let mut decrypted_buffer = [0u8; 256];
    let mut decrypted_len = 0usize;

    // Try wired first
    if let Ok(mut parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        #[cfg(feature = "decryption")]
        if let Some(key_bytes) = key {
            if let Some(user_data) = &parsed_data.user_data {
                if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    ..
                } = user_data
                {
                    if long_tpl_header.is_encrypted() {
                        if let Ok(mfr) = &long_tpl_header.manufacturer {
                            let mut provider = crate::decryption::StaticKeyProvider::<1>::new();
                            let mfr_id = mfr.to_id();
                            let id_num = long_tpl_header.identification_number.number;
                            let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                            if let Ok(len) =
                                user_data.decrypt_variable_data(&provider, &mut decrypted_buffer)
                            {
                                decrypted_len = len;
                            }
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "decryption"))]
        let _ = key;

        // Apply decrypted data records if decryption succeeded
        #[cfg(feature = "decryption")]
        if decrypted_len > 0 {
            if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                ..
            }) = &parsed_data.user_data
            {
                let decrypted_data = decrypted_buffer.get(..decrypted_len).unwrap_or(&[]);
                parsed_data.data_records = Some(user_data::DataRecords::new(
                    decrypted_data,
                    Some(long_tpl_header),
                ));
            }
        }

        let mfr_code_str = parsed_data.user_data.as_ref().and_then(|ud| {
            if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header, ..
            } = ud
            {
                long_tpl_header
                    .manufacturer
                    .as_ref()
                    .ok()
                    .map(|m| format!("{}", m))
            } else {
                None
            }
        });
        let base = serde_yaml::to_string(&parsed_data).unwrap_or_default();
        return if let Some(code) = mfr_code_str {
            if let Some(info) = crate::manufacturers::lookup_manufacturer(&code) {
                format!(
                    "{}manufacturer_info:\n  name: {}\n  website: {}\n  description: {}\n",
                    base, info.name, info.website, info.description
                )
            } else {
                base
            }
        } else {
            base
        };
    }

    // If wired fails, try wireless - strip Format A CRCs if present
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(mut parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        #[cfg(feature = "decryption")]
        {
            let mut long_header_for_records: Option<&user_data::LongTplHeader> = None;
            if let Some(key_bytes) = key {
                let manufacturer_id = &parsed_data.frame.manufacturer_id;
                if let Some(user_data) = &parsed_data.user_data {
                    let (is_encrypted, long_header) = match user_data {
                        UserDataBlock::VariableDataStructureWithLongTplHeader {
                            long_tpl_header,
                            ..
                        } => (long_tpl_header.is_encrypted(), Some(long_tpl_header)),
                        UserDataBlock::VariableDataStructureWithShortTplHeader {
                            short_tpl_header,
                            ..
                        } => (short_tpl_header.is_encrypted(), None),
                        _ => (false, None),
                    };
                    long_header_for_records = long_header;

                    if is_encrypted {
                        let mut provider = crate::decryption::StaticKeyProvider::<1>::new();

                        let decrypt_result = match user_data {
                            UserDataBlock::VariableDataStructureWithLongTplHeader {
                                long_tpl_header,
                                ..
                            } => {
                                if let Ok(mfr) = &long_tpl_header.manufacturer {
                                    let mfr_id = mfr.to_id();
                                    let id_num = long_tpl_header.identification_number.number;
                                    let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                    user_data
                                        .decrypt_variable_data(&provider, &mut decrypted_buffer)
                                } else {
                                    Err(crate::decryption::DecryptionError::DecryptionFailed)
                                }
                            }
                            UserDataBlock::VariableDataStructureWithShortTplHeader { .. } => {
                                let mfr_id = manufacturer_id.manufacturer_code.to_id();
                                let id_num = manufacturer_id.identification_number.number;
                                let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                user_data.decrypt_variable_data_with_context(
                                    &provider,
                                    manufacturer_id.manufacturer_code,
                                    id_num,
                                    manufacturer_id.version,
                                    manufacturer_id.device_type,
                                    &mut decrypted_buffer,
                                )
                            }
                            _ => Err(crate::decryption::DecryptionError::UnknownEncryptionState),
                        };

                        if let Ok(len) = decrypt_result {
                            decrypted_len = len;
                        }
                    }
                }
            }

            // Apply decrypted data records if decryption succeeded
            if decrypted_len > 0 {
                let decrypted_data = decrypted_buffer.get(..decrypted_len).unwrap_or(&[]);
                parsed_data.data_records = Some(user_data::DataRecords::new(
                    decrypted_data,
                    long_header_for_records,
                ));
            }
        }
        #[cfg(not(feature = "decryption"))]
        let _ = key;

        let mfr_code_str = format!("{}", parsed_data.frame.manufacturer_id.manufacturer_code);
        let base = serde_yaml::to_string(&parsed_data).unwrap_or_default();
        return if let Some(info) = crate::manufacturers::lookup_manufacturer(&mfr_code_str) {
            format!(
                "{}manufacturer_info:\n  name: {}\n  website: {}\n  description: {}\n",
                base, info.name, info.website, info.description
            )
        } else {
            base
        };
    }

    // If both fail, return error
    "---\nerror: Could not parse data\n".to_string()
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_table(input: &str, key: Option<&[u8; 16]>) -> String {
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
                let mut _is_encyrpted = false;
                if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block: _,
                    ..
                }) = &parsed_data.user_data
                {
                    let mut info_table = Table::new();
                    info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                    info_table.set_titles(row!["Field", "Value"]);
                    info_table.add_row(row![
                        "Identification Number",
                        long_tpl_header.identification_number
                    ]);
                    {
                        let mfr_str = long_tpl_header
                            .manufacturer
                            .as_ref()
                            .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m));
                        info_table.add_row(row!["Manufacturer", &mfr_str]);
                        if let Some(info) = crate::manufacturers::lookup_manufacturer(&mfr_str) {
                            info_table.add_row(row!["Manufacturer Name", info.name]);
                            info_table.add_row(row!["Website", info.website]);
                            info_table.add_row(row!["Description", info.description]);
                        }
                    }
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
                    _is_encyrpted = long_tpl_header.is_encrypted();
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
                            None => ")".to_string(),
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

    // If wired fails, try wireless - strip Format A CRCs if present
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        let wireless_mbus_link_layer::WirelessFrame {
            function,
            manufacturer_id,
            data,
        } = &parsed_data.frame;
        {
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_BOX_CHARS);
            table.set_titles(row!["Field", "Value"]);
            table.add_row(row!["Function", format!("{:?}", function)]);
            {
                let mfr_str = format!("{}", manufacturer_id.manufacturer_code);
                table.add_row(row!["Manufacturer Code", &mfr_str]);
                if let Some(info) = crate::manufacturers::lookup_manufacturer(&mfr_str) {
                    table.add_row(row!["Manufacturer Name", info.name]);
                    table.add_row(row!["Website", info.website]);
                    table.add_row(row!["Description", info.description]);
                }
            }
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
            let mut is_encrypted = false;
            match &parsed_data.user_data {
                Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    variable_data_block: _,
                    extended_link_layer: _,
                }) => {
                    let mut info_table = Table::new();
                    info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
                    info_table.set_titles(row!["Field", "Value"]);
                    info_table.add_row(row![
                        "Identification Number",
                        long_tpl_header.identification_number
                    ]);
                    {
                        let mfr_str = long_tpl_header
                            .manufacturer
                            .as_ref()
                            .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m));
                        info_table.add_row(row!["Manufacturer", &mfr_str]);
                        if let Some(info) = crate::manufacturers::lookup_manufacturer(&mfr_str) {
                            info_table.add_row(row!["Manufacturer Name", info.name]);
                            info_table.add_row(row!["Website", info.website]);
                            info_table.add_row(row!["Description", info.description]);
                        }
                    }
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
                    is_encrypted = long_tpl_header.is_encrypted();
                }
                Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
                    short_tpl_header,
                    variable_data_block: _,
                    extended_link_layer: _,
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
                    is_encrypted = short_tpl_header.is_encrypted();
                }
                _ => (),
            }

            let mut value_table = Table::new();
            value_table.set_format(*format::consts::FORMAT_BOX_CHARS);
            value_table.set_titles(row!["Value", "Data Information", "Header Hex", "Data Hex"]);

            if is_encrypted {
                #[cfg(feature = "decryption")]
                if let Some(key_bytes) = key {
                    // Try to decrypt
                    if let Some(user_data) = &parsed_data.user_data {
                        let mut decrypted = [0u8; 256];
                        let mut provider = crate::decryption::StaticKeyProvider::<1>::new();

                        // Get manufacturer info from user_data or frame
                        let decrypt_result = match user_data {
                            UserDataBlock::VariableDataStructureWithLongTplHeader {
                                long_tpl_header,
                                ..
                            } => {
                                if let Ok(mfr) = &long_tpl_header.manufacturer {
                                    let mfr_id = mfr.to_id();
                                    let id_num = long_tpl_header.identification_number.number;
                                    let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                    user_data.decrypt_variable_data(&provider, &mut decrypted)
                                } else {
                                    Err(crate::decryption::DecryptionError::DecryptionFailed)
                                }
                            }
                            UserDataBlock::VariableDataStructureWithShortTplHeader { .. } => {
                                // For short TPL, use link layer manufacturer info
                                let mfr_id = manufacturer_id.manufacturer_code.to_id();
                                let id_num = manufacturer_id.identification_number.number;
                                let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                user_data.decrypt_variable_data_with_context(
                                    &provider,
                                    manufacturer_id.manufacturer_code,
                                    id_num,
                                    manufacturer_id.version,
                                    manufacturer_id.device_type,
                                    &mut decrypted,
                                )
                            }
                            _ => Err(crate::decryption::DecryptionError::UnknownEncryptionState),
                        };

                        match decrypt_result {
                            Ok(len) => {
                                table_output.push_str("Decrypted successfully\n");
                                // Parse decrypted data records
                                let decrypted_data = decrypted.get(..len).unwrap_or(&[]);
                                // Get long_tpl_header if available for proper data record parsing
                                let long_header = match user_data {
                                    UserDataBlock::VariableDataStructureWithLongTplHeader {
                                        long_tpl_header,
                                        ..
                                    } => Some(long_tpl_header),
                                    _ => None,
                                };
                                let data_records =
                                    user_data::DataRecords::new(decrypted_data, long_header);
                                for record in data_records.flatten() {
                                    let value_information = match record
                                        .data_record_header
                                        .processed_data_record_header
                                        .value_information
                                    {
                                        Some(ref x) => format!("{}", x),
                                        None => ")".to_string(),
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
                                table_output.push_str(&value_table.to_string());
                            }
                            Err(e) => {
                                table_output.push_str(&format!("Decryption failed: {:?}\n", e));
                                table_output.push_str("Encrypted Payload : ");
                                table_output.push_str(
                                    &data
                                        .iter()
                                        .map(|b| format!("{:02X}", b))
                                        .collect::<String>(),
                                );
                                table_output.push('\n');
                            }
                        }
                    }
                } else {
                    table_output.push_str("Encrypted Payload : ");
                    table_output.push_str(
                        &data
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<String>(),
                    );
                    table_output.push('\n');
                }

                #[cfg(not(feature = "decryption"))]
                {
                    let _ = key; // Suppress unused warning
                    table_output.push_str("Encrypted Payload : ");
                    table_output.push_str(
                        &data
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<String>(),
                    );
                    table_output.push('\n');
                }
            } else {
                if let Some(data_records) = &parsed_data.data_records {
                    for record in data_records.clone().flatten() {
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => ")".to_string(),
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
pub fn parse_to_csv(input: &str, key: Option<&[u8; 16]>) -> String {
    use crate::user_data::UserDataBlock;
    use prettytable::csv;

    let data = clean_and_convert(input);
    // Buffer for decrypted data - must live as long as data_records
    let mut decrypted_buffer = [0u8; 256];
    let mut decrypted_len = 0usize;

    let mut writer = csv::Writer::from_writer(vec![]);

    // Try wired first
    if let Ok(mut parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        #[cfg(feature = "decryption")]
        if let Some(key_bytes) = key {
            if let Some(user_data) = &parsed_data.user_data {
                if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    ..
                } = user_data
                {
                    if long_tpl_header.is_encrypted() {
                        if let Ok(mfr) = &long_tpl_header.manufacturer {
                            let mut provider = crate::decryption::StaticKeyProvider::<1>::new();
                            let mfr_id = mfr.to_id();
                            let id_num = long_tpl_header.identification_number.number;
                            let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                            if let Ok(len) =
                                user_data.decrypt_variable_data(&provider, &mut decrypted_buffer)
                            {
                                decrypted_len = len;
                            }
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "decryption"))]
        let _ = key;

        // Apply decrypted data records if decryption succeeded
        #[cfg(feature = "decryption")]
        if decrypted_len > 0 {
            if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                ..
            }) = &parsed_data.user_data
            {
                let decrypted_data = decrypted_buffer.get(..decrypted_len).unwrap_or(&[]);
                parsed_data.data_records = Some(user_data::DataRecords::new(
                    decrypted_data,
                    Some(long_tpl_header),
                ));
            }
        }

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
                            None => ")".to_string(),
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

    // If wired fails, try wireless - strip Format A CRCs if present
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(mut parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        // Reset decrypted_len for wireless section
        decrypted_len = 0;

        #[cfg(feature = "decryption")]
        {
            let mut long_header_for_records: Option<&user_data::LongTplHeader> = None;
            if let Some(key_bytes) = key {
                let manufacturer_id = &parsed_data.frame.manufacturer_id;
                if let Some(user_data) = &parsed_data.user_data {
                    let (is_encrypted, long_header) = match user_data {
                        UserDataBlock::VariableDataStructureWithLongTplHeader {
                            long_tpl_header,
                            ..
                        } => (long_tpl_header.is_encrypted(), Some(long_tpl_header)),
                        UserDataBlock::VariableDataStructureWithShortTplHeader {
                            short_tpl_header,
                            ..
                        } => (short_tpl_header.is_encrypted(), None),
                        _ => (false, None),
                    };
                    long_header_for_records = long_header;

                    if is_encrypted {
                        let mut provider = crate::decryption::StaticKeyProvider::<1>::new();

                        let decrypt_result = match user_data {
                            UserDataBlock::VariableDataStructureWithLongTplHeader {
                                long_tpl_header,
                                ..
                            } => {
                                if let Ok(mfr) = &long_tpl_header.manufacturer {
                                    let mfr_id = mfr.to_id();
                                    let id_num = long_tpl_header.identification_number.number;
                                    let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                    user_data
                                        .decrypt_variable_data(&provider, &mut decrypted_buffer)
                                } else {
                                    Err(crate::decryption::DecryptionError::DecryptionFailed)
                                }
                            }
                            UserDataBlock::VariableDataStructureWithShortTplHeader { .. } => {
                                let mfr_id = manufacturer_id.manufacturer_code.to_id();
                                let id_num = manufacturer_id.identification_number.number;
                                let _ = provider.add_key(mfr_id, id_num, *key_bytes);
                                user_data.decrypt_variable_data_with_context(
                                    &provider,
                                    manufacturer_id.manufacturer_code,
                                    id_num,
                                    manufacturer_id.version,
                                    manufacturer_id.device_type,
                                    &mut decrypted_buffer,
                                )
                            }
                            _ => Err(crate::decryption::DecryptionError::UnknownEncryptionState),
                        };

                        if let Ok(len) = decrypt_result {
                            decrypted_len = len;
                        }
                    }
                }
            }

            // Apply decrypted data records if decryption succeeded
            if decrypted_len > 0 {
                let decrypted_data = decrypted_buffer.get(..decrypted_len).unwrap_or(&[]);
                parsed_data.data_records = Some(user_data::DataRecords::new(
                    decrypted_data,
                    long_header_for_records,
                ));
            }
        }

        let frame_type = "Wireless";

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
                variable_data_block: _,
                extended_link_layer: _,
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
                    None => ")".to_string(),
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

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_mermaid(input: &str, _key: Option<&[u8; 16]>) -> String {
    use user_data::UserDataBlock;

    const MAX_PER_ROW: usize = 4;
    // Colors for data record nodes (fill, text), cycling through the palette
    const RECORD_COLORS: &[(&str, &str)] = &[
        ("#1565c0", "#fff"),
        ("#2e7d32", "#fff"),
        ("#e65100", "#fff"),
        ("#6a1b9a", "#fff"),
        ("#c62828", "#fff"),
        ("#00695c", "#fff"),
        ("#f9a825", "#000"),
        ("#4527a0", "#fff"),
    ];

    let data = clean_and_convert(input);

    // Try wired first
    if let Ok(parsed_data) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        let mut out = String::from("flowchart TD\n");
        let mut styles = String::new();

        match parsed_data.frame {
            frames::WiredFrame::LongFrame {
                function,
                address,
                data: _,
            } => {
                // Frame header subgraph
                out.push_str("    subgraph FRAME_SG[\"Frame Header\"]\n");
                out.push_str("");
                out.push_str(&format!(
                    "        FTYPE[\"Long Frame\"]\n        FUNC[\"Function: {}\"]\n        ADDR[\"Address: {}\"]\n",
                    mermaid_escape(&format!("{}", function)),
                    mermaid_escape(&format!("{}", address))
                ));
                let (chains, pads) =
                    mermaid_centered_chains(&["FTYPE", "FUNC", "ADDR"], MAX_PER_ROW, "FP");
                out.push_str(&chains);
                out.push_str("    end\n");
                styles.push_str(&pads);
                styles.push_str("    style FRAME_SG fill:#2e86c1,color:#fff,stroke:#1a5276\n");
                styles.push_str("    style FTYPE fill:#2980b9,color:#fff,stroke:#1a5276\n");
                styles.push_str("    style FUNC fill:#2980b9,color:#fff,stroke:#1a5276\n");
                styles.push_str("    style ADDR fill:#2980b9,color:#fff,stroke:#1a5276\n");

                if let Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header,
                    ..
                }) = &parsed_data.user_data
                {
                    let mfr = long_tpl_header
                        .manufacturer
                        .as_ref()
                        .map_or_else(|e| format!("Error: {}", e), |m| format!("{}", m));

                    let mfr_info = crate::manufacturers::lookup_manufacturer(&mfr);
                    out.push_str("    subgraph DEV_SG[\"Device Info\"]\n");
                    out.push_str("");
                    out.push_str(&format!(
                        "        DEV1[\"ID: {}\"]\n",
                        mermaid_escape(&format!("{}", long_tpl_header.identification_number))
                    ));
                    out.push_str(&format!(
                        "        DEV2[\"Manufacturer: {}\"]\n",
                        mermaid_escape(&mfr)
                    ));
                    out.push_str(&format!(
                        "        DEV3[\"Version: {}\"]\n",
                        long_tpl_header.version
                    ));
                    out.push_str(&format!(
                        "        DEV4[\"Device Type: {}\"]\n",
                        mermaid_escape(&format!("{:?}", long_tpl_header.device_type))
                    ));
                    out.push_str(&format!(
                        "        DEV5[\"Access Number: {}\"]\n",
                        long_tpl_header.short_tpl_header.access_number
                    ));
                    out.push_str(&format!(
                        "        DEV6[\"Status: {}\"]\n",
                        mermaid_escape(&format!("{}", long_tpl_header.short_tpl_header.status))
                    ));
                    let mut dev_node_count = 6usize;
                    if let Some(ref info) = mfr_info {
                        dev_node_count += 1;
                        out.push_str(&format!(
                            "        DEV{}[\"Name: {}\"]\n",
                            dev_node_count,
                            mermaid_escape(info.name)
                        ));
                        dev_node_count += 1;
                        out.push_str(&format!(
                            "        DEV{}[\"Website: {}\"]\n",
                            dev_node_count,
                            mermaid_escape(info.website)
                        ));
                        dev_node_count += 1;
                        out.push_str(&format!(
                            "        DEV{}[\"{}\"]\n",
                            dev_node_count,
                            mermaid_escape(info.description)
                        ));
                    }
                    let dev_ids: Vec<String> =
                        (1..=dev_node_count).map(|i| format!("DEV{}", i)).collect();
                    let dev_id_refs: Vec<&str> = dev_ids.iter().map(|s| s.as_str()).collect();
                    let (chains, pads) = mermaid_centered_chains(&dev_id_refs, MAX_PER_ROW, "DP");
                    out.push_str(&chains);
                    out.push_str("    end\n");
                    styles.push_str(&pads);
                    styles.push_str("    style DEV_SG fill:#1e8449,color:#fff,stroke:#145a32\n");
                    for i in 1..=dev_node_count {
                        styles.push_str(&format!(
                            "    style DEV{} fill:#27ae60,color:#fff,stroke:#145a32\n",
                            i
                        ));
                    }

                    out.push_str("    FRAME_SG --> DEV_SG\n");
                }

                if let Some(data_records) = parsed_data.data_records {
                    out.push_str("    subgraph REC_SG[\"Data Records\"]\n");
                    out.push_str("");
                    let records: Vec<_> = data_records.flatten().collect();
                    for (i, record) in records.iter().enumerate() {
                        let value_information = match record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                        {
                            Some(ref x) => format!("{}", x),
                            None => String::new(),
                        };
                        let label = format!("({}{}", record.data, value_information);
                        out.push_str(&format!("        R{}[\"{}\"]\n", i, mermaid_escape(&label)));
                        let (fill, text) = RECORD_COLORS
                            .get(i % RECORD_COLORS.len())
                            .copied()
                            .unwrap_or(("#888", "#fff"));
                        styles.push_str(&format!(
                            "    style R{} fill:{},color:{},stroke:#333\n",
                            i, fill, text
                        ));
                    }
                    let ids: Vec<String> = (0..records.len()).map(|i| format!("R{}", i)).collect();
                    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                    let (chains, pads) = mermaid_centered_chains(&id_refs, MAX_PER_ROW, "RP");
                    out.push_str(&chains);
                    out.push_str("    end\n");
                    styles.push_str("    style REC_SG fill:#6c3483,color:#fff,stroke:#4a235a\n");
                    styles.push_str(&pads);
                    out.push_str("    DEV_SG --> REC_SG\n");
                }
            }
            frames::WiredFrame::ShortFrame { function, address } => {
                out.push_str("    subgraph FRAME_SG[\"Short Frame\"]\n");
                out.push_str(&format!(
                    "        FUNC[\"Function: {}\"]\n        ADDR[\"Address: {}\"]\n",
                    mermaid_escape(&format!("{}", function)),
                    mermaid_escape(&format!("{}", address))
                ));
                out.push_str("    end\n");
                styles.push_str("    style FRAME_SG fill:#2e86c1,color:#fff,stroke:#1a5276\n");
            }
            frames::WiredFrame::SingleCharacter { character } => {
                out.push_str(&format!(
                    "    FRAME[\"Single Character: 0x{:02X}\"]\n",
                    character
                ));
                styles.push_str("    style FRAME fill:#2e86c1,color:#fff,stroke:#1a5276\n");
            }
            frames::WiredFrame::ControlFrame {
                function, address, ..
            } => {
                out.push_str("    subgraph FRAME_SG[\"Control Frame\"]\n");
                out.push_str(&format!(
                    "        FUNC[\"Function: {}\"]\n        ADDR[\"Address: {}\"]\n",
                    mermaid_escape(&format!("{}", function)),
                    mermaid_escape(&format!("{}", address))
                ));
                out.push_str("    end\n");
                styles.push_str("    style FRAME_SG fill:#2e86c1,color:#fff,stroke:#1a5276\n");
            }
            _ => {
                out.push_str("    FRAME[\"Unknown Frame\"]\n");
            }
        }
        out.push_str(&styles);
        return out;
    }

    // Try wireless
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(parsed_data) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        let wireless_mbus_link_layer::WirelessFrame {
            function,
            manufacturer_id,
            data: _,
        } = &parsed_data.frame;

        let mut out = String::from("flowchart TD\n");
        let mut styles = String::new();

        let wmfr_str = format!("{}", manufacturer_id.manufacturer_code);
        let wmfr_info = crate::manufacturers::lookup_manufacturer(&wmfr_str);
        out.push_str("    subgraph FRAME_SG[\"Wireless Frame\"]\n");
        out.push_str(&format!("        FUNC[\"Function: {:?}\"]\n", function));
        out.push_str(&format!(
            "        MFR[\"Manufacturer: {}\"]\n",
            mermaid_escape(&wmfr_str)
        ));
        out.push_str(&format!(
            "        ID[\"ID: {:?}\"]\n",
            manufacturer_id.identification_number
        ));
        out.push_str(&format!(
            "        DEVT[\"Device Type: {:?}\"]\n",
            manufacturer_id.device_type
        ));
        out.push_str(&format!(
            "        VER[\"Version: {:?}\"]\n",
            manufacturer_id.version
        ));
        let mut wframe_nodes: Vec<&str> = vec!["FUNC", "MFR", "ID", "DEVT", "VER"];
        if let Some(ref info) = wmfr_info {
            out.push_str(&format!(
                "        MFRNAME[\"Name: {}\"]\n",
                mermaid_escape(info.name)
            ));
            out.push_str(&format!(
                "        MFRWEB[\"Website: {}\"]\n",
                mermaid_escape(info.website)
            ));
            out.push_str(&format!(
                "        MFRDESC[\"{}\"]\n",
                mermaid_escape(info.description)
            ));
            wframe_nodes.extend_from_slice(&["MFRNAME", "MFRWEB", "MFRDESC"]);
        }
        out.push_str("    end\n");
        styles.push_str("    style FRAME_SG fill:#2e86c1,color:#fff,stroke:#1a5276\n");
        for node in &wframe_nodes {
            styles.push_str(&format!(
                "    style {} fill:#2980b9,color:#fff,stroke:#1a5276\n",
                node
            ));
        }

        if let Some(data_records) = parsed_data.data_records {
            out.push_str("    subgraph REC_SG[\"Data Records\"]\n");
            let records: Vec<_> = data_records.flatten().collect();
            for (i, record) in records.iter().enumerate() {
                let value_information = match record
                    .data_record_header
                    .processed_data_record_header
                    .value_information
                {
                    Some(ref x) => format!("{}", x),
                    None => String::new(),
                };
                let label = format!("({}{}", record.data, value_information);
                out.push_str(&format!("        R{}[\"{}\"]\n", i, mermaid_escape(&label)));
                let (fill, text) = RECORD_COLORS
                    .get(i % RECORD_COLORS.len())
                    .copied()
                    .unwrap_or(("#888", "#fff"));
                styles.push_str(&format!(
                    "    style R{} fill:{},color:{},stroke:#333\n",
                    i, fill, text
                ));
            }
            out.push_str("    end\n");
            styles.push_str("    style REC_SG fill:#6c3483,color:#fff,stroke:#4a235a\n");
            out.push_str("    FRAME_SG --> REC_SG\n");
        }

        out.push_str(&styles);
        return out;
    }

    "flowchart TD\n    ERR[\"Error: Could not parse data\"]\n".to_string()
}

/// Returns (body, styles) where body contains padding node declarations + chain lines,
/// and styles contains the invisible styles for padding nodes.
/// Pads each incomplete row symmetrically so nodes appear centred.
#[cfg(feature = "std")]
fn mermaid_centered_chains(ids: &[&str], max_per_row: usize, pad_prefix: &str) -> (String, String) {
    let mut body = String::new();
    let mut styles = String::new();
    let mut pad_idx = 0usize;
    for chunk in ids.chunks(max_per_row) {
        let row: Vec<String> = if chunk.len() == max_per_row {
            chunk.iter().map(|s| s.to_string()).collect()
        } else {
            let padding = max_per_row - chunk.len();
            let left = padding / 2;
            let right = padding - left;
            let mut row: Vec<String> = Vec::new();
            for _ in 0..left {
                let id = format!("{}P{}", pad_prefix, pad_idx);
                body.push_str(&format!("        {}[\" \"]\n", id));
                styles.push_str(&format!(
                    "    style {} fill:none,stroke:none,color:none\n",
                    id
                ));
                row.push(id);
                pad_idx += 1;
            }
            row.extend(chunk.iter().map(|s| s.to_string()));
            for _ in 0..right {
                let id = format!("{}P{}", pad_prefix, pad_idx);
                body.push_str(&format!("        {}[\" \"]\n", id));
                styles.push_str(&format!(
                    "    style {} fill:none,stroke:none,color:none\n",
                    id
                ));
                row.push(id);
                pad_idx += 1;
            }
            row
        };
        // Emit individual pairs instead of a chain to maximise mermaid compatibility
        for pair in row.windows(2) {
            if let (Some(a), Some(b)) = (pair.first(), pair.get(1)) {
                body.push_str(&format!("        {}~~~{}\n", a, b));
            }
        }
    }
    (body, styles)
}

#[cfg(feature = "std")]
fn mermaid_escape(s: &str) -> String {
    s.replace('"', "#quot;")
        .replace('[', "#91;")
        .replace(']', "#93;")
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_converter() {
        use super::parse_to_csv;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let csv_output: String = parse_to_csv(input, None);
        println!("{}", csv_output);
        let yaml_output: String = super::parse_to_yaml(input, None);
        println!("{}", yaml_output);
        let json_output: String = super::parse_to_json(input, None);
        println!("{}", json_output);
        let table_output: String = super::parse_to_table(input, None);
        println!("{}", table_output);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_csv_expected_output() {
        use super::parse_to_csv;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let csv_output = parse_to_csv(input, None);

        let expected = "FrameType,Function,Address,Identification Number,Manufacturer,Access Number,Status,Security Mode,Version,Device Type,DataPoint1_Value,DataPoint1_Info,DataPoint2_Value,DataPoint2_Info,DataPoint3_Value,DataPoint3_Info,DataPoint4_Value,DataPoint4_Info,DataPoint5_Value,DataPoint5_Info,DataPoint6_Value,DataPoint6_Info,DataPoint7_Value,DataPoint7_Info,DataPoint8_Value,DataPoint8_Info,DataPoint9_Value,DataPoint9_Info,DataPoint10_Value,DataPoint10_Info\nLongFrame,\"RspUd (ACD: false, DFC: false)\",Primary (1),02205100,SLB,0,\"Permanent error, Manufacturer specific 3\",No encryption used,2,Heat Meter (Return),(0)e4[Wh](Energy),\"0,Inst,32-bit Integer\",(3)e-1[m³](Volume),\"0,Inst,BCD 8-digit\",(0)e3[W](Power),\"0,Inst,BCD 6-digit\",(0)e-3[m³h⁻¹](VolumeFlow),\"0,Inst,BCD 6-digit\",(1288)e-1[°C](FlowTemperature),\"0,Inst,BCD 4-digit\",(516)e-1[°C](ReturnTemperature),\"0,Inst,BCD 4-digit\",(7723)e-2[°K](TemperatureDifference),\"0,Inst,BCD 6-digit\",(12/Jan/12)(Date),\"0,Inst,Date Type G\",(3383)[day](OperatingTime),\"0,Inst,16-bit Integer\",\"(Manufacturer Specific: [96, 0])\",\"0,Inst,Special Functions (ManufacturerSpecific)\"\n";

        assert_eq!(csv_output, expected);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_yaml_expected_output() {
        use super::parse_to_yaml;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let yaml_output = parse_to_yaml(input, None);

        // First line of YAML output to test against - we'll test just the beginning to avoid a massive string
        let expected_start = "frame: !LongFrame\n  function: !RspUd\n    acd: false\n    dfc: false\n  address: !Primary 1\nuser_data: !VariableDataStructureWithLongTplHeader\n";

        assert!(yaml_output.starts_with(expected_start));
        // Additional checks for specific content in the YAML
        assert!(yaml_output.contains("device_type: HeatMeterReturn"));
        assert!(yaml_output.contains("identification_number:"));
        assert!(yaml_output.contains("status: PERMANENT_ERROR | MANUFACTURER_SPECIFIC_3"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_expected_output() {
        use super::parse_to_json;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let json_output = parse_to_json(input, None);

        // Testing specific content in JSON
        assert!(json_output.contains("\"Ok\""));
        assert!(json_output.contains("\"LongFrame\""));
        assert!(json_output.contains("\"RspUd\""));
        assert!(json_output.contains("\"number\": 2205100"));
        assert!(json_output.contains("\"device_type\": \"HeatMeterReturn\""));
        assert!(json_output.contains("\"status\": \"PERMANENT_ERROR | MANUFACTURER_SPECIFIC_3\""));

        // Verify JSON structure is valid
        let json_parsed = serde_json::from_str::<serde_json::Value>(&json_output);
        assert!(json_parsed.is_ok());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_mermaid_expected_output() {
        use super::parse_to_mermaid;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let mermaid_output = parse_to_mermaid(input, None);

        assert!(mermaid_output.starts_with("flowchart TD\n"));
        assert!(mermaid_output.contains("Long Frame"));
        assert!(mermaid_output.contains("Device Info"));
        assert!(mermaid_output.contains("Data Records"));
        assert!(mermaid_output.contains("02205100"));
        assert!(mermaid_output.contains("SLB"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_table_expected_output() {
        use super::parse_to_table;
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let table_output = parse_to_table(input, None);

        // First section of the table output
        assert!(table_output.starts_with("Long Frame"));

        // Key content pieces to verify
        assert!(table_output.contains("RspUd (ACD: false, DFC: false)"));
        assert!(table_output.contains("Primary (1)"));
        assert!(table_output.contains("Identification Number"));
        assert!(table_output.contains("02205100"));
        assert!(table_output.contains("SLB"));

        // Data point verifications
        assert!(table_output.contains("(0)e4[Wh]"));
        assert!(table_output.contains("(3)e-1[m³](Volume)"));
        assert!(table_output.contains("(1288)e-1[°C](FlowTemperature)"));
        assert!(table_output.contains("(12/Jan/12)(Date)"));
        assert!(table_output.contains("(3383)[day]"));
    }
}
