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
                    match &x {
                        user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                            variable_data_block,
                            ..
                        }
                        | user_data::UserDataBlock::VariableDataStructureWithShortTplHeader {
                            variable_data_block,
                            ..
                        }
                        | user_data::UserDataBlock::VariableDataStructureWithoutTplHeader {
                            variable_data_block,
                            ..
                        } => {
                            data_records = Some((*variable_data_block).into());
                        }
                        _ => {}
                    }
                    user_data = Some(x);
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
                user_data::UserDataBlock::VariableDataStructureWithoutTplHeader {
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
        "csv" => parse_to_csv(data, key),
        "xml" => parse_to_xml(data),
        "mermaid" => parse_to_mermaid(data, key),
        "annotated" => parse_to_annotated(data),
        "hexview" => parse_to_hexview(data, key),
        "annotated-text" => parse_to_annotated_text(data),
        _ => parse_to_table(data, key),
    }
}

/// Normalized, human-readable interpretation of a parsed frame.
///
/// This is the single source of truth for the table and CSV renderers and is
/// embedded as `summary` in the JSON and YAML outputs, so every output format
/// exposes the same header information regardless of whether it comes from a
/// long TPL header, a short TPL header or the wireless link layer.
#[cfg(feature = "std")]
#[derive(serde::Serialize)]
struct FrameSummary {
    frame_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identification_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<ManufacturerSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_number: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_type: Option<String>,
    encrypted: bool,
    decrypted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    encrypted_payload_hex: Option<String>,
    records: Vec<RecordSummary>,
}

#[cfg(feature = "std")]
#[derive(serde::Serialize)]
struct ManufacturerSummary {
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[cfg(feature = "std")]
#[derive(serde::Serialize)]
struct RecordSummary {
    /// Human-readable rendering (same string the table and CSV outputs use).
    display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exponent: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity: Option<String>,
    data_information: String,
    header_hex: String,
    data_hex: String,
}

#[cfg(feature = "std")]
#[derive(serde::Serialize)]
struct SummarySection<'a> {
    summary: &'a FrameSummary,
}

#[cfg(feature = "std")]
struct ParsedOutput {
    summary: FrameSummary,
    json: serde_json::Value,
    yaml: String,
}

#[cfg(feature = "std")]
impl FrameSummary {
    fn new(frame_type: &'static str) -> Self {
        Self {
            frame_type,
            function: None,
            address: None,
            identification_number: None,
            manufacturer: None,
            access_number: None,
            status: None,
            security_mode: None,
            version: None,
            device_type: None,
            encrypted: false,
            decrypted: false,
            encrypted_payload_hex: None,
            records: Vec::new(),
        }
    }
}

#[cfg(feature = "std")]
fn manufacturer_summary(code: String) -> ManufacturerSummary {
    let info = crate::manufacturers::lookup_manufacturer(&code);
    ManufacturerSummary {
        code,
        name: info.as_ref().map(|i| i.name.to_string()),
        website: info.as_ref().map(|i| i.website.to_string()),
        description: info.as_ref().map(|i| i.description.to_string()),
    }
}

#[cfg(feature = "std")]
fn hex_string(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect()
}

#[cfg(feature = "std")]
fn user_data_is_encrypted(user_data: Option<&user_data::UserDataBlock>) -> bool {
    use user_data::UserDataBlock;
    match user_data {
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        }) => long_tpl_header.is_encrypted(),
        Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
            short_tpl_header, ..
        }) => short_tpl_header.is_encrypted(),
        _ => false,
    }
}

#[cfg(feature = "std")]
fn variable_data_block<'a>(user_data: Option<&user_data::UserDataBlock<'a>>) -> Option<&'a [u8]> {
    use user_data::UserDataBlock;
    match user_data {
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            variable_data_block,
            ..
        })
        | Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
            variable_data_block,
            ..
        })
        | Some(UserDataBlock::VariableDataStructureWithoutTplHeader {
            variable_data_block,
            ..
        }) => Some(variable_data_block),
        _ => None,
    }
}

#[cfg(feature = "std")]
fn record_summaries(records: &user_data::DataRecords) -> Vec<RecordSummary> {
    use user_data::data_information::DataType;

    records
        .clone()
        .flatten()
        .map(|record| {
            let value_information = record
                .data_record_header
                .processed_data_record_header
                .value_information
                .as_ref();
            let value_information_display = match value_information {
                Some(x) => format!("{}", x),
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
            let value = match record.data.value {
                Some(DataType::Number(n)) | Some(DataType::LossyNumber(n)) => Some(n),
                _ => None,
            };
            let unit = value_information
                .map(|vi| vi.units.iter().map(ToString::to_string).collect::<String>())
                .filter(|s| !s.is_empty());
            let quantity = value_information
                .map(|vi| {
                    vi.labels
                        .iter()
                        .map(|label| format!("{:?}", label))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|s| !s.is_empty());
            RecordSummary {
                display: format!("({}{}", record.data, value_information_display),
                value,
                exponent: value_information.map(|vi| vi.decimal_scale_exponent),
                unit,
                quantity,
                data_information,
                header_hex: record.data_record_header_hex(),
                data_hex: record.data_hex(),
            }
        })
        .collect()
}

#[cfg(feature = "std")]
fn apply_user_data_summary(
    summary: &mut FrameSummary,
    user_data: Option<&user_data::UserDataBlock>,
) {
    use user_data::UserDataBlock;
    match user_data {
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        }) => {
            summary.identification_number = Some(long_tpl_header.identification_number.to_string());
            summary.manufacturer = Some(manufacturer_summary(
                long_tpl_header
                    .manufacturer
                    .as_ref()
                    .map_or_else(|e| format!("Error: {}", e), |m| m.to_string()),
            ));
            summary.access_number = Some(long_tpl_header.short_tpl_header.access_number);
            summary.status = Some(long_tpl_header.short_tpl_header.status.to_string());
            summary.security_mode = Some(
                long_tpl_header
                    .short_tpl_header
                    .configuration_field
                    .security_mode()
                    .to_string(),
            );
            summary.version = Some(long_tpl_header.version);
            summary.device_type = Some(long_tpl_header.device_type.to_string());
        }
        Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
            short_tpl_header, ..
        }) => {
            summary.access_number = Some(short_tpl_header.access_number);
            summary.status = Some(short_tpl_header.status.to_string());
            summary.security_mode = Some(
                short_tpl_header
                    .configuration_field
                    .security_mode()
                    .to_string(),
            );
        }
        Some(UserDataBlock::FixedDataStructure {
            identification_number,
            access_number,
            status,
            ..
        }) => {
            summary.identification_number = Some(identification_number.to_string());
            summary.access_number = Some(*access_number);
            summary.status = Some(status.to_string());
        }
        _ => {}
    }
}

