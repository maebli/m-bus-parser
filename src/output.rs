//! Typed, harmonized output contract for `std` consumers.
//!
//! The low-level parsers remain allocation-free and `no_std`; this module owns
//! the presentation model used by the CLI, Python, WebAssembly, and text
//! renderers.

use core::str::FromStr;
use std::fmt;

use m_bus_core::SecurityMode;
use serde::Serialize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use wired_mbus_link_layer as wired;
use wireless_mbus_link_layer as wireless;

use crate::mbus_data::MbusData;
use crate::user_data;
use crate::user_data::data_information::{DataFieldCoding, DataType, Month, SingleEveryOrInvalid};
use crate::user_data::value_information::{Unit, UnitName};

const SCHEMA_VERSION: u8 = 1;
const DEFAULT_TABLE_WIDTH: usize = 100;
const MINIMUM_TABLE_WIDTH: usize = 32;

/// Canonical output formats supported by the typed rendering API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
    Csv,
    Mermaid,
    Xml,
    Annotated,
    AnnotatedText,
}

impl OutputFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Csv => "csv",
            Self::Mermaid => "mermaid",
            Self::Xml => "xml",
            Self::Annotated => "annotated",
            Self::AnnotatedText => "annotated-text",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OutputFormat {
    type Err = OutputError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "table" | "table_format" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "csv" => Ok(Self::Csv),
            "mermaid" => Ok(Self::Mermaid),
            "xml" => Ok(Self::Xml),
            "annotated" | "hexview" => Ok(Self::Annotated),
            "annotated-text" => Ok(Self::AnnotatedText),
            _ => Err(OutputError::UnsupportedFormat {
                format: value.to_string(),
            }),
        }
    }
}

/// Options controlling protocol decoding.
#[derive(Debug, Clone)]
pub struct DecodeOptions {
    pub key: Option<[u8; 16]>,
    pub include_enrichment: bool,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            key: None,
            include_enrichment: true,
        }
    }
}

/// Options controlling decoding and human-oriented rendering.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub decode: DecodeOptions,
    pub table_width: Option<usize>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            decode: DecodeOptions::default(),
            table_width: Some(DEFAULT_TABLE_WIDTH),
        }
    }
}

/// Stable error taxonomy shared by Rust and language bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputError {
    EmptyInput,
    InvalidHex {
        index: usize,
        message: String,
    },
    InvalidFrame {
        wired: String,
        wireless: String,
    },
    UnsupportedFormat {
        format: String,
    },
    InvalidOption {
        option: &'static str,
        message: String,
    },
    Decryption {
        code: &'static str,
        message: String,
    },
    Rendering {
        code: &'static str,
        message: String,
    },
    Serialization {
        format: &'static str,
        message: String,
    },
}

impl OutputError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyInput => "input.empty",
            Self::InvalidHex { .. } => "input.invalid_hex",
            Self::InvalidFrame { .. } => "frame.invalid",
            Self::UnsupportedFormat { .. } => "format.unsupported",
            Self::InvalidOption { .. } => "option.invalid",
            Self::Decryption { code, .. } | Self::Rendering { code, .. } => code,
            Self::Serialization { .. } => "render.serialization",
        }
    }

    #[must_use]
    pub const fn layer(&self) -> &'static str {
        match self {
            Self::EmptyInput | Self::InvalidHex { .. } => "input",
            Self::InvalidFrame { .. } => "link",
            Self::UnsupportedFormat { .. }
            | Self::InvalidOption { .. }
            | Self::Serialization { .. } => "render",
            Self::Decryption { .. } => "security",
            Self::Rendering { .. } => "application",
        }
    }

    #[must_use]
    pub const fn byte_offset(&self) -> Option<usize> {
        match self {
            Self::InvalidHex { index, .. } => Some(*index),
            _ => None,
        }
    }
}

impl fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("input must not be empty"),
            Self::InvalidHex { index, message } => {
                write!(formatter, "invalid hexadecimal input at character {index}: {message}")
            }
            Self::InvalidFrame { wired, wireless } => write!(
                formatter,
                "data is not a valid wired or wireless M-Bus frame (wired: {wired}; wireless: {wireless})"
            ),
            Self::UnsupportedFormat { format } => write!(
                formatter,
                "unsupported output format {format:?}; expected table, json, yaml, csv, mermaid, xml, annotated, or annotated-text"
            ),
            Self::InvalidOption { option, message } => {
                write!(formatter, "invalid {option}: {message}")
            }
            Self::Decryption { message, .. }
            | Self::Rendering { message, .. }
            | Self::Serialization { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for OutputError {}

#[derive(Debug, Clone, Serialize)]
pub struct DecodedOutput {
    pub schema_version: u8,
    pub decode_state: String,
    pub protocol: String,
    pub frame: FrameOutput,
    pub meter: MeterOutput,
    pub transport: TransportOutput,
    pub security: SecurityOutput,
    pub records: Vec<RecordOutput>,
    pub raw: RawOutput,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<EnrichmentOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameOutput {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MeterIdentity {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type_code: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeterOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<MeterIdentity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alternate_identities: Vec<MeterIdentity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourcedU8 {
    pub source: String,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourcedStatus {
    pub source: String,
    pub raw: String,
    pub display: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationOutput {
    pub source: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EllOutput {
    pub communication_control: String,
    pub access_number: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_crc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_kind: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub access_numbers: Vec<SourcedU8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<SourcedStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<ConfigurationOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ell: Option<EllOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityOutput {
    pub payload_protection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_code: Option<u8>,
    pub decryption_state: String,
    pub key_supplied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_payload_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decrypted_payload_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordOutput {
    pub index: usize,
    pub function: String,
    pub storage_number: u64,
    pub tariff: u64,
    pub subunit: u64,
    pub quantities: Vec<String>,
    pub value: ValueOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitOutput>,
    pub data_coding: String,
    pub header_hex: String,
    pub data_hex: String,
    pub record_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueOutput {
    pub kind: String,
    pub display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub significand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_power10: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_power10: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<serde_json::Value>,
    pub raw_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitComponent {
    pub name: String,
    pub exponent: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitOutput {
    pub display: String,
    pub components: Vec<UnitComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ucum: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    pub original_frame_hex: String,
    pub normalized_frame_hex: String,
    pub byte_length: usize,
    pub format_a_crc_stripped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub layer: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichmentOutput {
    pub source: String,
    pub manufacturer_code: String,
    pub manufacturer_name: String,
    pub manufacturer_website: String,
    pub manufacturer_description: String,
}

#[derive(Clone)]
struct SecurityContext {
    encrypted: bool,
    mode: Option<SecurityMode>,
    ell_encrypted: bool,
    decrypted: bool,
    key_supplied: bool,
    decrypted_payload: Option<Vec<u8>>,
}

/// Decode a human-readable hexadecimal frame.
pub fn decode_hex(input: &str, options: &DecodeOptions) -> Result<DecodedOutput, OutputError> {
    let bytes = decode_hex_bytes(input)?;
    let mut decoded = decode_bytes(&bytes, options)?;
    decoded.raw.input = Some(input.to_string());
    Ok(decoded)
}

/// Decode an already materialized frame.
pub fn decode_bytes(data: &[u8], options: &DecodeOptions) -> Result<DecodedOutput, OutputError> {
    if data.is_empty() {
        return Err(OutputError::EmptyInput);
    }

    let wired_error = match MbusData::<wired::WiredFrame>::try_from(data) {
        Ok(mut parsed) => {
            let mut decrypted_buffer = [0u8; 512];
            let security = prepare_security(
                parsed.user_data.as_ref(),
                None,
                options.key.as_ref(),
                &mut decrypted_buffer,
            )?;
            if let Some(decrypted) = security.decrypted_payload.as_deref() {
                parsed.data_records = decrypted_records(parsed.user_data.as_ref(), decrypted);
            } else if security.encrypted {
                parsed.data_records = None;
            }
            return Ok(build_wired_output(
                data,
                &parsed,
                security.clone(),
                options.include_enrichment,
            ));
        }
        Err(error) => error.to_string(),
    };

    let mut crc_buffer = [0u8; 512];
    let normalized = wireless::strip_format_a_crcs(data, &mut crc_buffer).unwrap_or(data);
    let stripped = normalized.len() != data.len();
    match MbusData::<wireless::WirelessFrame>::try_from(normalized) {
        Ok(mut parsed) => {
            let mut decrypted_buffer = [0u8; 512];
            let security = prepare_security(
                parsed.user_data.as_ref(),
                Some(&parsed.frame.manufacturer_id),
                options.key.as_ref(),
                &mut decrypted_buffer,
            )?;
            if let Some(decrypted) = security.decrypted_payload.as_deref() {
                parsed.data_records = decrypted_records(parsed.user_data.as_ref(), decrypted);
            } else if security.encrypted {
                parsed.data_records = None;
            }
            Ok(build_wireless_output(
                data,
                normalized,
                stripped,
                &parsed,
                security.clone(),
                options.include_enrichment,
            ))
        }
        Err(error) => Err(OutputError::InvalidFrame {
            wired: wired_error,
            wireless: format!("{error:?}"),
        }),
    }
}

/// Render a human-readable hexadecimal frame.
pub fn render_hex(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    let bytes = decode_hex_bytes(input)?;
    render_bytes(&bytes, format, options)
}

/// Render an already materialized frame.
pub fn render_bytes(
    data: &[u8],
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    match format {
        OutputFormat::Xml => {
            crate::rscada_xml::render_from_bytes(data, options.decode.key.as_ref()).map_err(
                |message| OutputError::Rendering {
                    code: "xml.unsupported",
                    message,
                },
            )
        }
        OutputFormat::Annotated => {
            crate::mbus_data::render_annotated_bytes(data, options.decode.key.as_ref())
        }
        OutputFormat::AnnotatedText => {
            crate::annotate::annotate_and_render(data).map_err(|error| OutputError::Rendering {
                code: "annotation.failed",
                message: error.to_string(),
            })
        }
        _ => {
            let decoded = decode_bytes(data, &options.decode)?;
            match format {
                OutputFormat::Json => serde_json::to_string_pretty(&decoded).map_err(|error| {
                    OutputError::Serialization {
                        format: "json",
                        message: error.to_string(),
                    }
                }),
                OutputFormat::Yaml => {
                    serde_yaml::to_string(&decoded).map_err(|error| OutputError::Serialization {
                        format: "yaml",
                        message: error.to_string(),
                    })
                }
                OutputFormat::Csv => render_csv(&decoded),
                OutputFormat::Table => {
                    render_table(&decoded, options.table_width.unwrap_or(DEFAULT_TABLE_WIDTH))
                }
                OutputFormat::Mermaid => Ok(render_mermaid(&decoded)),
                _ => unreachable!("specialized formats handled above"),
            }
        }
    }
}

/// Strictly parse supported hexadecimal input spellings.
pub fn decode_hex_bytes(input: &str) -> Result<Vec<u8>, OutputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(OutputError::EmptyInput);
    }

    if trimmed
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        if !trimmed.len().is_multiple_of(2) {
            return Err(OutputError::InvalidHex {
                index: trimmed.len() - 1,
                message: "compact hexadecimal input must contain an even number of digits"
                    .to_string(),
            });
        }
        return (0..trimmed.len())
            .step_by(2)
            .map(|index| {
                u8::from_str_radix(&trimmed[index..index + 2], 16).map_err(|error| {
                    OutputError::InvalidHex {
                        index,
                        message: error.to_string(),
                    }
                })
            })
            .collect();
    }

    let mut bytes = Vec::new();
    let mut token = String::new();
    let mut token_start = 0usize;
    let mut saw_separator = true;
    for (index, character) in trimmed.char_indices() {
        if character.is_ascii_whitespace() || matches!(character, ':' | '-') {
            if !token.is_empty() {
                push_hex_token(&mut bytes, &token, token_start)?;
                token.clear();
            }
            saw_separator = true;
            continue;
        }
        if saw_separator {
            token_start = index;
            saw_separator = false;
        }
        token.push(character);
    }
    if !token.is_empty() {
        push_hex_token(&mut bytes, &token, token_start)?;
    }
    if bytes.is_empty() {
        return Err(OutputError::EmptyInput);
    }
    Ok(bytes)
}

fn push_hex_token(output: &mut Vec<u8>, token: &str, index: usize) -> Result<(), OutputError> {
    let digits = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
        .unwrap_or(token);
    if digits.len() != 2
        || !digits
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(OutputError::InvalidHex {
            index,
            message: format!("expected a complete byte token, found {token:?}"),
        });
    }
    let byte = u8::from_str_radix(digits, 16).map_err(|error| OutputError::InvalidHex {
        index,
        message: error.to_string(),
    })?;
    output.push(byte);
    Ok(())
}

fn prepare_security(
    user_data: Option<&user_data::UserDataBlock<'_>>,
    wireless_id: Option<&wireless::ManufacturerId>,
    key: Option<&[u8; 16]>,
    output: &mut [u8],
) -> Result<SecurityContext, OutputError> {
    let (encrypted, mode, ell_encrypted) = security_fields(user_data);
    if !encrypted {
        return Ok(SecurityContext {
            encrypted: false,
            mode,
            ell_encrypted,
            decrypted: false,
            key_supplied: key.is_some(),
            decrypted_payload: None,
        });
    }
    let Some(key) = key else {
        return Ok(SecurityContext {
            encrypted,
            mode,
            ell_encrypted,
            decrypted: false,
            key_supplied: false,
            decrypted_payload: None,
        });
    };
    if ell_encrypted && mode.is_none() {
        return Err(OutputError::Decryption {
            code: "security.unsupported_mode",
            message: "decryption was requested for an ELL-protected payload, which is not currently supported"
                .to_string(),
        });
    }

    #[cfg(feature = "decryption")]
    {
        let block = user_data.ok_or_else(|| OutputError::Decryption {
            code: "security.unknown_state",
            message: "encrypted payload does not expose an application data block".to_string(),
        })?;
        let len = decrypt_user_data(block, wireless_id, key, output).map_err(|error| {
            let code = match error {
                crate::decryption::DecryptionError::UnsupportedMode(_) => {
                    "security.unsupported_mode"
                }
                crate::decryption::DecryptionError::InvalidKeyLength => "security.invalid_key",
                _ => "security.decryption_failed",
            };
            OutputError::Decryption {
                code,
                message: error.to_string(),
            }
        })?;
        let decrypted = output
            .get(..len)
            .ok_or_else(|| OutputError::Decryption {
                code: "security.decryption_failed",
                message: "decryption returned an invalid payload length".to_string(),
            })?
            .to_vec();
        Ok(SecurityContext {
            encrypted,
            mode,
            ell_encrypted,
            decrypted: true,
            key_supplied: true,
            decrypted_payload: Some(decrypted),
        })
    }

    #[cfg(not(feature = "decryption"))]
    {
        let _ = (user_data, wireless_id, key, output);
        Err(OutputError::Decryption {
            code: "security.feature_unavailable",
            message: "this build does not include decryption support".to_string(),
        })
    }
}

fn security_fields(
    user_data: Option<&user_data::UserDataBlock<'_>>,
) -> (bool, Option<SecurityMode>, bool) {
    use user_data::UserDataBlock;
    match user_data {
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        }) => {
            let mode = long_tpl_header
                .short_tpl_header
                .configuration_field
                .security_mode();
            (
                !matches!(mode, SecurityMode::NoEncryption),
                Some(mode),
                false,
            )
        }
        Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
            extended_link_layer,
            short_tpl_header,
            ..
        }) => {
            let ell_encrypted = extended_link_layer
                .as_ref()
                .is_some_and(|ell| ell.encryption.is_some());
            let mode = short_tpl_header.configuration_field.security_mode();
            if ell_encrypted && matches!(mode, SecurityMode::NoEncryption) {
                (true, None, true)
            } else {
                (
                    !matches!(mode, SecurityMode::NoEncryption),
                    Some(mode),
                    ell_encrypted,
                )
            }
        }
        _ => (false, None, false),
    }
}

#[cfg(feature = "decryption")]
pub(crate) fn decrypt_user_data(
    user_data: &user_data::UserDataBlock<'_>,
    wireless_id: Option<&wireless::ManufacturerId>,
    key: &[u8; 16],
    output: &mut [u8],
) -> Result<usize, crate::decryption::DecryptionError> {
    use user_data::UserDataBlock;
    let mut provider = crate::decryption::StaticKeyProvider::<1>::new();
    match user_data {
        UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        } => {
            let manufacturer = long_tpl_header
                .manufacturer
                .map_err(|_| crate::decryption::DecryptionError::UnknownEncryptionState)?;
            provider.add_key(
                manufacturer.to_id(),
                long_tpl_header.identification_number.number,
                *key,
            )?;
            user_data.decrypt_variable_data(&provider, output)
        }
        UserDataBlock::VariableDataStructureWithShortTplHeader {
            short_tpl_header, ..
        } => {
            if matches!(
                short_tpl_header.configuration_field.security_mode(),
                SecurityMode::NoEncryption
            ) {
                return Err(crate::decryption::DecryptionError::UnsupportedMode(
                    short_tpl_header.configuration_field.security_mode(),
                ));
            }
            let id =
                wireless_id.ok_or(crate::decryption::DecryptionError::UnknownEncryptionState)?;
            provider.add_key(
                id.manufacturer_code.to_id(),
                id.identification_number.number,
                *key,
            )?;
            user_data.decrypt_variable_data_with_context(
                &provider,
                id.manufacturer_code,
                id.identification_number.number,
                id.version,
                id.device_type,
                output,
            )
        }
        _ => Err(crate::decryption::DecryptionError::UnknownEncryptionState),
    }
}

fn decrypted_records<'a>(
    block: Option<&'a user_data::UserDataBlock<'a>>,
    decrypted: &'a [u8],
) -> Option<user_data::DataRecords<'a>> {
    match block {
        Some(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header,
            ..
        }) => Some(user_data::DataRecords::new(
            decrypted,
            Some(long_tpl_header),
        )),
        Some(
            user_data::UserDataBlock::VariableDataStructureWithShortTplHeader { .. }
            | user_data::UserDataBlock::VariableDataStructureWithoutTplHeader { .. },
        ) => Some(user_data::DataRecords::new(decrypted, None)),
        _ => None,
    }
}

fn build_wired_output(
    original: &[u8],
    parsed: &MbusData<wired::WiredFrame<'_>>,
    security_context: SecurityContext,
    include_enrichment: bool,
) -> DecodedOutput {
    let (kind, function, address, payload) = match &parsed.frame {
        wired::WiredFrame::LongFrame {
            function,
            address,
            data,
        } => (
            "long",
            Some(function.to_string()),
            Some(address.to_string()),
            Some(*data),
        ),
        wired::WiredFrame::ControlFrame {
            function,
            address,
            data,
        } => (
            "control",
            Some(function.to_string()),
            Some(address.to_string()),
            Some(*data),
        ),
        wired::WiredFrame::ShortFrame { function, address } => (
            "short",
            Some(function.to_string()),
            Some(address.to_string()),
            None,
        ),
        wired::WiredFrame::SingleCharacter { .. } => ("single_character", None, None, None),
        _ => ("unknown", None, None, None),
    };

    build_output(
        "wired",
        FrameOutput {
            kind: kind.to_string(),
            function,
            address,
            control_field: None,
        },
        original,
        original,
        false,
        payload,
        parsed.user_data.as_ref(),
        parsed.data_records.as_ref(),
        parsed.application_error.as_ref(),
        None,
        security_context,
        include_enrichment,
    )
}

fn build_wireless_output(
    original: &[u8],
    normalized: &[u8],
    format_a_crc_stripped: bool,
    parsed: &MbusData<wireless::WirelessFrame<'_>>,
    security_context: SecurityContext,
    include_enrichment: bool,
) -> DecodedOutput {
    build_output(
        "wireless",
        FrameOutput {
            kind: "wireless".to_string(),
            function: parsed.frame.function.map(|function| function.to_string()),
            address: None,
            control_field: Some(format!("{:02X}", parsed.frame.control_field)),
        },
        original,
        normalized,
        format_a_crc_stripped,
        Some(parsed.frame.data),
        parsed.user_data.as_ref(),
        parsed.data_records.as_ref(),
        parsed.application_error.as_ref(),
        Some(&parsed.frame.manufacturer_id),
        security_context,
        include_enrichment,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_output(
    protocol: &str,
    frame: FrameOutput,
    original: &[u8],
    normalized: &[u8],
    format_a_crc_stripped: bool,
    payload: Option<&[u8]>,
    user_data: Option<&user_data::UserDataBlock<'_>>,
    records: Option<&user_data::DataRecords<'_>>,
    application_error: Option<&m_bus_core::ApplicationLayerError>,
    wireless_id: Option<&wireless::ManufacturerId>,
    security_context: SecurityContext,
    include_enrichment: bool,
) -> DecodedOutput {
    let mut diagnostics = Vec::new();
    if let Some(error) = application_error {
        diagnostics.push(Diagnostic {
            severity: "warning".to_string(),
            code: "application.partial".to_string(),
            layer: "application".to_string(),
            message: error.to_string(),
            offset_start: None,
            offset_end: None,
        });
    }
    if security_context.encrypted && !security_context.decrypted {
        diagnostics.push(Diagnostic {
            severity: "warning".to_string(),
            code: "security.key_missing".to_string(),
            layer: "security".to_string(),
            message: "payload is encrypted; header metadata is available but records require a key"
                .to_string(),
            offset_start: None,
            offset_end: None,
        });
    } else if security_context.decrypted {
        diagnostics.push(Diagnostic {
            severity: "info".to_string(),
            code: "security.decrypted_unverified".to_string(),
            layer: "security".to_string(),
            message: "AES-CBC payload was transformed but cannot be authenticated".to_string(),
            offset_start: None,
            offset_end: None,
        });
    } else if security_context.key_supplied {
        diagnostics.push(Diagnostic {
            severity: "info".to_string(),
            code: "security.key_unused".to_string(),
            layer: "security".to_string(),
            message: "a key was supplied for a cleartext frame and was not used".to_string(),
            offset_start: None,
            offset_end: None,
        });
    }

    let (record_outputs, record_error) = collect_records(records);
    if let Some((offset, message)) = record_error.as_ref() {
        diagnostics.push(Diagnostic {
            severity: "warning".to_string(),
            code: "application.record_partial".to_string(),
            layer: "application".to_string(),
            message: message.clone(),
            offset_start: Some(*offset),
            offset_end: None,
        });
    }

    let (meter, transport) = meter_and_transport(user_data, wireless_id);
    let enrichment = if include_enrichment {
        meter
            .identity
            .as_ref()
            .and_then(|identity| identity.manufacturer_code.as_deref())
            .and_then(enrichment_for)
    } else {
        None
    };

    let encrypted_payload_hex = security_context
        .encrypted
        .then(|| payload.map(hex_string))
        .flatten();
    let decrypted_payload_hex = security_context
        .decrypted_payload
        .as_deref()
        .map(hex_string);
    let decryption_state = if !security_context.encrypted {
        "not_applicable"
    } else if security_context.decrypted {
        "decrypted_unverified"
    } else {
        "key_missing"
    };
    let payload_protection = if security_context.encrypted {
        "encrypted"
    } else if security_context.ell_encrypted {
        "unknown"
    } else {
        "cleartext"
    };
    let mode = security_context.mode;
    let partial = application_error.is_some()
        || record_error.is_some()
        || (security_context.encrypted && !security_context.decrypted);

    DecodedOutput {
        schema_version: SCHEMA_VERSION,
        decode_state: if partial { "partial" } else { "complete" }.to_string(),
        protocol: protocol.to_string(),
        frame,
        meter,
        transport,
        security: SecurityOutput {
            payload_protection: payload_protection.to_string(),
            mode: mode.map(|value| value.to_string()),
            mode_code: mode.map(|value| value.to_bits()),
            decryption_state: decryption_state.to_string(),
            key_supplied: security_context.key_supplied,
            encrypted_payload_hex,
            decrypted_payload_hex,
        },
        records: record_outputs,
        raw: RawOutput {
            input: None,
            original_frame_hex: hex_string(original),
            normalized_frame_hex: hex_string(normalized),
            byte_length: original.len(),
            format_a_crc_stripped,
            payload_hex: payload.map(hex_string),
        },
        diagnostics,
        enrichment,
    }
}

fn meter_and_transport(
    user_data: Option<&user_data::UserDataBlock<'_>>,
    wireless_id: Option<&wireless::ManufacturerId>,
) -> (MeterOutput, TransportOutput) {
    use user_data::UserDataBlock;
    let link_identity = wireless_id.map(identity_from_wireless);
    let mut primary = link_identity.clone();
    let mut alternates = Vec::new();
    let mut transport = TransportOutput {
        header_kind: None,
        access_numbers: Vec::new(),
        statuses: Vec::new(),
        configuration: None,
        ell: None,
    };

    match user_data {
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            extended_link_layer,
            long_tpl_header,
            ..
        }) => {
            let application_identity = identity_from_long(long_tpl_header);
            if let Some(link) = link_identity {
                if link != application_identity {
                    alternates.push(link);
                }
            }
            primary = Some(application_identity);
            transport.header_kind = Some("long_tpl".to_string());
            add_short_transport(
                &mut transport,
                "tpl.long",
                &long_tpl_header.short_tpl_header,
            );
            transport.ell = extended_link_layer.as_ref().map(ell_output);
        }
        Some(UserDataBlock::VariableDataStructureWithShortTplHeader {
            extended_link_layer,
            short_tpl_header,
            ..
        }) => {
            let synthetic = extended_link_layer
                .as_ref()
                .is_some_and(|ell| ell.encryption.is_some())
                && matches!(
                    short_tpl_header.configuration_field.security_mode(),
                    SecurityMode::NoEncryption
                );
            transport.header_kind = Some(if synthetic { "ell" } else { "short_tpl" }.to_string());
            if synthetic {
                if let Some(ell) = extended_link_layer {
                    transport.access_numbers.push(SourcedU8 {
                        source: "ell".to_string(),
                        value: ell.access_number,
                    });
                }
            } else {
                add_short_transport(&mut transport, "tpl.short", short_tpl_header);
            }
            transport.ell = extended_link_layer.as_ref().map(ell_output);
        }
        Some(UserDataBlock::VariableDataStructureWithoutTplHeader {
            extended_link_layer,
            ..
        }) => {
            transport.header_kind = Some(
                if extended_link_layer.is_some() {
                    "ell"
                } else {
                    "none"
                }
                .to_string(),
            );
            if let Some(ell) = extended_link_layer {
                transport.access_numbers.push(SourcedU8 {
                    source: "ell".to_string(),
                    value: ell.access_number,
                });
                transport.ell = Some(ell_output(ell));
            }
        }
        Some(UserDataBlock::FixedDataStructure {
            identification_number,
            access_number,
            status,
            device_type_and_unit,
            ..
        }) => {
            primary = Some(MeterIdentity {
                source: "application.fixed".to_string(),
                id: Some(identification_number.to_string()),
                manufacturer_code: None,
                version: None,
                device_type: Some(format!("fixed type/unit 0x{device_type_and_unit:04X}")),
                device_type_code: None,
            });
            transport.header_kind = Some("fixed".to_string());
            transport.access_numbers.push(SourcedU8 {
                source: "application.fixed".to_string(),
                value: *access_number,
            });
            transport.statuses.push(SourcedStatus {
                source: "application.fixed".to_string(),
                raw: format!("{:02X}", status.bits()),
                display: status.to_string(),
            });
        }
        Some(UserDataBlock::ResetAtApplicationLevel { .. }) => {
            transport.header_kind = Some("application_reset".to_string());
        }
        None => {}
        Some(_) => {}
    }

    (
        MeterOutput {
            identity: primary,
            alternate_identities: alternates,
        },
        transport,
    )
}

fn add_short_transport(
    transport: &mut TransportOutput,
    source: &str,
    header: &user_data::ShortTplHeader,
) {
    transport.access_numbers.push(SourcedU8 {
        source: source.to_string(),
        value: header.access_number,
    });
    transport.statuses.push(SourcedStatus {
        source: source.to_string(),
        raw: format!("{:02X}", header.status.bits()),
        display: header.status.to_string(),
    });
    transport.configuration = Some(ConfigurationOutput {
        source: source.to_string(),
        raw: format!("{:04X}", header.configuration_field.raw()),
    });
}

fn ell_output(ell: &user_data::extended_link_layer::ExtendedLinkLayer) -> EllOutput {
    EllOutput {
        communication_control: format!("{:02X}", ell.communication_control),
        access_number: ell.access_number,
        receiver_manufacturer: ell
            .receiver_address
            .as_ref()
            .map(|address| format!("{:04X}", address.manufacturer)),
        receiver_address: ell
            .receiver_address
            .as_ref()
            .map(|address| hex_string(&address.address)),
        session_number: ell
            .encryption
            .as_ref()
            .map(|encryption| hex_string(&encryption.session_number)),
        payload_crc: ell
            .encryption
            .as_ref()
            .map(|encryption| format!("{:04X}", encryption.payload_crc)),
    }
}

fn identity_from_wireless(id: &wireless::ManufacturerId) -> MeterIdentity {
    MeterIdentity {
        source: "link.wireless".to_string(),
        id: Some(id.identification_number.to_string()),
        manufacturer_code: Some(id.manufacturer_code.to_string()),
        version: Some(id.version),
        device_type: Some(id.device_type.to_string()),
        device_type_code: Some(id.device_type.into()),
    }
}

fn identity_from_long(header: &user_data::LongTplHeader) -> MeterIdentity {
    MeterIdentity {
        source: "tpl.long".to_string(),
        id: Some(header.identification_number.to_string()),
        manufacturer_code: header.manufacturer.as_ref().ok().map(ToString::to_string),
        version: Some(header.version),
        device_type: Some(header.device_type.to_string()),
        device_type_code: Some(header.device_type.into()),
    }
}

fn enrichment_for(code: &str) -> Option<EnrichmentOutput> {
    crate::manufacturers::lookup_manufacturer(code).map(|info| EnrichmentOutput {
        source: "bundled_manufacturer_registry".to_string(),
        manufacturer_code: code.to_string(),
        manufacturer_name: info.name.to_string(),
        manufacturer_website: info.website.to_string(),
        manufacturer_description: info.description.to_string(),
    })
}

fn collect_records(
    records: Option<&user_data::DataRecords<'_>>,
) -> (Vec<RecordOutput>, Option<(usize, String)>) {
    let Some(records) = records else {
        return (Vec::new(), None);
    };
    let mut output = Vec::new();
    let mut offset = 0usize;
    for item in records.clone() {
        match item {
            Ok(record) => {
                let size = record.raw_bytes().len();
                output.push(record_output(output.len(), &record));
                offset += size;
            }
            Err(error) => return (output, Some((offset, error.to_string()))),
        }
    }
    (output, None)
}

fn record_output(index: usize, record: &user_data::DataRecord<'_>) -> RecordOutput {
    let information = record.data_information();
    let value_information = record.value_information();
    let function = information
        .map(|value| value.function_field.to_string())
        .unwrap_or_else(|| "manufacturer_specific".to_string());
    let storage_number = information.map_or(0, |value| value.storage_number);
    let tariff = information.map_or(0, |value| value.tariff);
    let subunit = information.map_or(0, |value| value.device);
    let data_coding = information
        .map(|value| value.data_field_coding.to_string())
        .unwrap_or_else(|| "manufacturer specific".to_string());
    let quantities = value_information
        .map(|value| {
            value
                .labels
                .iter()
                .map(|label| format!("{label:?}"))
                .collect()
        })
        .unwrap_or_default();
    let unit = value_information
        .and_then(|value| (!value.units.is_empty()).then(|| unit_output(value.units.as_slice())));

    RecordOutput {
        index,
        function,
        storage_number,
        tariff,
        subunit,
        quantities,
        value: value_output(record),
        unit,
        data_coding,
        header_hex: record.data_record_header_hex(),
        data_hex: record.data_hex(),
        record_hex: spaced_hex(record.raw_bytes()),
    }
}

fn value_output(record: &user_data::DataRecord<'_>) -> ValueOutput {
    let header_size = record.data_record_header.get_size();
    let raw = record.raw_bytes().get(header_size..).unwrap_or_default();
    let raw_hex = spaced_hex(raw);
    let information = record.data_information();
    let value_information = record.value_information();
    let scale = value_information.map(|value| value.decimal_scale_exponent);
    let offset = record_offset_power10(record);

    match record.value() {
        Some(DataType::Number(number)) | Some(DataType::LossyNumber(number)) => {
            if information
                .is_some_and(|value| value.data_field_coding == DataFieldCoding::Real32Bit)
            {
                let (number_value, special) = finite_number(*number);
                return ValueOutput {
                    kind: "float".to_string(),
                    display: special.clone().unwrap_or_else(|| number.to_string()),
                    decimal: None,
                    significand: None,
                    number: number_value,
                    scale_power10: None,
                    offset_power10: None,
                    precision: Some("f32_approximate".to_string()),
                    special,
                    components: None,
                    raw_hex,
                };
            }

            let significand =
                exact_significand(record, *number).unwrap_or_else(|| format!("{number:.0}"));
            let scale_value = scale.unwrap_or(0);
            let decimal = scaled_decimal(&significand, scale_value, offset);
            let convenience_number = decimal
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite());
            ValueOutput {
                kind: "decimal".to_string(),
                display: decimal.clone(),
                decimal: Some(decimal),
                significand: Some(significand),
                number: convenience_number,
                scale_power10: Some(scale_value),
                offset_power10: offset,
                precision: Some("exact".to_string()),
                special: None,
                components: None,
                raw_hex,
            }
        }
        Some(DataType::Text(text)) => {
            let display = text.to_string();
            ValueOutput {
                kind: "text".to_string(),
                display,
                decimal: None,
                significand: None,
                number: None,
                scale_power10: None,
                offset_power10: None,
                precision: None,
                special: None,
                components: None,
                raw_hex,
            }
        }
        Some(DataType::Date(day, month, year)) => {
            temporal_value("date", day, month, year, None, None, None, raw_hex)
        }
        Some(DataType::Time(second, minute, hour)) => temporal_value(
            "time",
            &SingleEveryOrInvalid::Invalid(),
            &SingleEveryOrInvalid::Invalid(),
            &SingleEveryOrInvalid::Invalid(),
            Some(hour),
            Some(minute),
            Some(second),
            raw_hex,
        ),
        Some(DataType::DateTime(day, month, year, hour, minute)) => temporal_value(
            "datetime",
            day,
            month,
            year,
            Some(hour),
            Some(minute),
            None,
            raw_hex,
        ),
        Some(DataType::DateTimeWithSeconds(day, month, year, hour, minute, second)) => {
            temporal_value(
                "datetime",
                day,
                month,
                year,
                Some(hour),
                Some(minute),
                Some(second),
                raw_hex,
            )
        }
        Some(DataType::ManufacturerSpecific(data)) => ValueOutput {
            kind: "manufacturer_specific".to_string(),
            display: hex_string(data),
            decimal: None,
            significand: None,
            number: None,
            scale_power10: None,
            offset_power10: None,
            precision: None,
            special: None,
            components: None,
            raw_hex,
        },
        None => ValueOutput {
            kind: "none".to_string(),
            display: "No data".to_string(),
            decimal: None,
            significand: None,
            number: None,
            scale_power10: None,
            offset_power10: None,
            precision: None,
            special: None,
            components: None,
            raw_hex,
        },
        Some(_) => ValueOutput {
            kind: "unknown".to_string(),
            display: "Unsupported value type".to_string(),
            decimal: None,
            significand: None,
            number: None,
            scale_power10: None,
            offset_power10: None,
            precision: None,
            special: None,
            components: None,
            raw_hex,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn temporal_value(
    kind: &str,
    day: &SingleEveryOrInvalid<u8>,
    month: &SingleEveryOrInvalid<Month>,
    year: &SingleEveryOrInvalid<u16>,
    hour: Option<&SingleEveryOrInvalid<u8>>,
    minute: Option<&SingleEveryOrInvalid<u8>>,
    second: Option<&SingleEveryOrInvalid<u8>>,
    raw_hex: String,
) -> ValueOutput {
    let mut components = serde_json::Map::new();
    if kind != "time" {
        components.insert("day".to_string(), component_json(day));
        components.insert("month".to_string(), component_json(month));
        components.insert("year".to_string(), component_json(year));
    }
    if let Some(value) = hour {
        components.insert("hour".to_string(), component_json(value));
    }
    if let Some(value) = minute {
        components.insert("minute".to_string(), component_json(value));
    }
    if let Some(value) = second {
        components.insert("second".to_string(), component_json(value));
    }

    let iso = temporal_iso(day, month, year, hour, minute, second, kind);
    let display = iso.clone().unwrap_or_else(|| {
        components
            .iter()
            .map(|(name, value)| format!("{name}={}", value["value_or_state"]))
            .collect::<Vec<_>>()
            .join(", ")
    });
    if let Some(iso) = &iso {
        components.insert("iso".to_string(), serde_json::Value::String(iso.clone()));
    }
    ValueOutput {
        kind: kind.to_string(),
        display,
        decimal: None,
        significand: None,
        number: None,
        scale_power10: None,
        offset_power10: None,
        precision: None,
        special: None,
        components: Some(serde_json::Value::Object(components)),
        raw_hex,
    }
}

fn component_json<T>(component: &SingleEveryOrInvalid<T>) -> serde_json::Value
where
    T: Serialize + fmt::Display,
{
    match component {
        SingleEveryOrInvalid::Single(value) => serde_json::json!({
            "state": "single",
            "value": value,
            "value_or_state": value.to_string(),
        }),
        SingleEveryOrInvalid::Every() => {
            serde_json::json!({"state": "every", "value_or_state": "every"})
        }
        SingleEveryOrInvalid::Invalid() => {
            serde_json::json!({"state": "invalid", "value_or_state": "invalid"})
        }
        _ => serde_json::json!({"state": "unknown", "value_or_state": "unknown"}),
    }
}

fn temporal_iso(
    day: &SingleEveryOrInvalid<u8>,
    month: &SingleEveryOrInvalid<Month>,
    year: &SingleEveryOrInvalid<u16>,
    hour: Option<&SingleEveryOrInvalid<u8>>,
    minute: Option<&SingleEveryOrInvalid<u8>>,
    second: Option<&SingleEveryOrInvalid<u8>>,
    kind: &str,
) -> Option<String> {
    let day = single(day);
    let month = single(month).and_then(month_number);
    let year = single(year);
    let hour = hour.and_then(single);
    let minute = minute.and_then(single);
    let second = second.and_then(single);
    match kind {
        "date" => Some(format!("{:04}-{:02}-{:02}", year?, month?, day?)),
        "time" => Some(format!("{:02}:{:02}:{:02}", hour?, minute?, second?)),
        _ if second.is_some() => Some(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year?, month?, day?, hour?, minute?, second?
        )),
        _ => Some(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00",
            year?, month?, day?, hour?, minute?
        )),
    }
}

fn single<T: Copy>(component: &SingleEveryOrInvalid<T>) -> Option<T> {
    match component {
        SingleEveryOrInvalid::Single(value) => Some(*value),
        _ => None,
    }
}

const fn month_number(month: Month) -> Option<u8> {
    Some(match month {
        Month::January => 1,
        Month::February => 2,
        Month::March => 3,
        Month::April => 4,
        Month::May => 5,
        Month::June => 6,
        Month::July => 7,
        Month::August => 8,
        Month::September => 9,
        Month::October => 10,
        Month::November => 11,
        Month::December => 12,
        #[allow(unreachable_patterns)]
        _ => return None,
    })
}

fn finite_number(value: f64) -> (Option<f64>, Option<String>) {
    if value.is_nan() {
        (None, Some("nan".to_string()))
    } else if value == f64::INFINITY {
        (None, Some("positive_infinity".to_string()))
    } else if value == f64::NEG_INFINITY {
        (None, Some("negative_infinity".to_string()))
    } else {
        (Some(value), None)
    }
}

fn exact_significand(record: &user_data::DataRecord<'_>, parsed: f64) -> Option<String> {
    let information = record.data_information()?;
    let header_size = record.data_record_header.get_size();
    let data = record.raw_bytes().get(header_size..)?;
    match information.data_field_coding {
        DataFieldCoding::Integer8Bit => signed_le_decimal(data.get(..1)?),
        DataFieldCoding::Integer16Bit => signed_le_decimal(data.get(..2)?),
        DataFieldCoding::Integer24Bit => signed_le_decimal(data.get(..3)?),
        DataFieldCoding::Integer32Bit => signed_le_decimal(data.get(..4)?),
        DataFieldCoding::Integer48Bit => signed_le_decimal(data.get(..6)?),
        DataFieldCoding::Integer64Bit => signed_le_decimal(data.get(..8)?),
        DataFieldCoding::BCD2Digit
        | DataFieldCoding::BCD4Digit
        | DataFieldCoding::BCD6Digit
        | DataFieldCoding::BCD8Digit
        | DataFieldCoding::BCDDigit12 => Some(format!("{parsed:.0}")),
        DataFieldCoding::VariableLength => exact_variable_significand(data, parsed),
        _ => None,
    }
}

fn exact_variable_significand(data: &[u8], parsed: f64) -> Option<String> {
    let marker = *data.first()?;
    match marker {
        0xC0..=0xC9 | 0xD0..=0xD9 => Some(format!("{parsed:.0}")),
        0xE0..=0xEF => {
            let length = usize::from(marker - 0xE0);
            signed_le_decimal(data.get(1..=length)?)
        }
        0xF0..=0xF4 => {
            let length = usize::from(marker - 0xEC) * 4;
            signed_le_decimal(data.get(1..=length)?)
        }
        0xF5 => signed_le_decimal(data.get(1..=48)?),
        0xF6 => signed_le_decimal(data.get(1..=64)?),
        _ => None,
    }
}

fn signed_le_decimal(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let negative = bytes.last().is_some_and(|byte| byte & 0x80 != 0);
    let magnitude = if negative {
        let mut value: Vec<u8> = bytes.iter().map(|byte| !byte).collect();
        let mut carry = 1u16;
        for byte in &mut value {
            let sum = u16::from(*byte) + carry;
            *byte = sum as u8;
            carry = sum >> 8;
            if carry == 0 {
                break;
            }
        }
        value
    } else {
        bytes.to_vec()
    };
    let mut digits = vec![0u8];
    for byte in magnitude.iter().rev() {
        let mut carry = u16::from(*byte);
        for digit in &mut digits {
            let value = u16::from(*digit) * 256 + carry;
            *digit = (value % 10) as u8;
            carry = value / 10;
        }
        while carry != 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let mut value: String = digits
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + *digit))
        .collect();
    if negative && value != "0" {
        value.insert(0, '-');
    }
    Some(value)
}