#[cfg(feature = "std")]
fn finalize_summary(
    summary: &mut FrameSummary,
    data_records: Option<&user_data::DataRecords>,
    user_data: Option<&user_data::UserDataBlock>,
    encrypted: bool,
    decrypted: bool,
) {
    summary.encrypted = encrypted;
    summary.decrypted = decrypted;
    if encrypted && !decrypted {
        summary.encrypted_payload_hex = variable_data_block(user_data).map(hex_string);
    } else if let Some(records) = data_records {
        summary.records = record_summaries(records);
    }
}

#[cfg(feature = "std")]
fn summarize_wired(
    parsed: &MbusData<frames::WiredFrame>,
    encrypted: bool,
    decrypted: bool,
) -> FrameSummary {
    let mut summary = match &parsed.frame {
        frames::WiredFrame::LongFrame { .. } => FrameSummary::new("LongFrame"),
        frames::WiredFrame::ShortFrame { .. } => FrameSummary::new("ShortFrame"),
        frames::WiredFrame::ControlFrame { .. } => FrameSummary::new("ControlFrame"),
        frames::WiredFrame::SingleCharacter { .. } => FrameSummary::new("SingleCharacter"),
        _ => FrameSummary::new("Unknown"),
    };
    match &parsed.frame {
        frames::WiredFrame::LongFrame {
            function, address, ..
        }
        | frames::WiredFrame::ControlFrame {
            function, address, ..
        }
        | frames::WiredFrame::ShortFrame { function, address } => {
            summary.function = Some(function.to_string());
            summary.address = Some(address.to_string());
        }
        _ => {}
    }
    apply_user_data_summary(&mut summary, parsed.user_data.as_ref());
    finalize_summary(
        &mut summary,
        parsed.data_records.as_ref(),
        parsed.user_data.as_ref(),
        encrypted,
        decrypted,
    );
    summary
}

#[cfg(feature = "std")]
fn summarize_wireless(
    parsed: &MbusData<WirelessFrame>,
    encrypted: bool,
    decrypted: bool,
) -> FrameSummary {
    let manufacturer_id = &parsed.frame.manufacturer_id;
    let mut summary = FrameSummary::new("Wireless");
    summary.function = Some(parsed.frame.function.to_string());
    summary.identification_number = Some(manufacturer_id.identification_number.to_string());
    summary.manufacturer = Some(manufacturer_summary(
        manufacturer_id.manufacturer_code.to_string(),
    ));
    summary.version = Some(manufacturer_id.version);
    summary.device_type = Some(manufacturer_id.device_type.to_string());
    // A long TPL header overrides the link-layer addressing where present.
    apply_user_data_summary(&mut summary, parsed.user_data.as_ref());
    finalize_summary(
        &mut summary,
        parsed.data_records.as_ref(),
        parsed.user_data.as_ref(),
        encrypted,
        decrypted,
    );
    summary
}

#[cfg(feature = "std")]
fn build_output<T: serde::Serialize>(parsed: &T, summary: FrameSummary) -> ParsedOutput {
    let mut json = serde_json::to_value(parsed).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = json {
        map.insert(
            "summary".to_string(),
            serde_json::to_value(&summary).unwrap_or_default(),
        );
    }

    let mut yaml = serde_yaml::to_string(parsed).unwrap_or_default();
    yaml.push_str(
        &serde_yaml::to_string(&SummarySection { summary: &summary }).unwrap_or_default(),
    );

    ParsedOutput {
        summary,
        json,
        yaml,
    }
}