fn record_offset_power10(record: &user_data::DataRecord<'_>) -> Option<isize> {
    let block = record
        .data_record_header
        .raw_data_record_header
        .value_information_block
        .as_ref()?;
    block
        .value_information_extension
        .as_ref()?
        .iter()
        .filter_map(|extension| {
            let code = extension.data & 0x7F;
            (0x78..=0x7B)
                .contains(&code)
                .then(|| isize::from(extension.data & 0b11) - 3)
        })
        .next_back()
}

fn scaled_decimal(significand: &str, scale: isize, offset: Option<isize>) -> String {
    let (negative, digits) = split_sign(significand);
    let minimum_exponent = offset.map_or(scale, |value| value.min(scale));
    let mut left = digits.to_string();
    left.push_str(&"0".repeat(usize::try_from(scale - minimum_exponent).unwrap_or(0)));
    let signed_left = if negative { format!("-{left}") } else { left };
    let integer = if let Some(offset_exponent) = offset {
        let mut right = "1".to_string();
        right.push_str(
            &"0".repeat(usize::try_from(offset_exponent - minimum_exponent).unwrap_or(0)),
        );
        add_signed_decimal_integers(&signed_left, &right)
    } else {
        signed_left
    };
    apply_power10(&integer, minimum_exponent)
}

fn split_sign(value: &str) -> (bool, &str) {
    value
        .strip_prefix('-')
        .map_or((false, value), |digits| (true, digits))
}

fn add_signed_decimal_integers(left: &str, right: &str) -> String {
    let (left_negative, left_digits) = split_sign(left);
    let (right_negative, right_digits) = split_sign(right);
    if left_negative == right_negative {
        let result = add_abs(left_digits, right_digits);
        if left_negative && result != "0" {
            format!("-{result}")
        } else {
            result
        }
    } else {
        match compare_abs(left_digits, right_digits) {
            std::cmp::Ordering::Equal => "0".to_string(),
            std::cmp::Ordering::Greater => {
                let result = subtract_abs(left_digits, right_digits);
                if left_negative {
                    format!("-{result}")
                } else {
                    result
                }
            }
            std::cmp::Ordering::Less => {
                let result = subtract_abs(right_digits, left_digits);
                if right_negative {
                    format!("-{result}")
                } else {
                    result
                }
            }
        }
    }
}

fn add_abs(left: &str, right: &str) -> String {
    let mut output = Vec::new();
    let mut carry = 0u8;
    let mut left = left.bytes().rev();
    let mut right = right.bytes().rev();
    loop {
        let a = left.next().map(|value| value - b'0');
        let b = right.next().map(|value| value - b'0');
        if a.is_none() && b.is_none() && carry == 0 {
            break;
        }
        let sum = a.unwrap_or(0) + b.unwrap_or(0) + carry;
        output.push(b'0' + sum % 10);
        carry = sum / 10;
    }
    output.reverse();
    String::from_utf8(output).unwrap_or_else(|_| "0".to_string())
}