/// Parses a frame (wired first, wireless as fallback), applies decryption when
/// a key is provided, and produces the shared output model every text format
/// renders from.
#[cfg(feature = "std")]
#[cfg_attr(not(feature = "decryption"), allow(unused_mut))]
fn parse_frame_output(input: &str, key: Option<&[u8; 16]>) -> Option<ParsedOutput> {
    let data = clean_and_convert(input);
    #[cfg(feature = "decryption")]
    let mut decrypted_buffer = [0u8; 256];
    #[cfg(not(feature = "decryption"))]
    let _ = key;

    // Try wired first
    if let Ok(mut parsed) = MbusData::<frames::WiredFrame>::try_from(data.as_slice()) {
        let encrypted = user_data_is_encrypted(parsed.user_data.as_ref());
        let mut decrypted = false;
        #[cfg(feature = "decryption")]
        if encrypted {
            if let (Some(key_bytes), Some(user_data)) = (key, parsed.user_data.as_ref()) {
                if let Some(len) = decrypt_variable_data_with_key(
                    user_data,
                    None,
                    key_bytes,
                    &mut decrypted_buffer,
                ) {
                    if let (
                        Some(decrypted_data),
                        Some(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                            long_tpl_header,
                            ..
                        }),
                    ) = (decrypted_buffer.get(..len), &parsed.user_data)
                    {
                        parsed.data_records = Some(user_data::DataRecords::new(
                            decrypted_data,
                            Some(long_tpl_header),
                        ));
                        decrypted = true;
                    }
                }
            }
        }
        let summary = summarize_wired(&parsed, encrypted, decrypted);
        return Some(build_output(&parsed, summary));
    }

    // If wired fails, try wireless - strip Format A CRCs if present
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(&data, &mut crc_buf).unwrap_or(&data);
    if let Ok(mut parsed) =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data)
    {
        let encrypted = user_data_is_encrypted(parsed.user_data.as_ref());
        let mut decrypted = false;
        #[cfg(feature = "decryption")]
        if encrypted {
            if let (Some(key_bytes), Some(user_data)) = (key, parsed.user_data.as_ref()) {
                if let Some(len) = decrypt_variable_data_with_key(
                    user_data,
                    Some(&parsed.frame.manufacturer_id),
                    key_bytes,
                    &mut decrypted_buffer,
                ) {
                    if let Some(decrypted_data) = decrypted_buffer.get(..len) {
                        let long_header = match &parsed.user_data {
                            Some(
                                user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
                                    long_tpl_header,
                                    ..
                                },
                            ) => Some(long_tpl_header),
                            _ => None,
                        };
                        parsed.data_records =
                            Some(user_data::DataRecords::new(decrypted_data, long_header));
                        decrypted = true;
                    }
                }
            }
        }
        let summary = summarize_wireless(&parsed, encrypted, decrypted);
        return Some(build_output(&parsed, summary));
    }

    None
}

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_json(input: &str, key: Option<&[u8; 16]>) -> String {
    match parse_frame_output(input, key) {
        Some(output) => serde_json::to_string_pretty(&output.json).unwrap_or_default(),
        None => "{}".to_string(),
    }
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_yaml(input: &str, key: Option<&[u8; 16]>) -> String {
    match parse_frame_output(input, key) {
        Some(output) => output.yaml,
        None => "---\nerror: Could not parse data\n".to_string(),
    }
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_table(input: &str, key: Option<&[u8; 16]>) -> String {
    match parse_frame_output(input, key) {
        Some(output) => render_table(&output.summary, key.is_some()),
        None => "Error: Could not parse data as wired or wireless M-Bus".to_string(),
    }
}

#[cfg(feature = "std")]
fn render_table(summary: &FrameSummary, key_provided: bool) -> String {
    let mut out = String::new();
    let title = match summary.frame_type {
        "LongFrame" => "Long Frame",
        "ShortFrame" => "Short Frame",
        "ControlFrame" => "Control Frame",
        "SingleCharacter" => "Single Character Frame",
        "Wireless" => "Wireless Frame",
        other => other,
    };
    out.push_str(title);
    out.push('\n');

    // Wired frames carry link-layer function and address in their own table.
    if let (Some(function), Some(address)) = (&summary.function, &summary.address) {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["Function", "Address"]);
        table.add_row(row![function, address]);
        out.push_str(&table.to_string());
    }

    let mut info_table = Table::new();
    info_table.set_format(*format::consts::FORMAT_BOX_CHARS);
    info_table.set_titles(row!["Field", "Value"]);
    if summary.address.is_none() {
        if let Some(function) = &summary.function {
            info_table.add_row(row!["Function", function]);
        }
    }
    if let Some(identification_number) = &summary.identification_number {
        info_table.add_row(row!["Identification Number", identification_number]);
    }
    if let Some(manufacturer) = &summary.manufacturer {
        info_table.add_row(row!["Manufacturer", manufacturer.code]);
        if let Some(name) = &manufacturer.name {
            info_table.add_row(row!["Manufacturer Name", name]);
        }
        if let Some(website) = &manufacturer.website {
            info_table.add_row(row!["Website", website]);
        }
        if let Some(description) = &manufacturer.description {
            info_table.add_row(row!["Description", description]);
        }
    }
    if let Some(access_number) = summary.access_number {
        info_table.add_row(row!["Access Number", access_number]);
    }
    if let Some(status) = &summary.status {
        info_table.add_row(row!["Status", status]);
    }
    if let Some(security_mode) = &summary.security_mode {
        info_table.add_row(row!["Security Mode", security_mode]);
    }
    if let Some(version) = summary.version {
        info_table.add_row(row!["Version", version]);
    }
    if let Some(device_type) = &summary.device_type {
        info_table.add_row(row!["Device Type", device_type]);
    }
    if !info_table.is_empty() {
        out.push_str(&info_table.to_string());
    }

    if summary.encrypted && !summary.decrypted {
        if key_provided {
            out.push_str("Decryption failed\n");
        }
        if let Some(payload_hex) = &summary.encrypted_payload_hex {
            out.push_str("Encrypted Payload : ");
            out.push_str(payload_hex);
            out.push('\n');
        }
        return out;
    }

    if summary.decrypted {
        out.push_str("Decrypted successfully\n");
    }

    if matches!(summary.frame_type, "LongFrame" | "Wireless") || !summary.records.is_empty() {
        let mut value_table = Table::new();
        value_table.set_format(*format::consts::FORMAT_BOX_CHARS);
        value_table.set_titles(row!["Value", "Data Information", "Header Hex", "Data Hex"]);
        for record in &summary.records {
            value_table.add_row(row![
                record.display,
                record.data_information,
                record.header_hex,
                record.data_hex
            ]);
        }
        out.push_str(&value_table.to_string());
    }

    out
}

#[cfg(feature = "std")]
#[must_use]
pub fn parse_to_csv(input: &str, key: Option<&[u8; 16]>) -> String {
    use prettytable::csv;

    let mut writer = csv::Writer::from_writer(vec![]);

    let Some(output) = parse_frame_output(input, key) else {
        writer
            .write_record(["Error"])
            .map_err(|_| ())
            .unwrap_or_default();
        writer
            .write_record(["Error parsing data as wired or wireless M-Bus"])
            .map_err(|_| ())
            .unwrap_or_default();
        let csv_data = writer.into_inner().unwrap_or_default();
        return String::from_utf8(csv_data)
            .unwrap_or_else(|_| "Error converting CSV data to string".to_string());
    };

    let summary = &output.summary;

    let mut headers = vec![
        "FrameType".to_string(),
        "Function".to_string(),
        "Address".to_string(),
        "Identification Number".to_string(),
        "Manufacturer".to_string(),
        "Manufacturer Name".to_string(),
        "Access Number".to_string(),
        "Status".to_string(),
        "Security Mode".to_string(),
        "Version".to_string(),
        "Device Type".to_string(),
    ];
    for i in 1..=summary.records.len() {
        headers.push(format!("DataPoint{}_Value", i));
        headers.push(format!("DataPoint{}_Info", i));
    }
    writer
        .write_record(&headers)
        .map_err(|_| ())
        .unwrap_or_default();

    let mut row = vec![
        summary.frame_type.to_string(),
        summary.function.clone().unwrap_or_default(),
        summary.address.clone().unwrap_or_default(),
        summary.identification_number.clone().unwrap_or_default(),
        summary
            .manufacturer
            .as_ref()
            .map(|m| m.code.clone())
            .unwrap_or_default(),
        summary
            .manufacturer
            .as_ref()
            .and_then(|m| m.name.clone())
            .unwrap_or_default(),
        summary
            .access_number
            .map(|n| n.to_string())
            .unwrap_or_default(),
        summary.status.clone().unwrap_or_default(),
        summary.security_mode.clone().unwrap_or_default(),
        summary.version.map(|v| v.to_string()).unwrap_or_default(),
        summary.device_type.clone().unwrap_or_default(),
    ];
    for record in &summary.records {
        row.push(record.display.clone());
        row.push(record.data_information.clone());
    }
    writer
        .write_record(&row)
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
#[must_use]
fn parse_to_xml(input: &str) -> String {
    let data = clean_and_convert(input);
    crate::rscada_xml::render_from_bytes(&data)
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_annotated(input: &str) -> String {
    let data = clean_and_convert(input);
    annotated_segments_to_json(&data)
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_hexview(input: &str, key: Option<&[u8; 16]>) -> String {
    let data = clean_and_convert(input);

    #[cfg(feature = "decryption")]
    if let Some(key_bytes) = key {
        if let Some(display_data) = decrypted_hexview_data(&data, key_bytes) {
            return match crate::annotate::annotate_frame(&display_data) {
                Ok(mut segments) => {
                    replace_encrypted_payload_segments(&mut segments, &display_data);
                    serde_json::to_string_pretty(&serde_json::json!({
                        "bytes": display_data,
                        "segments": segments,
                        "decrypted": true,
                    }))
                    .unwrap_or_default()
                }
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            };
        }
    }

    #[cfg(not(feature = "decryption"))]
    let _ = key;

    annotated_segments_to_json(&data)
}

#[cfg(feature = "std")]
#[must_use]
fn annotated_segments_to_json(data: &[u8]) -> String {
    match crate::annotate::annotate_frame(data) {
        Ok(segments) => serde_json::to_string_pretty(&segments).unwrap_or_default(),
        Err(e) => format!("{{\"error\": \"{}\"}}", e),
    }
}

#[cfg(all(feature = "std", feature = "decryption"))]
fn decrypted_hexview_data(data: &[u8], key: &[u8; 16]) -> Option<Vec<u8>> {
    decrypted_wired_hexview_data(data, key).or_else(|| decrypted_wireless_hexview_data(data, key))
}

#[cfg(all(feature = "std", feature = "decryption"))]
fn replace_encrypted_payload_segments(
    segments: &mut Vec<crate::annotate::ByteSegment>,
    display_data: &[u8],
) {
    let mut rewritten = Vec::with_capacity(segments.len());

    for segment in segments.drain(..) {
        if segment.kind == crate::annotate::SegmentKind::EncryptedPayload {
            let before_len = rewritten.len();
            if let Some(data) = display_data.get(segment.start..segment.end) {
                crate::annotate::annotate_data_records(&mut rewritten, segment.start, data);
            }
            if rewritten.len() == before_len {
                rewritten.push(segment);
            }
        } else {
            rewritten.push(segment);
        }
    }

    *segments = rewritten;
}

#[cfg(all(feature = "std", feature = "decryption"))]
fn decrypted_wired_hexview_data(data: &[u8], key: &[u8; 16]) -> Option<Vec<u8>> {
    let parsed_data = MbusData::<frames::WiredFrame>::try_from(data).ok()?;
    let user_data = parsed_data.user_data.as_ref()?;
    let variable_data_len = user_data.variable_data_len();
    if variable_data_len == 0 {
        return None;
    }

    let mut decrypted_buffer = [0u8; 256];
    let decrypted_len =
        decrypt_variable_data_with_key(user_data, None, key, &mut decrypted_buffer)?;

    let user_data_len = match parsed_data.frame {
        frames::WiredFrame::LongFrame {
            data: user_data_slice,
            ..
        }
        | frames::WiredFrame::ControlFrame {
            data: user_data_slice,
            ..
        } => user_data_slice.len(),
        _ => return None,
    };

    let payload_start = 6 + user_data_len.checked_sub(variable_data_len)?;
    let payload_end = payload_start + variable_data_len;
    let mut display_data = data.to_vec();
    let decrypted_data = decrypted_buffer.get(..decrypted_len)?;
    display_data.splice(payload_start..payload_end, decrypted_data.iter().copied());

    let length = display_data.len().checked_sub(6)?;
    if length > u8::MAX as usize || display_data.len() < 6 {
        return None;
    }
    *display_data.get_mut(1)? = length as u8;
    *display_data.get_mut(2)? = length as u8;

    let checksum_index = display_data.len().checked_sub(2)?;
    let checksum = display_data
        .get(4..checksum_index)?
        .iter()
        .fold(0u8, |acc, byte| acc.wrapping_add(*byte));
    *display_data.get_mut(checksum_index)? = checksum;

    Some(display_data)
}

#[cfg(all(feature = "std", feature = "decryption"))]
fn decrypted_wireless_hexview_data(data: &[u8], key: &[u8; 16]) -> Option<Vec<u8>> {
    let mut crc_buf = [0u8; 512];
    let wireless_data =
        wireless_mbus_link_layer::strip_format_a_crcs(data, &mut crc_buf).unwrap_or(data);
    let parsed_data =
        MbusData::<wireless_mbus_link_layer::WirelessFrame>::try_from(wireless_data).ok()?;
    let user_data = parsed_data.user_data.as_ref()?;
    let variable_data_len = user_data.variable_data_len();
    if variable_data_len == 0 {
        return None;
    }

    let mut decrypted_buffer = [0u8; 256];
    let manufacturer_id = &parsed_data.frame.manufacturer_id;
    let decrypted_len = decrypt_variable_data_with_key(
        user_data,
        Some(manufacturer_id),
        key,
        &mut decrypted_buffer,
    )?;

    let app_data_len = parsed_data.frame.data.len();
    let app_start = wireless_data.len().checked_sub(app_data_len)?;
    let payload_start = app_start + app_data_len.checked_sub(variable_data_len)?;
    let payload_end = payload_start + variable_data_len;

    let mut display_data = wireless_data.to_vec();
    let decrypted_data = decrypted_buffer.get(..decrypted_len)?;
    display_data.splice(payload_start..payload_end, decrypted_data.iter().copied());
    if let Some(length) = display_data.len().checked_sub(1) {
        if length <= u8::MAX as usize {
            if let Some(length_byte) = display_data.get_mut(0) {
                *length_byte = length as u8;
            }
        }
    }

    Some(display_data)
}

#[cfg(all(feature = "std", feature = "decryption"))]
fn decrypt_variable_data_with_key(
    user_data: &user_data::UserDataBlock<'_>,
    wireless_manufacturer_id: Option<&wireless_mbus_link_layer::ManufacturerId>,
    key: &[u8; 16],
    output: &mut [u8],
) -> Option<usize> {
    use user_data::UserDataBlock;

    let mut provider = crate::decryption::StaticKeyProvider::<1>::new();
    match user_data {
        UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        } => {
            if !long_tpl_header.is_encrypted() {
                return None;
            }
            let manufacturer = long_tpl_header.manufacturer.as_ref().ok()?;
            provider
                .add_key(
                    manufacturer.to_id(),
                    long_tpl_header.identification_number.number,
                    *key,
                )
                .ok()?;
            user_data.decrypt_variable_data(&provider, output).ok()
        }
        UserDataBlock::VariableDataStructureWithShortTplHeader {
            short_tpl_header, ..
        } => {
            if !short_tpl_header.is_encrypted() {
                return None;
            }
            let manufacturer_id = wireless_manufacturer_id?;
            provider
                .add_key(
                    manufacturer_id.manufacturer_code.to_id(),
                    manufacturer_id.identification_number.number,
                    *key,
                )
                .ok()?;
            user_data
                .decrypt_variable_data_with_context(
                    &provider,
                    manufacturer_id.manufacturer_code,
                    manufacturer_id.identification_number.number,
                    manufacturer_id.version,
                    manufacturer_id.device_type,
                    output,
                )
                .ok()
        }
        _ => None,
    }
}

#[cfg(feature = "std")]
#[must_use]
fn parse_to_annotated_text(input: &str) -> String {
    let data = clean_and_convert(input);
    match crate::annotate::annotate_and_render(&data) {
        Ok(text) => text,
        Err(e) => format!("Error: {}", e),
    }
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

        let expected = "FrameType,Function,Address,Identification Number,Manufacturer,Manufacturer Name,Access Number,Status,Security Mode,Version,Device Type,DataPoint1_Value,DataPoint1_Info,DataPoint2_Value,DataPoint2_Info,DataPoint3_Value,DataPoint3_Info,DataPoint4_Value,DataPoint4_Info,DataPoint5_Value,DataPoint5_Info,DataPoint6_Value,DataPoint6_Info,DataPoint7_Value,DataPoint7_Info,DataPoint8_Value,DataPoint8_Info,DataPoint9_Value,DataPoint9_Info,DataPoint10_Value,DataPoint10_Info\nLongFrame,\"RspUd (ACD: false, DFC: false)\",Primary (1),02205100,SLB,Schlumberger Industries,0,\"Permanent error, Manufacturer specific 3\",No encryption used,2,Heat Meter (Return),(0)e4[Wh](Energy),\"0,Inst,32-bit Integer\",(3)e-1[m³](Volume),\"0,Inst,BCD 8-digit\",(0)e3[W](Power),\"0,Inst,BCD 6-digit\",(0)e-3[m³h⁻¹](VolumeFlow),\"0,Inst,BCD 6-digit\",(1288)e-1[°C](FlowTemperature),\"0,Inst,BCD 4-digit\",(516)e-1[°C](ReturnTemperature),\"0,Inst,BCD 4-digit\",(7723)e-2[°K](TemperatureDifference),\"0,Inst,BCD 6-digit\",(12/Jan/12)(Date),\"0,Inst,Date Type G\",(3383)[day](OperatingTime),\"0,Inst,16-bit Integer\",\"(Manufacturer Specific: [96, 0])\",\"0,Inst,Special Functions (ManufacturerSpecific)\"\n";

        assert_eq!(csv_output, expected);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_wireless_csv_contains_header_info() {
        // Wireless frame with a short TPL header: the identification number,
        // manufacturer, version and device type come from the link layer and
        // must not be missing from the CSV output.
        let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
        let csv_output = super::parse_to_csv(input, None);

        let mut lines = csv_output.lines();
        let header: Vec<&str> = lines.next().expect("header row").split(',').collect();
        let row = lines.next().expect("data row");
        // Parse the row respecting quoting via a simple check of key fields
        assert!(header.starts_with(&[
            "FrameType",
            "Function",
            "Address",
            "Identification Number",
            "Manufacturer",
            "Manufacturer Name",
            "Access Number",
            "Status",
            "Security Mode",
            "Version",
            "Device Type"
        ]));
        assert!(row.starts_with("Wireless,SndNk,,12345678,ELS,Elster Group,42,No Error(s),"));
        assert!(row.contains("AES-CBC-128; IV ≠ 0"));
        assert!(row.contains(",51,Gas Meter"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_wireless_encrypted_csv_without_key_has_no_garbage_records() {
        let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
        let csv_output = super::parse_to_csv(input, None);
        assert!(!csv_output.contains("DataPoint1_Value"));
        // Header info must still be present even though the payload is encrypted
        assert!(csv_output.contains("12345678"));
    }

    #[cfg(all(feature = "std", feature = "decryption"))]
    #[test]
    fn test_wireless_csv_with_key_contains_records() {
        let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
        let key = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x11,
        ];
        let csv_output = super::parse_to_csv(input, Some(&key));
        assert!(csv_output.contains("12345678"));
        assert!(csv_output.contains("(2850427)e-2[m³](Volume)"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_and_yaml_contain_summary() {
        let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
        let json_output = super::parse_to_json(input, None);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("json output should be valid");
        let summary = parsed.get("summary").expect("json should contain summary");
        assert_eq!(
            summary.get("frame_type").and_then(|v| v.as_str()),
            Some("Wireless")
        );
        assert_eq!(
            summary
                .get("identification_number")
                .and_then(|v| v.as_str()),
            Some("12345678")
        );
        assert_eq!(
            summary
                .get("manufacturer")
                .and_then(|m| m.get("code"))
                .and_then(|v| v.as_str()),
            Some("ELS")
        );
        assert_eq!(
            summary.get("security_mode").and_then(|v| v.as_str()),
            Some("AES-CBC-128; IV ≠ 0")
        );
        assert_eq!(
            summary.get("status").and_then(|v| v.as_str()),
            Some("No Error(s)")
        );
        assert_eq!(
            summary.get("encrypted").and_then(|v| v.as_bool()),
            Some(true)
        );

        let yaml_output = super::parse_to_yaml(input, None);
        assert!(yaml_output.contains("summary:"));
        assert!(yaml_output.contains("frame_type: Wireless"));
        assert!(yaml_output.contains("security_mode: AES-CBC-128; IV ≠ 0"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_all_formats_agree_on_header_fields() {
        // The same header information must be present in every output format.
        let inputs = [
            // wired long frame
            "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16",
            // wireless with short TPL header
            "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A",
            // wireless without TPL header (ELL)
            "1444AE0C7856341201078C2027780B134365877AC5",
        ];
        for input in inputs {
            let output = super::parse_frame_output(input, None).expect("frame should parse");
            let summary = &output.summary;
            let table = super::parse_to_table(input, None);
            let csv = super::parse_to_csv(input, None);
            let json = super::parse_to_json(input, None);
            let yaml = super::parse_to_yaml(input, None);
            for value in [
                summary.identification_number.clone(),
                summary.manufacturer.as_ref().map(|m| m.code.clone()),
                summary.status.clone(),
                summary.security_mode.clone(),
                summary.device_type.clone(),
            ]
            .into_iter()
            .flatten()
            {
                for (format, output) in [
                    ("table", &table),
                    ("csv", &csv),
                    ("json", &json),
                    ("yaml", &yaml),
                ] {
                    assert!(
                        output.contains(&value),
                        "{format} output is missing header value {value:?} for input {input}"
                    );
                }
            }

            // The structured record fields in the JSON summary must agree with
            // the normalized summary the table and CSV render from.
            let parsed: serde_json::Value =
                serde_json::from_str(&json).expect("json output should be valid");
            let json_records = parsed
                .get("summary")
                .and_then(|s| s.get("records"))
                .and_then(|r| r.as_array())
                .expect("json summary should contain records");
            assert_eq!(json_records.len(), summary.records.len());
            for (json_record, record) in json_records.iter().zip(&summary.records) {
                assert_eq!(
                    json_record.get("display").and_then(|v| v.as_str()),
                    Some(record.display.as_str()),
                    "json record display mismatch for input {input}"
                );
                assert_eq!(
                    json_record.get("value").and_then(|v| v.as_f64()),
                    record.value,
                    "json record value mismatch for input {input}"
                );
                assert_eq!(
                    json_record
                        .get("exponent")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as isize),
                    record.exponent,
                    "json record exponent mismatch for input {input}"
                );
                assert_eq!(
                    json_record.get("unit").and_then(|v| v.as_str()),
                    record.unit.as_deref(),
                    "json record unit mismatch for input {input}"
                );
                assert_eq!(
                    json_record.get("quantity").and_then(|v| v.as_str()),
                    record.quantity.as_deref(),
                    "json record quantity mismatch for input {input}"
                );
                // The display string the table/CSV formats use must embed the
                // same structured information.
                if let Some(unit) = &record.unit {
                    assert!(
                        record.display.contains(unit.as_str()),
                        "display {:?} should contain unit {unit:?}",
                        record.display
                    );
                }
                if let Some(exponent) = record.exponent.filter(|e| *e != 0) {
                    assert!(
                        record.display.contains(&format!("e{exponent}")),
                        "display {:?} should contain exponent {exponent}",
                        record.display
                    );
                }
            }
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_records_contain_structured_values() {
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let json_output = super::parse_to_json(input, None);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("json output should be valid");
        let records = parsed
            .get("summary")
            .and_then(|s| s.get("records"))
            .and_then(|r| r.as_array())
            .expect("summary should contain records");

        // "(3)e-1[m³](Volume)" becomes structured fields.
        let volume = records
            .iter()
            .find(|r| r.get("quantity").and_then(|v| v.as_str()) == Some("Volume"))
            .expect("volume record");
        assert_eq!(volume.get("value").and_then(|v| v.as_f64()), Some(3.0));
        assert_eq!(volume.get("exponent").and_then(|v| v.as_i64()), Some(-1));
        assert_eq!(volume.get("unit").and_then(|v| v.as_str()), Some("m³"));
        assert_eq!(
            volume.get("display").and_then(|v| v.as_str()),
            Some("(3)e-1[m³](Volume)")
        );

        // A date record has no numeric value but keeps its display string.
        let date = records
            .iter()
            .find(|r| r.get("quantity").and_then(|v| v.as_str()) == Some("Date"))
            .expect("date record");
        assert!(date.get("value").is_none());
        assert_eq!(
            date.get("display").and_then(|v| v.as_str()),
            Some("(12/Jan/12)(Date)")
        );

        // A manufacturer-specific record has neither value information nor a
        // numeric value.
        let manufacturer_specific = records
            .iter()
            .find(|r| {
                r.get("data_information")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.contains("ManufacturerSpecific"))
            })
            .expect("manufacturer specific record");
        assert!(manufacturer_specific.get("value").is_none());
        assert!(manufacturer_specific.get("exponent").is_none());
        assert!(manufacturer_specific.get("unit").is_none());
        assert!(manufacturer_specific.get("quantity").is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_and_yaml_have_no_manufacturer_info() {
        let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let json_output = super::parse_to_json(input, None);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("json output should be valid");
        assert!(parsed.get("manufacturer_info").is_none());
        // summary.manufacturer carries the same information instead.
        assert_eq!(
            parsed
                .get("summary")
                .and_then(|s| s.get("manufacturer"))
                .and_then(|m| m.get("name"))
                .and_then(|v| v.as_str()),
            Some("Schlumberger Industries")
        );

        let yaml_output = super::parse_to_yaml(input, None);
        assert!(!yaml_output.contains("manufacturer_info:"));
        assert!(yaml_output.contains("name: Schlumberger Industries"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_byte_payloads_serialize_as_hex_strings() {
        // Wireless frame: frame.data must be a compact uppercase hex string.
        let wireless_input = "1444AE0C7856341201078C2027780B134365877AC5";
        let json_output = super::parse_to_json(wireless_input, None);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("json output should be valid");
        let frame_data = parsed
            .get("frame")
            .and_then(|f| f.get("data"))
            .expect("wireless frame should contain data");
        let hex = frame_data.as_str().expect("frame data should be a string");
        assert!(!hex.is_empty());
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hex, hex.to_uppercase());

        // Data records: raw_bytes must be hex strings as well.
        let wired_input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
        let json_output = super::parse_to_json(wired_input, None);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("json output should be valid");
        let records = parsed
            .get("data_records")
            .and_then(|r| r.as_array())
            .expect("data records");
        let first_raw_bytes = records
            .first()
            .and_then(|r| r.get("raw_bytes"))
            .and_then(|v| v.as_str())
            .expect("raw_bytes should be a hex string");
        assert_eq!(first_raw_bytes, "040700000000");

        // Manufacturer-specific data is a hex string instead of a byte array.
        assert!(json_output.contains("\"ManufacturerSpecific\": \"6000\""));
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

    #[cfg(all(feature = "std", feature = "decryption"))]
    #[test]
    fn decrypted_utf8_text_is_not_rendered_as_latin1() {
        let input = "2E44931578563412330333637A2A00202557FB8016CA78E1243700B52E981E1918233AFE5E826DD0D4AD7854C697E7C8EB";
        let key = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x11,
        ];

        for format in ["table", "json", "yaml", "csv"] {
            let output = super::serialize_mbus_data(input, format, Some(&key));
            assert!(
                output.contains("m³"),
                "{format} output should contain the decoded UTF-8 text: {output}"
            );
            assert!(
                !output.contains("mÂ³"),
                "{format} output contains mojibake: {output}"
            );
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_annotated_output() {
        let input = "68 4D 4D 68 08 01 72 01 00 00 00 96 15 01 00 18 00 00 00 0C 78 56 00 00 00 01 FD 1B 00 02 FC 03 48 52 25 74 44 0D 22 FC 03 48 52 25 74 F1 0C 12 FC 03 48 52 25 74 63 11 02 65 B4 09 22 65 86 09 12 65 B7 09 01 72 00 72 65 00 00 B2 01 65 00 00 1F B3 16";
        let output = super::serialize_mbus_data(input, "annotated", None);

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_else(|e| {
            panic!(
                "annotated output should be valid JSON: {}\nOutput: {}",
                e, output
            )
        });

        // Should be an array
        assert!(parsed.is_array(), "annotated output should be a JSON array");
        let segments = parsed.as_array().expect("array");

        // Should cover all 83 bytes
        assert!(!segments.is_empty());

        // First segment should start at 0
        assert_eq!(segments[0].get("start").and_then(|v| v.as_u64()), Some(0));

        // Last segment should end at 83
        let last = segments.last().expect("non-empty");
        assert_eq!(last.get("end").and_then(|v| v.as_u64()), Some(83));

        // Check contiguity
        for window in segments.windows(2) {
            let end = window[0].get("end").and_then(|v| v.as_u64());
            let start = window[1].get("start").and_then(|v| v.as_u64());
            assert_eq!(end, start, "segments should be contiguous");
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_hexview_output_for_ci_78_frame_is_annotated_json() {
        let input = "1444AE0C7856341201078C2027780B134365877AC5";
        let output = super::serialize_mbus_data(input, "hexview", None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_else(|e| {
            panic!(
                "hexview output should be valid annotated JSON: {}\nOutput: {}",
                e, output
            )
        });
        let segments = parsed
            .as_array()
            .expect("hexview output should be a JSON array");

        assert!(segments.iter().any(|seg| {
            seg.get("kind").and_then(|v| v.as_str()) == Some("CiField")
                && seg
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .is_some_and(|detail| detail.contains("0x78"))
        }));
        assert!(segments.iter().any(|seg| {
            seg.get("kind").and_then(|v| v.as_str()) == Some("DataPayload")
                && seg.get("detail").and_then(|v| v.as_str()) == Some("876543")
        }));
        assert!(!segments.iter().any(|seg| {
            seg.get("kind").and_then(|v| v.as_str()) == Some("Unknown")
                || seg
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .is_some_and(|detail| detail.contains("Unparseable data record bytes"))
        }));
    }

    #[cfg(all(feature = "std", feature = "decryption"))]
    #[test]
    fn test_hexview_with_key_displays_decrypted_wireless_payload() {
        let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
        let key = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x11,
        ];
        let output = super::serialize_mbus_data(input, "hexview", Some(&key));

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_else(|e| {
            panic!(
                "decrypted hexview output should be valid JSON: {}\nOutput: {}",
                e, output
            )
        });
        assert_eq!(
            parsed.get("decrypted").and_then(|v| v.as_bool()),
            Some(true)
        );

        let bytes = parsed
            .get("bytes")
            .and_then(|v| v.as_array())
            .expect("decrypted hexview should include display bytes");
        let display_hex = bytes
            .iter()
            .map(|byte| format!("{:02X}", byte.as_u64().unwrap_or_default()))
            .collect::<String>();
        assert!(
            display_hex.contains("0C1427048502046D32371F1502FD170000"),
            "display bytes should contain decrypted record bytes: {display_hex}"
        );

        let segments = parsed
            .get("segments")
            .and_then(|v| v.as_array())
            .expect("decrypted hexview should include annotation segments");
        assert!(segments.iter().any(|seg| {
            seg.get("kind").and_then(|v| v.as_str()) == Some("DataPayload")
                && seg
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .is_some_and(|detail| detail.contains("2850427"))
        }));
        assert!(
            !segments
                .iter()
                .any(|seg| seg.get("kind").and_then(|v| v.as_str()) == Some("EncryptedPayload")),
            "decrypted hexview should annotate decrypted records, not ciphertext payloads"
        );
    }
}