fn subtract_abs(larger: &str, smaller: &str) -> String {
    let mut output = Vec::new();
    let mut borrow = 0i8;
    let mut smaller = smaller.bytes().rev();
    for value in larger.bytes().rev() {
        let mut digit = (value - b'0') as i8 - borrow;
        let sub = smaller.next().map_or(0, |item| (item - b'0') as i8);
        if digit < sub {
            digit += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        output.push(b'0' + u8::try_from(digit - sub).unwrap_or(0));
    }
    while output.len() > 1 && output.last() == Some(&b'0') {
        output.pop();
    }
    output.reverse();
    String::from_utf8(output).unwrap_or_else(|_| "0".to_string())
}

fn compare_abs(left: &str, right: &str) -> std::cmp::Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn apply_power10(integer: &str, exponent: isize) -> String {
    let (negative, digits) = split_sign(integer);
    let mut output = if exponent >= 0 {
        let mut value = digits.to_string();
        value.push_str(&"0".repeat(usize::try_from(exponent).unwrap_or(0)));
        value
    } else {
        let places = exponent.unsigned_abs();
        if digits.len() > places {
            let split = digits.len() - places;
            format!("{}.{}", &digits[..split], &digits[split..])
        } else {
            format!("0.{}{}", "0".repeat(places - digits.len()), digits)
        }
    };
    while output.contains('.') && output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    if negative && output != "0" {
        output.insert(0, '-');
    }
    output
}

fn unit_output(units: &[Unit]) -> UnitOutput {
    let display = units.iter().map(ToString::to_string).collect::<String>();
    let components = units
        .iter()
        .map(|unit| UnitComponent {
            name: format!("{:?}", unit.name),
            exponent: unit.exponent,
        })
        .collect();
    let ucum = units
        .iter()
        .map(ucum_component)
        .collect::<Option<Vec<_>>>()
        .map(|components| components.join("."));
    UnitOutput {
        display,
        components,
        ucum,
    }
}

fn ucum_component(unit: &Unit) -> Option<String> {
    let symbol = match unit.name {
        UnitName::Watt => "W",
        UnitName::Joul => "J",
        UnitName::Kilogram => "kg",
        UnitName::Tonne => "t",
        UnitName::Meter => "m",
        UnitName::Feet => "[ft_i]",
        UnitName::Celsius => "Cel",
        UnitName::Kelvin => "K",
        UnitName::Bar => "bar",
        UnitName::Second => "s",
        UnitName::Minute => "min",
        UnitName::Hour => "h",
        UnitName::Day => "d",
        UnitName::Week => "wk",
        UnitName::Month => "mo",
        UnitName::Year => "a",
        UnitName::Liter => "L",
        UnitName::Volt => "V",
        UnitName::Ampere => "A",
        UnitName::Percent => "%",
        UnitName::Hertz => "Hz",
        UnitName::Fahrenheit => "[degF]",
        UnitName::AmericanGallon => "[gal_us]",
        UnitName::Calorie => "cal",
        _ => return None,
    };
    if unit.exponent == 1 {
        Some(symbol.to_string())
    } else {
        Some(format!("{symbol}{}", unit.exponent))
    }
}

fn render_csv(decoded: &DecodedOutput) -> Result<String, OutputError> {
    use prettytable::csv;

    const RECORD_FIELDS: &[&str] = &[
        "function",
        "storage_number",
        "tariff",
        "subunit",
        "quantities",
        "value_type",
        "value",
        "value_decimal",
        "raw_value",
        "scale_power10",
        "offset_power10",
        "unit",
        "unit_ucum",
        "data_coding",
        "header_hex",
        "data_hex",
        "record_hex",
    ];

    let mut writer = csv::Writer::from_writer(Vec::new());
    let mut headers = [
        "schema_version",
        "protocol",
        "frame_kind",
        "frame_function",
        "address",
        "meter_id",
        "manufacturer_code",
        "identity_source",
        "access_numbers",
        "status_codes",
        "security_mode",
        "decryption_state",
        "decode_state",
        "record_count",
        "diagnostic_codes",
        "manufacturer_name",
        "manufacturer_website",
        "frame_hex",
    ]
    .map(str::to_string)
    .to_vec();
    for record in &decoded.records {
        headers.extend(
            RECORD_FIELDS
                .iter()
                .map(|field| format!("record_{}_{field}", record.index)),
        );
    }
    writer
        .write_record(&headers)
        .map_err(|error| OutputError::Serialization {
            format: "csv",
            message: error.to_string(),
        })?;

    let mut row = csv_frame_row(decoded);
    for record in &decoded.records {
        row.extend(csv_record_values(record));
    }
    writer
        .write_record(row)
        .map_err(|error| OutputError::Serialization {
            format: "csv",
            message: error.to_string(),
        })?;

    let bytes = writer
        .into_inner()
        .map_err(|error| OutputError::Serialization {
            format: "csv",
            message: error.to_string(),
        })?;
    String::from_utf8(bytes).map_err(|error| OutputError::Serialization {
        format: "csv",
        message: error.to_string(),
    })
}

fn csv_frame_row(decoded: &DecodedOutput) -> Vec<String> {
    let identity = decoded.meter.identity.as_ref();
    let enrichment = decoded.enrichment.as_ref();
    let statuses = decoded
        .transport
        .statuses
        .iter()
        .map(|status| status.raw.clone())
        .collect::<Vec<_>>()
        .join("|");
    let access = decoded
        .transport
        .access_numbers
        .iter()
        .map(|value| format!("{}:{}", value.source, value.value))
        .collect::<Vec<_>>()
        .join("|");
    let diagnostics = decoded
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>()
        .join("|");
    vec![
        decoded.schema_version.to_string(),
        decoded.protocol.clone(),
        decoded.frame.kind.clone(),
        decoded.frame.function.clone().unwrap_or_default(),
        decoded.frame.address.clone().unwrap_or_default(),
        identity
            .and_then(|value| value.id.clone())
            .unwrap_or_default(),
        identity
            .and_then(|value| value.manufacturer_code.clone())
            .unwrap_or_default(),
        identity
            .map(|value| value.source.clone())
            .unwrap_or_default(),
        access,
        statuses,
        decoded.security.mode.clone().unwrap_or_default(),
        decoded.security.decryption_state.clone(),
        decoded.decode_state.clone(),
        decoded.records.len().to_string(),
        diagnostics,
        enrichment
            .map(|value| value.manufacturer_name.clone())
            .unwrap_or_default(),
        enrichment
            .map(|value| value.manufacturer_website.clone())
            .unwrap_or_default(),
        decoded.raw.original_frame_hex.clone(),
    ]
}

fn csv_record_values(record: &RecordOutput) -> Vec<String> {
    vec![
        record.function.clone(),
        record.storage_number.to_string(),
        record.tariff.to_string(),
        record.subunit.to_string(),
        record.quantities.join("|"),
        record.value.kind.clone(),
        record.value.display.clone(),
        record.value.decimal.clone().unwrap_or_default(),
        record.value.raw_hex.clone(),
        record
            .value
            .scale_power10
            .map(|value| value.to_string())
            .unwrap_or_default(),
        record
            .value
            .offset_power10
            .map(|value| value.to_string())
            .unwrap_or_default(),
        record
            .unit
            .as_ref()
            .map(|value| value.display.clone())
            .unwrap_or_default(),
        record
            .unit
            .as_ref()
            .and_then(|value| value.ucum.clone())
            .unwrap_or_default(),
        record.data_coding.clone(),
        record.header_hex.clone(),
        record.data_hex.clone(),
        record.record_hex.clone(),
    ]
}

fn render_table(decoded: &DecodedOutput, width: usize) -> Result<String, OutputError> {
    if width < MINIMUM_TABLE_WIDTH {
        return Err(OutputError::InvalidOption {
            option: "table_width",
            message: format!("must be at least {MINIMUM_TABLE_WIDTH} columns"),
        });
    }
    let mut output = String::new();
    let title = format!(
        "{} {} frame · {} decode",
        decoded.protocol, decoded.frame.kind, decoded.decode_state
    );
    output.push_str(&wrap_display(&title, width).join("\n"));
    output.push('\n');

    let identity = decoded.meter.identity.as_ref();
    let mut summary = vec![
        (
            "Function".to_string(),
            decoded
                .frame
                .function
                .clone()
                .unwrap_or_else(|| "—".to_string()),
        ),
        (
            "Meter".to_string(),
            identity
                .and_then(|value| value.id.clone())
                .unwrap_or_else(|| "—".to_string()),
        ),
        (
            "Manufacturer".to_string(),
            identity
                .and_then(|value| value.manufacturer_code.clone())
                .unwrap_or_else(|| "—".to_string()),
        ),
        (
            "Device".to_string(),
            identity
                .and_then(|value| value.device_type.clone())
                .unwrap_or_else(|| "—".to_string()),
        ),
        (
            "Security".to_string(),
            format!(
                "{} · {}",
                decoded.security.payload_protection, decoded.security.decryption_state
            ),
        ),
    ];
    if let Some(enrichment) = &decoded.enrichment {
        summary.push((
            "Manufacturer name".to_string(),
            enrichment.manufacturer_name.clone(),
        ));
        summary.push((
            "Website".to_string(),
            enrichment.manufacturer_website.clone(),
        ));
        summary.push((
            "Description".to_string(),
            enrichment.manufacturer_description.clone(),
        ));
    }
    output.push_str(&key_value_box(&summary, width));

    if !decoded.records.is_empty() {
        if width >= 100 {
            let content = width - 19;
            let widths = vec![
                3,
                (content * 34 / 100).max(18),
                12,
                11,
                13,
                content.saturating_sub(3 + (content * 34 / 100).max(18) + 12 + 11 + 13),
            ];
            let rows = decoded
                .records
                .iter()
                .map(|record| {
                    vec![
                        record.index.to_string(),
                        reading(record),
                        record.function.clone(),
                        format!(
                            "{}/{}/{}",
                            record.storage_number, record.tariff, record.subunit
                        ),
                        record.data_coding.clone(),
                        record.record_hex.clone(),
                    ]
                })
                .collect::<Vec<_>>();
            output.push_str(&box_table(
                &["#", "Reading", "Function", "S/T/U", "Coding", "Raw"],
                &rows,
                &widths,
            ));
        } else if width >= 72 {
            let content = width - 13;
            let reading_width = (content * 52 / 100).max(24);
            let widths = vec![
                3,
                reading_width,
                12,
                content.saturating_sub(3 + reading_width + 12),
            ];
            let rows = decoded
                .records
                .iter()
                .map(|record| {
                    vec![
                        record.index.to_string(),
                        format!(
                            "{}\n{} · raw {}",
                            reading(record),
                            record.data_coding,
                            record.record_hex
                        ),
                        record.function.clone(),
                        format!(
                            "{}/{}/{}",
                            record.storage_number, record.tariff, record.subunit
                        ),
                    ]
                })
                .collect::<Vec<_>>();
            output.push_str(&box_table(
                &["#", "Reading / detail", "Function", "S/T/U"],
                &rows,
                &widths,
            ));
        } else {
            for record in &decoded.records {
                output.push_str(&format!("Record {}\n", record.index));
                output.push_str(&key_value_box(
                    &[
                        ("Reading".to_string(), reading(record)),
                        ("Function".to_string(), record.function.clone()),
                        (
                            "Storage/Tariff/Subunit".to_string(),
                            format!(
                                "{}/{}/{}",
                                record.storage_number, record.tariff, record.subunit
                            ),
                        ),
                        ("Coding".to_string(), record.data_coding.clone()),
                        ("Raw".to_string(), record.record_hex.clone()),
                    ],
                    width,
                ));
            }
        }
    }
    if !decoded.diagnostics.is_empty() {
        output.push_str("Diagnostics\n");
        let rows = decoded
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code.clone(),
                    format!("{}: {}", diagnostic.severity, diagnostic.message),
                )
            })
            .collect::<Vec<_>>();
        output.push_str(&key_value_box(&rows, width));
    }
    Ok(output)
}

fn reading(record: &RecordOutput) -> String {
    let quantity = if record.quantities.is_empty() {
        "Reading".to_string()
    } else {
        record.quantities.join(", ")
    };
    let unit = record
        .unit
        .as_ref()
        .map(|value| format!(" {}", value.display))
        .unwrap_or_default();
    format!("{quantity}: {}{unit}", record.value.display)
}

fn key_value_box(rows: &[(String, String)], width: usize) -> String {
    let content = width.saturating_sub(7);
    let key_width = (content / 3).clamp(10, 24);
    let value_width = content.saturating_sub(key_width).max(1);
    let rows = rows
        .iter()
        .map(|(key, value)| vec![key.clone(), value.clone()])
        .collect::<Vec<_>>();
    box_table(&["Field", "Value"], &rows, &[key_width, value_width])
}

fn box_table(headers: &[&str], rows: &[Vec<String>], widths: &[usize]) -> String {
    let mut output = String::new();
    push_border(&mut output, '┌', '┬', '┐', widths);
    push_row(
        &mut output,
        &headers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        widths,
    );
    push_border(&mut output, '├', '┼', '┤', widths);
    for (index, row) in rows.iter().enumerate() {
        push_row(&mut output, row, widths);
        if index + 1 != rows.len() {
            push_border(&mut output, '├', '┼', '┤', widths);
        }
    }
    push_border(&mut output, '└', '┴', '┘', widths);
    output
}

fn push_border(output: &mut String, left: char, middle: char, right: char, widths: &[usize]) {
    output.push(left);
    for (index, width) in widths.iter().enumerate() {
        output.push_str(&"─".repeat(width + 2));
        output.push(if index + 1 == widths.len() {
            right
        } else {
            middle
        });
    }
    output.push('\n');
}

fn push_row(output: &mut String, cells: &[String], widths: &[usize]) {
    let wrapped = widths
        .iter()
        .enumerate()
        .map(|(index, width)| wrap_display(cells.get(index).map_or("", String::as_str), *width))
        .collect::<Vec<_>>();
    let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    for line in 0..height {
        output.push('│');
        for (index, width) in widths.iter().enumerate() {
            let value = wrapped
                .get(index)
                .and_then(|lines| lines.get(line))
                .map_or("", String::as_str);
            output.push(' ');
            output.push_str(value);
            output.push_str(&" ".repeat(width.saturating_sub(UnicodeWidthStr::width(value))));
            output.push(' ');
            output.push('│');
        }
        output.push('\n');
    }
}

fn wrap_display(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    for source_line in value.split('\n') {
        if source_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_width = 0usize;
        for character in source_line.chars() {
            let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
            if line_width != 0 && line_width + character_width > width {
                lines.push(line);
                line = String::new();
                line_width = 0;
            }
            line.push(character);
            line_width += character_width;
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn render_mermaid(decoded: &DecodedOutput) -> String {
    const RECORD_CLASSES: &[&str] = &[
        "record0", "record1", "record2", "record3", "record4", "record5", "record6", "record7",
    ];

    let mut output = String::from("flowchart TD\n");
    let mut frame_nodes = vec!["FRAME_KIND"];

    output.push_str("    subgraph FRAME_SG[\"Frame\"]\n");
    output.push_str("        direction LR\n");
    output.push_str(&format!(
        "        FRAME_KIND[\"{} · {}\"]\n",
        mermaid_escape(&decoded.protocol),
        mermaid_escape(&decoded.frame.kind)
    ));
    if let Some(function) = &decoded.frame.function {
        output.push_str(&format!(
            "        FRAME_FUNCTION[\"Function · {}\"]\n",
            mermaid_escape(function)
        ));
        frame_nodes.push("FRAME_FUNCTION");
    }
    if let Some(address) = &decoded.frame.address {
        output.push_str(&format!(
            "        FRAME_ADDRESS[\"Address · {}\"]\n",
            mermaid_escape(address)
        ));
        frame_nodes.push("FRAME_ADDRESS");
    }
    for pair in frame_nodes.windows(2) {
        if let [from, to] = pair {
            output.push_str(&format!("        {from} --> {to}\n"));
        }
    }
    output.push_str("    end\n");

    if let Some(identity) = &decoded.meter.identity {
        let mut meter_nodes = vec!["METER_ID", "METER_MANUFACTURER"];
        output.push_str("    subgraph METER_SG[\"Meter\"]\n");
        output.push_str("        direction LR\n");
        output.push_str(&format!(
            "        METER_ID[\"ID · {}\"]\n",
            mermaid_escape(identity.id.as_deref().unwrap_or("unknown"))
        ));
        output.push_str(&format!(
            "        METER_MANUFACTURER[\"Manufacturer · {}\"]\n",
            mermaid_escape(
                identity
                    .manufacturer_code
                    .as_deref()
                    .unwrap_or("unknown manufacturer")
            )
        ));
        if let Some(device_type) = &identity.device_type {
            output.push_str(&format!(
                "        METER_TYPE[\"Type · {}\"]\n",
                mermaid_escape(device_type)
            ));
            meter_nodes.push("METER_TYPE");
        }
        if let Some(version) = identity.version {
            output.push_str(&format!("        METER_VERSION[\"Version · {version}\"]\n"));
            meter_nodes.push("METER_VERSION");
        }
        for pair in meter_nodes.windows(2) {
            if let [from, to] = pair {
                output.push_str(&format!("        {from} --> {to}\n"));
            }
        }
        output.push_str("    end\n");
        output.push_str("    FRAME_SG --> METER_SG\n");
    }

    output.push_str("    subgraph SECURITY_SG[\"Security\"]\n");
    output.push_str("        direction LR\n");
    output.push_str(&format!(
        "        SECURITY_MODE[\"{}\"]\n        SECURITY_STATE[\"{}\"]\n",
        mermaid_escape(&decoded.security.payload_protection),
        mermaid_escape(&decoded.security.decryption_state)
    ));
    output.push_str("        SECURITY_MODE --> SECURITY_STATE\n");
    output.push_str("    end\n");
    output.push_str("    FRAME_SG --> SECURITY_SG\n");

    if !decoded.records.is_empty() {
        output.push_str(&format!(
            "    subgraph RECORDS_SG[\"Data Records · {}\"]\n",
            decoded.records.len()
        ));
        output.push_str("        direction LR\n");
        for record in &decoded.records {
            output.push_str(&format!(
                "        R{}[\"{}\"]\n",
                record.index,
                mermaid_escape(&reading(record))
            ));
        }
        for pair in decoded.records.windows(2) {
            if let [from, to] = pair {
                output.push_str(&format!("        R{} ~~~ R{}\n", from.index, to.index));
            }
        }
        output.push_str("    end\n");
        if decoded.meter.identity.is_some() {
            output.push_str("    METER_SG --> RECORDS_SG\n");
        } else {
            output.push_str("    FRAME_SG --> RECORDS_SG\n");
        }
    }

    if let Some(diagnostic) = decoded.diagnostics.first() {
        output.push_str(&format!(
            "    DIAG[\"{} diagnostic(s) · {}\"]\n    FRAME_SG --> DIAG\n",
            decoded.diagnostics.len(),
            mermaid_escape(&diagnostic.code)
        ));
    }

    output.push('\n');
    output.push_str("    classDef frame fill:#1565c0,color:#fff,stroke:#0d47a1,stroke-width:2px\n");
    output.push_str("    classDef meter fill:#2e7d32,color:#fff,stroke:#1b5e20,stroke-width:2px\n");
    output.push_str(
        "    classDef security fill:#e65100,color:#fff,stroke:#bf360c,stroke-width:2px\n",
    );
    output
        .push_str("    classDef record0 fill:#1565c0,color:#fff,stroke:#0d47a1,stroke-width:2px\n");
    output
        .push_str("    classDef record1 fill:#2e7d32,color:#fff,stroke:#1b5e20,stroke-width:2px\n");
    output
        .push_str("    classDef record2 fill:#e65100,color:#fff,stroke:#bf360c,stroke-width:2px\n");
    output
        .push_str("    classDef record3 fill:#6a1b9a,color:#fff,stroke:#4a148c,stroke-width:2px\n");
    output
        .push_str("    classDef record4 fill:#c62828,color:#fff,stroke:#8e0000,stroke-width:2px\n");
    output
        .push_str("    classDef record5 fill:#00695c,color:#fff,stroke:#004d40,stroke-width:2px\n");
    output
        .push_str("    classDef record6 fill:#f9a825,color:#000,stroke:#f57f17,stroke-width:2px\n");
    output
        .push_str("    classDef record7 fill:#4527a0,color:#fff,stroke:#311b92,stroke-width:2px\n");
    output.push_str(
        "    classDef diagnostic fill:#b71c1c,color:#fff,stroke:#7f0000,stroke-width:2px\n",
    );
    output.push_str(&format!("    class {} frame\n", frame_nodes.join(",")));
    if let Some(identity) = &decoded.meter.identity {
        output.push_str("    class METER_ID,METER_MANUFACTURER meter\n");
        if identity.device_type.is_some() {
            output.push_str("    class METER_TYPE meter\n");
        }
        if identity.version.is_some() {
            output.push_str("    class METER_VERSION meter\n");
        }
    }
    output.push_str("    class SECURITY_MODE,SECURITY_STATE security\n");
    for record in &decoded.records {
        let record_class = RECORD_CLASSES
            .get(record.index % RECORD_CLASSES.len())
            .copied()
            .unwrap_or("record0");
        output.push_str(&format!("    class R{} {}\n", record.index, record_class));
    }
    if !decoded.diagnostics.is_empty() {
        output.push_str("    class DIAG diagnostic\n");
    }
    output.push_str("    style FRAME_SG fill:#e8f1fb,stroke:#1565c0,color:#0d1117\n");
    if decoded.meter.identity.is_some() {
        output.push_str("    style METER_SG fill:#e8f5e9,stroke:#2e7d32,color:#0d1117\n");
    }
    output.push_str("    style SECURITY_SG fill:#fff3e0,stroke:#e65100,color:#0d1117\n");
    if !decoded.records.is_empty() {
        output.push_str("    style RECORDS_SG fill:#f3e5f5,stroke:#6a1b9a,color:#0d1117\n");
    }

    output
}

fn mermaid_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', " ")
}

fn hex_string(data: &[u8]) -> String {
    data.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn spaced_hex(data: &[u8]) -> String {
    data.iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRED_FRAME: &str = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";

    #[test]
    fn strict_hex_accepts_compact_and_byte_tokens() {
        assert_eq!(decode_hex_bytes("683D").unwrap(), [0x68, 0x3D]);
        assert_eq!(
            decode_hex_bytes("0x68: 3d-0X16").unwrap(),
            [0x68, 0x3D, 0x16]
        );
    }

    #[test]
    fn strict_hex_rejects_odd_and_invalid_input() {
        assert!(decode_hex_bytes("123").is_err());
        assert!(decode_hex_bytes("68,3D").is_err());
        assert!(decode_hex_bytes("0x6").is_err());
    }

    #[test]
    fn canonical_json_has_versioned_layers() {
        let rendered =
            render_hex(WIRED_FRAME, OutputFormat::Json, &RenderOptions::default()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["protocol"], "wired");
        assert!(value["records"].is_array());
        assert!(value.get("summary").is_none());
    }

    #[test]
    fn table_respects_requested_width() {
        for width in [32, 71, 72, 99, 100, 120] {
            let rendered = render_hex(
                WIRED_FRAME,
                OutputFormat::Table,
                &RenderOptions {
                    table_width: Some(width),
                    ..RenderOptions::default()
                },
            )
            .unwrap();
            assert!(
                rendered
                    .lines()
                    .all(|line| UnicodeWidthStr::width(line) <= width),
                "line exceeded width {width}:\n{rendered}"
            );
        }
    }

    #[test]
    fn arbitrary_signed_integer_conversion_is_exact() {
        assert_eq!(signed_le_decimal(&[0xFF]), Some("-1".to_string()));
        assert_eq!(
            signed_le_decimal(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
            Some(i64::MAX.to_string())
        );
    }

    #[test]
    fn scaled_decimal_applies_additive_offset() {
        assert_eq!(scaled_decimal("876543", -3, None), "876.543");
        assert_eq!(scaled_decimal("10", 0, Some(0)), "11");
        assert_eq!(scaled_decimal("-10", -1, Some(-1)), "-0.9");
    }
}
