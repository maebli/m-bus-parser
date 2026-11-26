//! Typed, harmonized output contract for `std` consumers.
//!
//! The low-level parsers remain allocation-free and `no_std`; this module owns
//! the presentation model used by the CLI, Python, WebAssembly, and text
//! renderers.

use core::str::FromStr;
use std::fmt;

use m_bus_core::{ConfigurationField, Function, SecurityMode};
use m_bus_oms_rules::telegram::{self, Link, OmsReport, RecordAvailability, RuleFinding};
pub use m_bus_oms_rules::OmsSecurityModeCheck;
use serde::Serialize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use wired_mbus_link_layer as wired;
use wireless_mbus_link_layer as wireless;

use crate::mbus_data::MbusData;
use crate::user_data;
use crate::user_data::data_information::{
    DataFieldCoding, DataType, FunctionField, Month, SingleEveryOrInvalid, SpecialFunctions,
};
use crate::user_data::value_information::{Unit, UnitName, ValueLabel};

mod application;
mod fragments;
mod messages;
mod profile;
#[cfg(feature = "oms-gen3")]
mod sml;
pub use fragments::{decode_fragment_frames, decode_fragments, FragmentOutput};
pub(crate) mod profiles;
mod table_style;
use table_style::{Cell, TableStyle};
#[cfg(feature = "decryption-mode-13")]
mod tls;
#[cfg(feature = "decryption-mode-13")]
mod tls_session;
#[cfg(feature = "decryption-mode-13")]
use m_bus_core::decryption::mode13::TlsSession;
#[cfg(feature = "decryption-mode-13")]
pub use tls::{decode_hex_with_tls_session, render_hex_with_tls_session};
#[cfg(feature = "decryption-mode-13")]
pub use tls_session::ImportedTlsSession;
#[cfg(not(feature = "decryption-mode-13"))]
struct TlsSession<'a>(core::marker::PhantomData<&'a ()>);
pub use application::render_application_hex;
#[cfg(feature = "decryption-mode-13")]
pub use application::render_application_hex_with_tls_session;
use messages::application_output;

const SCHEMA_VERSION: u8 = 3;
const DEFAULT_TABLE_WIDTH: usize = 100;
const MINIMUM_TABLE_WIDTH: usize = 32;

/// Canonical output formats supported by the typed rendering API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputFormat {
    Table,
    TableAnsi,
    TableHtml,
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
            Self::TableAnsi => "table-ansi",
            Self::TableHtml => "table-html",
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
            "table-ansi" => Ok(Self::TableAnsi),
            "table-html" => Ok(Self::TableHtml),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<ApplicationOutput>,
    pub security: SecurityOutput,
    pub oms_5_0_1_checks: [RuleFinding; 6],
    pub record_availability: RecordAvailability,
    pub records: Vec<RecordOutput>,
    pub raw: RawOutput,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<EnrichmentOutput>,
}

impl DecodedOutput {
    pub fn assess_oms_5_0_1(&self) -> OmsReport {
        assess_oms_5_0_1(
            &self.frame,
            &self.transport,
            self.record_availability,
            &self.records,
        )
    }
}

fn assess_oms_5_0_1(
    frame: &FrameOutput,
    transport: &TransportOutput,
    availability: RecordAvailability,
    records: &[RecordOutput],
) -> OmsReport {
    let control = frame
        .control_field
        .as_deref()
        .and_then(|raw| u8::from_str_radix(raw, 16).ok());
    let link = match (frame.kind.as_str(), control) {
        ("wireless", Some(control)) => Link::Wireless(control),
        ("short" | "control" | "long", Some(control)) => {
            Function::try_from(control).map_or(Link::Unavailable, Link::Wired)
        }
        _ => Link::Unavailable,
    };
    let configuration = transport
        .configuration
        .as_ref()
        .and_then(|field| u16::from_str_radix(&field.raw, 16).ok())
        .map(ConfigurationField::from);
    let headers = records
        .iter()
        .map(|record| decode_hex_bytes(&record.header_hex).ok());
    telegram::assess_5_0_1(link, configuration, availability, headers)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_address_mask: Option<String>,
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
pub struct ApplicationOutput {
    pub control_information: String,
    pub protocol: String,
    pub payload_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TlsRecordOutput {
    pub content_type: String,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TlsOutput {
    pub protocol: String,
    pub records: Vec<TlsRecordOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityOutput {
    pub oms_5_0_1_security_mode_check: OmsSecurityModeCheck,
    pub payload_protection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_code: Option<u8>,
    pub decryption_state: String,
    pub key_supplied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_counter: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsOutput>,
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
    /// Standardized unit notation when available, otherwise a readable symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub data_coding: String,
    pub header_hex: String,
    pub data_hex: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub manufacturer_fields: Vec<ManufacturerField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManufacturerField {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    pub range: core::ops::Range<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueOutput {
    pub kind: String,
    /// The single canonical parsed representation for this value.
    ///
    /// Exact decimals and text use JSON strings, finite floating-point values
    /// use JSON numbers, complete temporal values use ISO 8601 strings, and
    /// partial temporal values use a compact component object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fragment_frames_hex: Vec<String>,
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
    tls: Option<TlsOutput>,
    application_mac_verified: bool,
}

impl SecurityContext {
    fn verify_application(
        mut self,
        block: Option<&user_data::UserDataBlock<'_>>,
        key: Option<&[u8; 16]>,
    ) -> Result<Self, OutputError> {
        if let Some(key) = key {
            self.application_mac_verified = messages::verify_application(block, &self, key)?;
        }
        Ok(self)
    }

    fn authenticated(&self) -> bool {
        self.decrypted
            && matches!(
                self.mode,
                Some(
                    SecurityMode::AesCbc128IvZero
                        | SecurityMode::AesCcm128
                        | SecurityMode::SpecificUsage13
                )
            )
    }
}

/// Decode a human-readable hexadecimal frame.
pub fn decode_hex(input: &str, options: &DecodeOptions) -> Result<DecodedOutput, OutputError> {
    decode_capture_hex(input, options)?.single()
}

pub fn decode_capture_hex(
    input: &str,
    options: &DecodeOptions,
) -> Result<FragmentOutput, OutputError> {
    if let Some(packets) = fragments::text_packets(input)? {
        let frames = packets.iter().map(Vec::as_slice).collect::<Vec<_>>();
        return decode_fragment_frames(&frames, options);
    }
    let bytes = decode_hex_bytes(input)?;
    let mut decoded = decode_capture_bytes(&bytes, options)?;
    if let FragmentOutput::Single(frame) = &mut decoded {
        frame.raw.input = Some(input.to_string());
    }
    Ok(decoded)
}

pub fn decode_capture_bytes(
    data: &[u8],
    options: &DecodeOptions,
) -> Result<FragmentOutput, OutputError> {
    match fragments::detect_bytes(data, options)? {
        Some(decoded) => Ok(decoded),
        None => decode_bytes_with_keys(data, options, None, None)
            .map(|frame| FragmentOutput::Single(Box::new(frame))),
    }
}

pub fn decode_bytes(data: &[u8], options: &DecodeOptions) -> Result<DecodedOutput, OutputError> {
    decode_capture_bytes(data, options)?.single()
}

#[cfg(feature = "decryption-mode-7")]
pub fn decode_bytes_with_application_key(
    data: &[u8],
    options: &DecodeOptions,
    application_key: &[u8; 16],
) -> Result<DecodedOutput, OutputError> {
    decode_bytes_with_keys(data, options, None, Some(application_key))
}

fn decode_bytes_with_keys(
    data: &[u8],
    options: &DecodeOptions,
    session: Option<&TlsSession<'_>>,
    application_key: Option<&[u8; 16]>,
) -> Result<DecodedOutput, OutputError> {
    if data.is_empty() {
        return Err(OutputError::EmptyInput);
    }

    if let Some(frame) = fragments::singleton_payload(data)? {
        let mut decoded = decode_bytes_with_keys(frame, options, session, application_key)?;
        decoded.raw.fragment_frames_hex.push(hex_string(data));
        return Ok(decoded);
    }

    let wired_error = match MbusData::<wired::WiredFrame>::try_from(data) {
        Ok(mut parsed) => {
            let mut decrypted_buffer = [0u8; 512];
            let security = prepare_security(
                parsed.user_data.as_ref(),
                None,
                options.key.as_ref(),
                session,
                &mut decrypted_buffer,
            )?
            .verify_application(parsed.user_data.as_ref(), application_key)?;
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
            let radio4 = radio4_application(&parsed.frame, normalized.get(9).copied())?;
            if radio4.is_some() {
                parsed.user_data = None;
                parsed.data_records = None;
                parsed.application_error = None;
            }
            let mut decrypted_buffer = [0u8; 512];
            let security = prepare_security(
                parsed.user_data.as_ref(),
                Some(&parsed.frame.manufacturer_id),
                options.key.as_ref(),
                session,
                &mut decrypted_buffer,
            )?
            .verify_application(parsed.user_data.as_ref(), application_key)?;
            if let Some(decrypted) = security.decrypted_payload.as_deref() {
                parsed.data_records = decrypted_records(parsed.user_data.as_ref(), decrypted);
            } else if security.encrypted {
                parsed.data_records = None;
            }
            let mut result = build_wireless_output(
                data,
                normalized,
                stripped,
                &parsed,
                security.clone(),
                options.include_enrichment,
            );
            if let Some(application) = radio4 {
                result.application = Some(application);
            }
            Ok(result)
        }
        Err(error) => Err(OutputError::InvalidFrame {
            wired: wired_error,
            wireless: format!("{error:?}"),
        }),
    }
}

/// Decode DIF/VIF records after link and transport headers have been removed.
pub fn decode_data_records(data: &[u8]) -> Result<Vec<RecordOutput>, OutputError> {
    if data.is_empty() {
        return Err(OutputError::EmptyInput);
    }
    let records = user_data::DataRecords::from(data);
    let (output, error) = collect_records(Some(&records), None);
    if let Some((offset, message)) = error {
        return Err(OutputError::Rendering {
            code: "application.records_invalid",
            message: format!("failed to parse data record at byte offset {offset}: {message}"),
        });
    }
    Ok(output)
}

/// Decode human-readable hexadecimal DIF/VIF records.
pub fn decode_data_records_hex(input: &str) -> Result<Vec<RecordOutput>, OutputError> {
    let data = decode_hex_bytes(input)?;
    decode_data_records(&data)
}

/// Render a human-readable hexadecimal frame.
pub fn render_hex(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    if let Some(packets) = fragments::text_packets(input)? {
        let frames = packets.iter().map(Vec::as_slice).collect::<Vec<_>>();
        return render_capture(
            decode_fragment_frames(&frames, &options.decode)?,
            format,
            options,
        );
    }
    let bytes = decode_hex_bytes(input)?;
    render_bytes(&bytes, format, options)
}

/// Render an already materialized frame.
pub fn render_bytes(
    data: &[u8],
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    if let Some(capture) = fragments::detect_bytes(data, &options.decode)? {
        return render_capture(capture, format, options);
    }
    let frame = fragments::singleton_payload(data)?.unwrap_or(data);
    match format {
        OutputFormat::Xml => {
            crate::rscada_xml::render_from_bytes(frame, options.decode.key.as_ref()).map_err(
                |message| OutputError::Rendering {
                    code: "xml.unsupported",
                    message,
                },
            )
        }
        OutputFormat::Annotated => {
            crate::mbus_data::render_annotated_bytes(frame, options.decode.key.as_ref())
        }
        OutputFormat::AnnotatedText => {
            crate::annotate::annotate_and_render(frame).map_err(|error| OutputError::Rendering {
                code: "annotation.failed",
                message: error.to_string(),
            })
        }
        _ => {
            let decoded = decode_bytes_with_keys(data, &options.decode, None, None)?;
            render_decoded(&decoded, format, options)
        }
    }
}

fn render_capture(
    capture: FragmentOutput,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    let frames = match &capture {
        FragmentOutput::Single(frame) => std::slice::from_ref(frame.as_ref()),
        FragmentOutput::Multiple(frames) => frames.as_slice(),
    };
    match format {
        OutputFormat::Json => {
            return serde_json::to_string_pretty(&capture).map_err(|e| OutputError::Serialization {
                format: "json",
                message: e.to_string(),
            })
        }
        OutputFormat::Yaml => {
            return serde_norway::to_string(&capture).map_err(|e| OutputError::Serialization {
                format: "yaml",
                message: e.to_string(),
            })
        }
        _ => {}
    }
    let mut parts = Vec::new();
    for frame in frames {
        let part = match format {
            OutputFormat::Xml | OutputFormat::Annotated | OutputFormat::AnnotatedText => {
                if frame.frame.kind == "reassembled" {
                    return Err(OutputError::Rendering {
                        code: "render.unsupported",
                        message: "Select JSON or table for a reassembled AFL message".into(),
                    });
                }
                render_bytes(
                    &decode_hex_bytes(&frame.raw.original_frame_hex)?,
                    format,
                    options,
                )?
            }
            _ => render_decoded(frame, format, options)?,
        };
        parts.push(part);
    }
    if parts.len() == 1 {
        return Ok(parts.pop().unwrap_or_default());
    }
    match format {
        OutputFormat::Csv => {
            for part in parts.iter_mut().skip(1) {
                if let Some(end) = part.find('\n') {
                    part.drain(..=end);
                }
            }
            Ok(parts.concat())
        }
        OutputFormat::Xml => {
            let documents = parts
                .iter()
                .map(|part| {
                    part.split_once("?>")
                        .map_or(part.as_str(), |(_, body)| body.trim_start())
                })
                .collect::<Vec<_>>();
            Ok(format!(
                "<MBusFrames>\n{}\n</MBusFrames>",
                documents.join("\n")
            ))
        }
        OutputFormat::Annotated => Ok(format!("[{}]", parts.join(",\n"))),
        OutputFormat::Mermaid => Err(OutputError::Rendering {
            code: "render.unsupported",
            message: "Select one frame to render a Mermaid diagram".into(),
        }),
        _ => Ok(parts.join("\n")),
    }
}

fn render_decoded(
    decoded: &DecodedOutput,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(decoded).map_err(|error| OutputError::Serialization {
                format: "json",
                message: error.to_string(),
            })
        }
        OutputFormat::Yaml => {
            serde_norway::to_string(decoded).map_err(|error| OutputError::Serialization {
                format: "yaml",
                message: error.to_string(),
            })
        }
        OutputFormat::Csv => render_csv(decoded),
        OutputFormat::Table | OutputFormat::TableAnsi | OutputFormat::TableHtml => {
            let style = match format {
                OutputFormat::TableAnsi => TableStyle::Terminal,
                OutputFormat::TableHtml => TableStyle::Html,
                _ => TableStyle::Plain,
            };
            render_table(
                decoded,
                options.table_width.unwrap_or(DEFAULT_TABLE_WIDTH),
                style,
            )
        }
        OutputFormat::Mermaid => Ok(render_mermaid(decoded)),
        _ => unreachable!("specialized formats handled by caller"),
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
        if character.is_ascii_whitespace() || matches!(character, ':' | '-' | ',') {
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
    session: Option<&TlsSession<'_>>,
    output: &mut [u8],
) -> Result<SecurityContext, OutputError> {
    let (encrypted, mode, ell_encrypted) = security_fields(user_data);
    #[cfg(not(feature = "decryption-mode-13"))]
    let _ = session;
    #[cfg(feature = "decryption-mode-13")]
    if mode == Some(SecurityMode::SpecificUsage13) {
        return tls::prepare_mode13(user_data, session);
    }
    #[cfg(feature = "decryption-mode-13")]
    if session.is_some() {
        return Err(OutputError::Decryption {
            code: "security.tls_transport_required",
            message: "TLS session decoding requires a mode 13 transport header; remove AFL framing after reassembly".to_string(),
        });
    }
    if !encrypted {
        return Ok(SecurityContext {
            encrypted: false,
            mode,
            ell_encrypted,
            decrypted: false,
            key_supplied: key.is_some(),
            decrypted_payload: None,
            tls: None,
            application_mac_verified: false,
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
            tls: None,
            application_mac_verified: false,
        });
    };
    if ell_encrypted && mode.is_none() {
        return Err(OutputError::Decryption {
            code: "security.unsupported_mode",
            message: "decryption was requested for an ELL-protected payload, which is not currently supported"
                .to_string(),
        });
    }

    #[cfg(feature = "decryption-api")]
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
                crate::decryption::DecryptionError::UnsupportedConfiguration => {
                    "security.unsupported_configuration"
                }
                crate::decryption::DecryptionError::AuthenticationFailed => {
                    "security.authentication_failed"
                }
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
        if cfg!(feature = "oms-gen2")
            && mode == Some(SecurityMode::AesCbc128IvNonZero)
            && !decrypted.starts_with(&[0x2f, 0x2f])
        {
            return Err(OutputError::Decryption {
                code: "security.decryption_failed",
                message: "Mode 5 decryption verification bytes do not match".into(),
            });
        }
        Ok(SecurityContext {
            encrypted,
            mode,
            ell_encrypted,
            decrypted: true,
            key_supplied: true,
            decrypted_payload: Some(decrypted),
            tls: None,
            application_mac_verified: false,
        })
    }

    #[cfg(not(feature = "decryption-api"))]
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
        Some(UserDataBlock::ApplicationMessage(message)) => {
            let mode = message
                .transport_header
                .short_header()
                .map(|header| header.configuration_field.security_mode());
            (message.transport_header.is_encrypted(), mode, false)
        }
        Some(UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header, ..
        }) => {
            let mode = long_tpl_header
                .short_tpl_header
                .configuration_field
                .security_mode();
            (long_tpl_header.is_encrypted(), Some(mode), false)
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
            if ell_encrypted && !short_tpl_header.is_encrypted() {
                (true, None, true)
            } else {
                (short_tpl_header.is_encrypted(), Some(mode), ell_encrypted)
            }
        }
        _ => (false, None, false),
    }
}

#[cfg(feature = "decryption-api")]
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
        }
        | UserDataBlock::ApplicationMessage(user_data::ApplicationMessage {
            transport_header: user_data::TransportHeader::Long(long_tpl_header),
            ..
        }) => {
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
        }
        | UserDataBlock::ApplicationMessage(user_data::ApplicationMessage {
            transport_header: user_data::TransportHeader::Short(short_tpl_header),
            ..
        }) => {
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
            Some(*function),
            Some(address.to_string()),
            Some(*data),
        ),
        wired::WiredFrame::ControlFrame {
            function,
            address,
            data,
        } => (
            "control",
            Some(*function),
            Some(address.to_string()),
            Some(*data),
        ),
        wired::WiredFrame::ShortFrame { function, address } => {
            ("short", Some(*function), Some(address.to_string()), None)
        }
        wired::WiredFrame::SingleCharacter { .. } => ("single_character", None, None, None),
        _ => ("unknown", None, None, None),
    };

    build_output(
        "wired",
        FrameOutput {
            kind: kind.to_string(),
            function: function.as_ref().map(frame_function_name),
            address,
            control_field: match parsed.frame {
                wired::WiredFrame::LongFrame { .. } | wired::WiredFrame::ControlFrame { .. } => {
                    original.get(4)
                }
                wired::WiredFrame::ShortFrame { .. } => original.get(1),
                _ => None,
            }
            .map(|byte| format!("{byte:02X}")),
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

fn radio4_application(
    frame: &wireless::WirelessFrame<'_>,
    raw_medium: Option<u8>,
) -> Result<Option<ApplicationOutput>, OutputError> {
    let Some((&0xa2, payload)) = frame.data.split_first() else {
        return Ok(None);
    };
    let id = &frame.manufacturer_id;
    if id.manufacturer_code.code != ['T', 'C', 'H']
        || !matches!(id.version, 0x70 | 0x95)
        || !matches!(raw_medium, Some(0x62 | 0x72))
    {
        return Ok(None);
    }
    let [_, _, _, previous_low, previous_high, _, _, current_low, current_high, ..] = payload
    else {
        return Err(OutputError::Rendering {
            code: "application.radio4_truncated",
            message: "Techem radio 4 payload is missing its volume counters".into(),
        });
    };
    let previous = u32::from(u16::from_le_bytes([*previous_low, *previous_high]));
    let current = u32::from(u16::from_le_bytes([*current_low, *current_high]));
    let cubic_metres = |tenths: u32| scaled_decimal(&tenths.to_string(), -1, None);
    Ok(Some(ApplicationOutput {
        control_information: "A2".into(),
        protocol: "Techem radio 4".into(),
        payload_hex: hex_string(payload),
        decoded: Some(serde_json::json!({
            "month_start_volume_m3": cubic_metres(previous),
            "volume_since_month_start_m3": cubic_metres(current),
            "total_volume_m3": cubic_metres(previous + current),
        })),
    }))
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
            function: parsed
                .frame
                .function
                .map(|function| frame_function_name(&function)),
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
    let fragment = application_error.is_some() && payload.is_some_and(fragments::is_fragment);
    let mut diagnostics = Vec::new();
    if let Some(error) = application_error {
        let (code, message) = if fragment {
            (
                "fragment.incomplete",
                "Supply all fragments in order to decode this message".to_owned(),
            )
        } else {
            ("application.partial", error.to_string())
        };
        diagnostics.push(Diagnostic {
            severity: "warning".to_string(),
            code: code.into(),
            layer: "application".to_string(),
            message,
            offset_start: None,
            offset_end: None,
        });
    }
    if security_context.encrypted && !security_context.decrypted {
        let (code, message) = if security_context.tls.is_some() {
            ("security.tls_session_required", "TLS records are available; application decryption requires session keys and a record counter. Handshake and channel records require an external TLS endpoint.")
        } else {
            (
                "security.key_missing",
                "payload is encrypted; header metadata is available but records require a key",
            )
        };
        diagnostics.push(Diagnostic {
            severity: "warning".to_string(),
            code: code.to_string(),
            layer: "security".to_string(),
            message: message.to_string(),
            offset_start: None,
            offset_end: None,
        });
    } else if security_context.authenticated() {
        diagnostics.push(Diagnostic {
            severity: "info".to_string(),
            code: "security.authenticated".to_string(),
            layer: "security".to_string(),
            message: if security_context.tls.is_some() {
                "TLS payload was decrypted and its authentication tags were verified"
            } else if security_context.mode == Some(SecurityMode::AesCbc128IvZero) {
                "AES-CBC payload was decrypted and its AES-CMAC was verified"
            } else {
                "AES-CCM payload was decrypted and its authentication tag was verified"
            }
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
    } else if security_context.key_supplied && !fragment {
        diagnostics.push(Diagnostic {
            severity: "info".to_string(),
            code: "security.key_unused".to_string(),
            layer: "security".to_string(),
            message: "a key was supplied for a cleartext frame and was not used".to_string(),
            offset_start: None,
            offset_end: None,
        });
    }

    let (meter, transport) = meter_and_transport(user_data, wireless_id);
    let (record_outputs, record_error) = match user_data {
        Some(user_data::UserDataBlock::FixedDataStructure {
            device_type_and_unit,
            status,
            counter1,
            counter2,
            ..
        }) => (
            fixed_record_outputs(*device_type_and_unit, *status, counter1, counter2, payload),
            None,
        ),
        _ => collect_records(records, meter.identity.as_ref()),
    };
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

    let (application, application_diagnostic) = application_output(user_data, &security_context);
    let application_partial = application_diagnostic.is_some();
    if let Some(diagnostic) = application_diagnostic {
        diagnostics.push(diagnostic);
    }
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
    let decryption_state = if fragment {
        "awaiting_fragments"
    } else if !security_context.encrypted {
        "not_applicable"
    } else if security_context.authenticated() {
        "decrypted_authenticated"
    } else if security_context.decrypted {
        "decrypted_unverified"
    } else if security_context.tls.is_some() {
        "session_required"
    } else {
        "key_missing"
    };
    let payload_protection = if security_context.encrypted {
        "encrypted"
    } else if security_context.ell_encrypted || fragment {
        "unknown"
    } else {
        "cleartext"
    };
    let mode = security_context.mode;
    let partial = application_error.is_some()
        || application_partial
        || record_error.is_some()
        || (security_context.encrypted && !security_context.decrypted);

    let record_availability = if fragment {
        RecordAvailability::Fragmented
    } else if security_context.encrypted && !security_context.decrypted {
        RecordAvailability::Encrypted
    } else if records.is_none() {
        RecordAvailability::Unavailable
    } else if record_error.is_some() {
        RecordAvailability::Partial
    } else {
        RecordAvailability::Complete
    };
    let report = assess_oms_5_0_1(&frame, &transport, record_availability, &record_outputs);
    DecodedOutput {
        record_availability,
        oms_5_0_1_checks: report.findings,
        schema_version: SCHEMA_VERSION,
        decode_state: if partial { "partial" } else { "complete" }.to_string(),
        protocol: protocol.to_string(),
        frame,
        meter,
        transport,
        application,
        security: SecurityOutput {
            oms_5_0_1_security_mode_check: report.security_mode,
            payload_protection: payload_protection.to_string(),
            mode: mode.map(|value| value.to_string()),
            mode_code: mode.map(|value| value.to_bits()),
            decryption_state: decryption_state.to_string(),
            key_supplied: security_context.key_supplied,
            key_id: user_data
                .filter(|block| block.authentication().is_some())
                .and_then(|block| block.transport_payload())
                .and_then(|(_, data)| data.first())
                .map(|extension| extension & 0x0f),
            message_counter: user_data
                .and_then(|block| block.authentication())
                .map(|header| header.message_counter),
            tls: security_context.tls,
            encrypted_payload_hex,
            decrypted_payload_hex,
        },
        records: record_outputs,
        raw: RawOutput {
            fragment_frames_hex: Vec::new(),
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
        secondary_address_mask: None,
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
        Some(UserDataBlock::ExtendedLinkLayerOnly(ell)) => {
            transport.header_kind = Some("ell".to_string());
            transport.access_numbers.push(SourcedU8 {
                source: "ell".to_string(),
                value: ell.access_number,
            });
            transport.ell = Some(ell_output(ell));
        }
        Some(UserDataBlock::FixedDataStructure {
            identification_number,
            access_number,
            status,
            device_type_and_unit,
            ..
        }) => {
            let [a, b] = device_type_and_unit.to_be_bytes();
            primary = Some(MeterIdentity {
                source: "application.fixed".to_string(),
                id: Some(identification_number.to_string()),
                manufacturer_code: None,
                version: None,
                device_type: Some(crate::fixed::medium(a, b).into()),
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
        Some(UserDataBlock::SelectSlave { secondary_address }) => {
            transport.header_kind = Some("slave_selection".to_string());
            transport.secondary_address_mask = Some(secondary_address.to_string());
        }
        Some(UserDataBlock::ApplicationMessage(message)) => {
            transport.header_kind = Some(
                match &message.transport_header {
                    user_data::TransportHeader::None => "none",
                    user_data::TransportHeader::Short(_) => "short_tpl",
                    user_data::TransportHeader::Long(_) => "long_tpl",
                }
                .to_string(),
            );
            if let Some(header) = message.transport_header.long_header() {
                let identity = identity_from_long(header);
                if let Some(link) = link_identity {
                    if link != identity {
                        alternates.push(link);
                    }
                }
                primary = Some(identity);
            }
            if let Some(header) = message.transport_header.short_header() {
                add_short_transport(&mut transport, "tpl", header);
            }
            if let Some(ell) = &message.extended_link_layer {
                transport.access_numbers.push(SourcedU8 {
                    source: "ell".to_string(),
                    value: ell.access_number,
                });
                transport.ell = Some(ell_output(ell));
            }
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

fn fixed_record_outputs(
    types: u16,
    status: user_data::StatusField,
    counter1: &user_data::Counter,
    counter2: &user_data::Counter,
    payload: Option<&[u8]>,
) -> Vec<RecordOutput> {
    let readings = crate::fixed::readings(types, status.bits(), counter1, counter2);
    let raw = payload.and_then(|bytes| bytes.last_chunk::<8>());
    let binary = status.contains(user_data::StatusField::COUNTER_BINARY_SIGNED);
    readings
        .into_iter()
        .enumerate()
        .map(|(index, reading)| RecordOutput {
            index,
            function: if reading.stored {
                "Stored value"
            } else {
                "Actual value"
            }
            .into(),
            storage_number: u64::from(reading.stored),
            tariff: 0,
            subunit: 0,
            quantities: vec![format!("Counter {}", index + 1)],
            value: ValueOutput {
                kind: "decimal".into(),
                value: Some(reading.value.to_string().into()),
            },
            unit: Some(reading.unit.into()),
            data_coding: if binary {
                "32-bit integer"
            } else {
                "8-digit BCD"
            }
            .into(),
            header_hex: format!("{:02X}", reading.unit_byte),
            data_hex: raw
                .and_then(|bytes| bytes.get(index * 4..index * 4 + 4))
                .map(hex_string)
                .unwrap_or_default(),
            manufacturer_fields: Vec::new(),
        })
        .collect()
}

fn collect_records(
    records: Option<&user_data::DataRecords<'_>>,
    identity: Option<&MeterIdentity>,
) -> (Vec<RecordOutput>, Option<(usize, String)>) {
    let Some(records) = records else {
        return (Vec::new(), None);
    };
    let mut output = Vec::new();
    let mut offset = 0usize;
    let mut bases = Vec::new();
    let mut first_error = None;
    let mut profiles = profiles::ProfileHistory::default();
    for item in records.clone() {
        match item {
            Ok(record) => {
                let size = record.raw_bytes().len();
                let mut parsed = record_output(output.len(), &record);
                if let (Some(identity), [0x0f, tail @ ..]) = (identity, record.raw_bytes()) {
                    parsed.manufacturer_fields = manufacturer_fields(identity, tail);
                }
                if let Some((name, value)) =
                    identity.and_then(|id| multical_energy_register(id, record.raw_bytes()))
                {
                    parsed.quantities = vec![name.into()];
                    parsed.unit = None;
                    parsed.value = ValueOutput {
                        kind: "decimal".into(),
                        value: Some(value.to_string().into()),
                    };
                }
                match profile::decode(&record, &bases, &mut profiles) {
                    Ok(Some(value)) => parsed.value = value,
                    Ok(None) => {}
                    Err(message) => {
                        first_error.get_or_insert_with(|| (offset, message.into()));
                        parsed.value = ValueOutput {
                            kind: "compact_profile".into(),
                            value: None,
                        };
                    }
                }
                if matches!(
                    record.value(),
                    Some(DataType::Number(_) | DataType::LossyNumber(_))
                ) {
                    bases.push(record.raw_bytes);
                }
                output.push(parsed);
                offset += size;
            }
            Err(error) => {
                first_error.get_or_insert_with(|| (offset, error.to_string()));
                break;
            }
        }
    }
    (output, first_error)
}

fn multical_energy_register(identity: &MeterIdentity, raw: &[u8]) -> Option<(&'static str, u32)> {
    if identity.manufacturer_code.as_deref() != Some("KAM") {
        return None;
    }
    match (identity.version?, identity.device_type_code?) {
        (0x34, 0x04 | 0x0a..=0x0d) | (0x35 | 0x40, 0x04 | 0x0c | 0x0d) | (0x19 | 0x39, 0x04) => (),
        _ => return None,
    }
    let [0x04 | 0x44, 0xff, register, bytes @ ..] = raw else {
        return None;
    };
    let name = match register {
        7 => "Flow temperature integral (E8 register count)",
        8 => "Return temperature integral (E9 register count)",
        _ => return None,
    };
    Some((name, u32::from_le_bytes(bytes.try_into().ok()?)))
}

fn manufacturer_fields(identity: &MeterIdentity, tail: &[u8]) -> Vec<ManufacturerField> {
    use m_bus_manufacturer::{decode, Integer, MeterInfo, Value, BUILTIN};
    let meter = MeterInfo {
        manufacturer: identity
            .manufacturer_code
            .as_deref()
            .and_then(|code| code.as_bytes().try_into().ok()),
        version: identity.version,
        device: identity.device_type_code.map(Into::into),
    };
    let mut fields = Vec::new();
    let result = decode(BUILTIN, &meter, tail, &mut |field| {
        let value = match &field.value {
            Value::Integer(value) => {
                let digits = match value {
                    Integer::Unsigned(n) => n.to_string(),
                    Integer::Signed(n) => n.to_string(),
                };
                scaled_decimal(&digits, field.exponent.into(), None)
            }
            Value::Bytes(bytes) => hex_string(bytes),
            Value::Date(date) => match date {
                m_bus_manufacturer::DateValue::Date { day, month, year } => {
                    temporal_value("date", day, month, year, None, None, None)
                }
                m_bus_manufacturer::DateValue::DateTime {
                    day,
                    month,
                    year,
                    hour,
                    minute,
                } => temporal_value("datetime", day, month, year, Some(hour), Some(minute), None),
            }
            .value
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_default(),
        };
        fields.push(ManufacturerField {
            name: field.name.to_owned(),
            value,
            unit: unit_output(field.units.iter().copied()),
            labels: field.active_labels().map(str::to_owned).collect(),
            range: field.range,
        });
    });
    if !matches!(result, Some(Ok(summary)) if summary.consumed > 0) {
        fields.clear();
    }
    fields
}

fn record_output(index: usize, record: &user_data::DataRecord<'_>) -> RecordOutput {
    let information = record.data_information();
    let value_information = record.value_information();
    let function = information
        .map(|value| match value.data_field_coding {
            DataFieldCoding::SpecialFunctions(function) => special_function_name(function),
            _ => record_function_name(value.function_field).to_string(),
        })
        .unwrap_or_else(|| special_record_name(record));
    let storage_number = information.map_or(0, |value| value.storage_number);
    let tariff = information.map_or(0, |value| value.tariff);
    let subunit = information.map_or(0, |value| value.device);
    let data_coding = information
        .map(|value| data_coding_name(value.data_field_coding))
        .unwrap_or_else(|| special_record_name(record));
    let quantities = value_information
        .map(|value| value.labels().map(|label| quantity_name(&label)).collect())
        .unwrap_or_default();
    let unit = value_information
        .map(|value| unit_output(value.units()))
        .filter(|unit| !unit.is_empty());

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
        manufacturer_fields: Vec::new(),
    }
}

pub(crate) fn frame_function_name(function: &m_bus_core::Function) -> String {
    use m_bus_core::Function;

    match function {
        Function::SndNk { prm } => format!("SND_NKE (PRM: {prm})"),
        Function::SndUd { fcb } => format!("SND_UD (FCB: {fcb})"),
        Function::SndUd2 => "SND_UD2".to_string(),
        Function::SndUd3 => "SND_UD3".to_string(),
        Function::SndNr => "SND_NR".to_string(),
        Function::SendIr => "SND_IR".to_string(),
        Function::AccNr => "ACC_NR".to_string(),
        Function::AccDmd => "ACC_DMD".to_string(),
        Function::ReqUd1 { fcb } => format!("REQ_UD1 (FCB: {fcb})"),
        Function::ReqUd2 { fcb } => format!("REQ_UD2 (FCB: {fcb})"),
        Function::RspUd { acd, dfc } => format!("RSP_UD (ACD: {acd}, DFC: {dfc})"),
        Function::Ack => "ACK".to_string(),
        Function::Nack => "NACK".to_string(),
        Function::CnfIr => "CNF_IR".to_string(),
        _ => format!("{function:?}"),
    }
}

const fn record_function_name(function: FunctionField) -> &'static str {
    match function {
        FunctionField::InstantaneousValue => "Instantaneous value",
        FunctionField::MaximumValue => "Maximum value",
        FunctionField::MinimumValue => "Minimum value",
        FunctionField::ValueDuringErrorState => "Value during error state",
        _ => "Unknown",
    }
}

fn special_record_name(record: &user_data::DataRecord<'_>) -> String {
    match record.raw_bytes().first().copied().map(|dif| dif & 0x7F) {
        Some(0x0F) => "Manufacturer specific".to_string(),
        Some(0x1F) => "More records follow".to_string(),
        Some(0x2F) => "Idle filler".to_string(),
        Some(0x7F) => "Global readout request".to_string(),
        _ => "Special function".to_string(),
    }
}

fn data_coding_name(coding: DataFieldCoding) -> String {
    match coding {
        DataFieldCoding::NoData => "No data".to_string(),
        DataFieldCoding::Integer8Bit => "8-bit integer".to_string(),
        DataFieldCoding::Integer16Bit => "16-bit integer".to_string(),
        DataFieldCoding::Integer24Bit => "24-bit integer".to_string(),
        DataFieldCoding::Integer32Bit => "32-bit integer".to_string(),
        DataFieldCoding::Real32Bit => "32-bit real".to_string(),
        DataFieldCoding::Integer48Bit => "48-bit integer".to_string(),
        DataFieldCoding::Integer64Bit => "64-bit integer".to_string(),
        DataFieldCoding::SelectionForReadout => "Selection for readout".to_string(),
        DataFieldCoding::BCD2Digit => "2-digit BCD".to_string(),
        DataFieldCoding::BCD4Digit => "4-digit BCD".to_string(),
        DataFieldCoding::BCD6Digit => "6-digit BCD".to_string(),
        DataFieldCoding::BCD8Digit => "8-digit BCD".to_string(),
        DataFieldCoding::VariableLength => "Variable length".to_string(),
        DataFieldCoding::BCDDigit12 => "12-digit BCD".to_string(),
        DataFieldCoding::DateTypeG => "Date (type G)".to_string(),
        DataFieldCoding::DateTimeTypeF => "Date and time (type F)".to_string(),
        DataFieldCoding::DateTimeTypeJ => "Time (type J)".to_string(),
        DataFieldCoding::DateTimeTypeI => "Date and time (type I)".to_string(),
        DataFieldCoding::SpecialFunctions(function) => special_function_name(function),
        _ => "Unknown".to_string(),
    }
}

fn special_function_name(function: SpecialFunctions) -> String {
    match function {
        SpecialFunctions::ManufacturerSpecific => "Manufacturer specific".to_string(),
        SpecialFunctions::MoreRecordsFollow => "More records follow".to_string(),
        SpecialFunctions::IdleFiller => "Idle filler".to_string(),
        SpecialFunctions::Reserved => "Reserved special function".to_string(),
        SpecialFunctions::GlobalReadoutRequest => "Global readout request".to_string(),
        _ => "Unknown special function".to_string(),
    }
}

fn quantity_name(label: &ValueLabel) -> String {
    match label {
        ValueLabel::Date => "Time point (date)".to_string(),
        ValueLabel::Time => "Time point (time)".to_string(),
        ValueLabel::DateTime | ValueLabel::DateTimeWithSeconds => {
            "Time point (date and time)".to_string()
        }
        ValueLabel::DataContainerForWmbusProtocol => {
            "Data container for wireless M-Bus protocol".to_string()
        }
        ValueLabel::PhaseUtoU => "Phase U-to-U".to_string(),
        ValueLabel::PhaseUtoI => "Phase U-to-I".to_string(),
        ValueLabel::PhaseItoU => "Phase I-to-U".to_string(),
        _ => sentence_case_identifier(&format!("{label:?}")),
    }
}

fn sentence_case_identifier(identifier: &str) -> String {
    let characters = identifier.chars().collect::<Vec<_>>();
    let mut words = Vec::new();
    let mut word = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let previous = index.checked_sub(1).and_then(|value| characters.get(value));
        let next = characters.get(index + 1);
        let boundary = !word.is_empty()
            && character.is_ascii_uppercase()
            && (previous.is_some_and(|value| value.is_ascii_lowercase() || value.is_ascii_digit())
                || (previous.is_some_and(|value| value.is_ascii_uppercase())
                    && next.is_some_and(|value| value.is_ascii_lowercase())));
        if boundary {
            words.push(word);
            word = String::new();
        }
        word.push(character);
    }
    if !word.is_empty() {
        words.push(word);
    }

    words
        .into_iter()
        .enumerate()
        .map(|(index, word)| standardized_word(&word, index == 0))
        .collect::<Vec<_>>()
        .join(" ")
}

fn standardized_word(word: &str, first: bool) -> String {
    match word {
        "Vif" => "VIF".to_string(),
        "Vife" => "VIFE".to_string(),
        "Obis" => "OBIS".to_string(),
        "Wmbus" => "wireless M-Bus".to_string(),
        _ if word
            .chars()
            .filter(|character| character.is_ascii_alphabetic())
            .all(|character| character.is_ascii_uppercase()) =>
        {
            word.to_string()
        }
        _ if first => word.to_string(),
        _ => word.to_ascii_lowercase(),
    }
}

fn value_output(record: &user_data::DataRecord<'_>) -> ValueOutput {
    if let Some(time) = profile::time_point(record) {
        return time;
    }
    let information = record.data_information();
    let value_information = record.value_information();
    let scale = value_information.map(|value| value.decimal_scale_exponent);
    let offset = record_offset_power10(record);

    let raw_value = record
        .raw_bytes()
        .get(record.data_record_header.get_size()..);
    match record.value() {
        Some(DataType::Number(number)) | Some(DataType::LossyNumber(number)) => {
            if information
                .is_some_and(|value| value.data_field_coding == DataFieldCoding::Real32Bit)
            {
                let (number_value, special) = finite_number(*number);
                return ValueOutput {
                    kind: "float".to_string(),
                    value: number_value
                        .and_then(serde_json::Number::from_f64)
                        .map(serde_json::Value::Number)
                        .or_else(|| special.map(serde_json::Value::String)),
                };
            }

            let significand =
                exact_significand(record, *number).unwrap_or_else(|| format!("{number:.0}"));
            let scale_value = scale.unwrap_or(0);
            let decimal = scaled_decimal(&significand, scale_value, offset);
            ValueOutput {
                kind: "decimal".to_string(),
                value: Some(serde_json::Value::String(decimal)),
            }
        }
        Some(DataType::Text(text)) => ValueOutput {
            kind: "text".to_string(),
            value: Some(serde_json::Value::String(text.to_string())),
        },
        Some(DataType::Date(..)) if raw_value == Some(&[0, 0]) => ValueOutput {
            kind: "date".into(),
            value: Some(serde_json::Value::Null),
        },
        Some(DataType::DateTime(..))
            if matches!(raw_value, Some([0, 0, 0, 0]) | Some([0x80..=0xff, _, _, _])) =>
        {
            ValueOutput {
                kind: "datetime".into(),
                value: Some(serde_json::Value::Null),
            }
        }
        Some(DataType::Date(day, month, year)) => {
            temporal_value("date", day, month, year, None, None, None)
        }
        Some(DataType::Time(second, minute, hour)) => temporal_value(
            "time",
            &SingleEveryOrInvalid::Invalid(),
            &SingleEveryOrInvalid::Invalid(),
            &SingleEveryOrInvalid::Invalid(),
            Some(hour),
            Some(minute),
            Some(second),
        ),
        Some(DataType::DateTime(day, month, year, hour, minute)) => {
            temporal_value("datetime", day, month, year, Some(hour), Some(minute), None)
        }
        Some(DataType::DateTimeWithSeconds(day, month, year, hour, minute, second)) => {
            temporal_value(
                "datetime",
                day,
                month,
                year,
                Some(hour),
                Some(minute),
                Some(second),
            )
        }
        Some(DataType::ManufacturerSpecific(_)) => ValueOutput {
            kind: "manufacturer_specific".to_string(),
            value: None,
        },
        None => ValueOutput {
            kind: "none".to_string(),
            value: None,
        },
        Some(_) => ValueOutput {
            kind: "unknown".to_string(),
            value: None,
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

    let value = temporal_iso(day, month, year, hour, minute, second, kind)
        .map(serde_json::Value::String)
        .unwrap_or_else(|| serde_json::Value::Object(components));
    ValueOutput {
        kind: kind.to_string(),
        value: Some(value),
    }
}

fn component_json<T>(component: &SingleEveryOrInvalid<T>) -> serde_json::Value
where
    T: Serialize,
{
    match component {
        SingleEveryOrInvalid::Single(value) => {
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
        }
        SingleEveryOrInvalid::Every() => serde_json::Value::String("every".to_string()),
        SingleEveryOrInvalid::Invalid() => serde_json::Value::String("invalid".to_string()),
        _ => serde_json::Value::String("unknown".to_string()),
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
    if digits.bytes().all(|digit| digit == b'0') {
        return "0".to_string();
    }
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

fn unit_output(units: impl Iterator<Item = Unit> + Clone) -> String {
    units
        .clone()
        .map(|unit| ucum_component(&unit))
        .collect::<Option<Vec<_>>>()
        .map_or_else(
            || units.map(|unit| unit.to_string()).collect::<String>(),
            |components| components.join("."),
        )
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
        UnitName::BritishThermalUnit => "[Btu_IT]",
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
        "unit",
        "data_coding",
        "header_hex",
        "data_hex",
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
    if decoded.transport.secondary_address_mask.is_some() {
        headers.push("secondary_address_mask".to_string());
    }
    if decoded.application.is_some() {
        headers.extend(
            [
                "application_protocol",
                "application_payload_hex",
                "application_decoded",
            ]
            .map(str::to_string),
        );
    }
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
    if let Some(mask) = &decoded.transport.secondary_address_mask {
        row.push(mask.clone());
    }
    if let Some(application) = &decoded.application {
        row.extend([
            application.protocol.clone(),
            application.payload_hex.clone(),
            application
                .decoded
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ]);
    }
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
        value_display(record),
        record.unit.clone().unwrap_or_default(),
        record.data_coding.clone(),
        record.header_hex.clone(),
        record.data_hex.clone(),
    ]
}

fn value_display(record: &RecordOutput) -> String {
    match &record.value.value {
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(serde_json::Value::Number(value)) => value.to_string(),
        Some(serde_json::Value::Object(components)) => components
            .iter()
            .map(|(name, value)| {
                let value = value
                    .as_str()
                    .map_or_else(|| value.to_string(), ToString::to_string);
                format!("{name}={value}")
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(value) => value.to_string(),
        None if record.value.kind == "manufacturer_specific" => record.data_hex.replace(' ', ""),
        None if record.value.kind == "none" => "No data".to_string(),
        None if record.value.kind == "unknown" => "Unsupported value type".to_string(),
        None => String::new(),
    }
}

fn record_hex(record: &RecordOutput) -> String {
    match (record.header_hex.is_empty(), record.data_hex.is_empty()) {
        (false, false) => format!("{} {}", record.header_hex, record.data_hex),
        (false, true) => record.header_hex.clone(),
        (true, false) => record.data_hex.clone(),
        (true, true) => String::new(),
    }
}

fn render_table(
    decoded: &DecodedOutput,
    width: usize,
    style: TableStyle,
) -> Result<String, OutputError> {
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
    push_text(&mut output, "", &title, width, style, Cell::Label);

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
    if let Some(mask) = &decoded.transport.secondary_address_mask {
        summary.push(("Secondary address mask".to_string(), mask.clone()));
    }
    if let Some(application) = &decoded.application {
        summary.push(("Application".to_string(), application.protocol.clone()));
        if let Some(decoded) = &application.decoded {
            summary.push(("Application data".to_string(), decoded.to_string()));
        }
        if !application.payload_hex.is_empty() {
            summary.push((
                "Application bytes".to_string(),
                application.payload_hex.clone(),
            ));
        }
    }
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
    output.push_str(&summary_fields(&summary, width, style));

    for record in &decoded.records {
        push_record_reading(&mut output, record, width, style);
    }
    output.push('\n');
    push_text(
        &mut output,
        "",
        "OMS 5.0.1 checks",
        width,
        style,
        Cell::Label,
    );
    for finding in &decoded.oms_5_0_1_checks {
        let (status, cell) = match finding.status {
            telegram::RuleStatus::Passed => ("Passed", Cell::Passed),
            telegram::RuleStatus::Failed => ("Failed", Cell::Failed),
            telegram::RuleStatus::CouldNotCheck => ("Could not check", Cell::Text),
            telegram::RuleStatus::NotApplicable => ("Not applicable", Cell::Text),
        };
        let mut heading = format!("{status} · {}", finding.rule);
        if let Some(number) = finding.record_number {
            heading.push_str(&format!(" · record {number}"));
        }
        push_text(&mut output, "", &heading, width, style, cell);
        let detail = format!("{} ({})", finding.detail, finding.reference);
        push_text(&mut output, "  ", &detail, width, style, Cell::Text);
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
        output.push_str(&summary_fields(&rows, width, style));
    }
    Ok(output)
}

fn push_record_reading(
    output: &mut String,
    record: &RecordOutput,
    width: usize,
    style: TableStyle,
) {
    let (quantity, value) = if record.value.kind == "manufacturer_specific" {
        (
            "Manufacturer data".to_string(),
            format!("{} bytes", record.data_hex.split_ascii_whitespace().count()),
        )
    } else {
        (quantity_label(record), value_with_unit(record))
    };
    output.push('\n');
    let label = format!("#{}  {quantity}:", record.index);
    let heading = format!("{label} {value}");
    if !heading.contains('\n') && UnicodeWidthStr::width(heading.as_str()) <= width {
        style.write(output, Cell::Label, &label);
        output.push(' ');
        style.write(output, Cell::Reading, &value);
        output.push('\n');
    } else {
        push_text(output, "", &label, width, style, Cell::Label);
        push_text(output, "  ", &value, width, style, Cell::Reading);
    }
    for field in &record.manufacturer_fields {
        let mut line = format!("{}: {}", field.name, field.value);
        if !field.unit.is_empty() {
            line.push(' ');
            line.push_str(&field.unit);
        }
        if !field.labels.is_empty() {
            line.push_str(" · ");
            line.push_str(&field.labels.join(", "));
        }
        push_text(output, "  ", &line, width, style, Cell::Reading);
    }
    let mut metadata = record.function.clone();
    let selectors = format!(
        "S/T/U {}/{}/{}",
        record.storage_number, record.tariff, record.subunit
    );
    for field in [record.data_coding.as_str(), &selectors] {
        if field == record.function {
            continue;
        }
        if UnicodeWidthStr::width(metadata.as_str()) + 3 + UnicodeWidthStr::width(field) > width - 2
        {
            push_text(output, "  ", &metadata, width, style, Cell::Text);
            metadata = field.to_owned();
        } else {
            metadata.push_str(" · ");
            metadata.push_str(field);
        }
    }
    push_text(output, "  ", &metadata, width, style, Cell::Text);
    push_text(
        output,
        "  ",
        &format!("Raw: {}", record_hex(record)),
        width,
        style,
        Cell::Bytes,
    );
}

fn quantity_label(record: &RecordOutput) -> String {
    if record.quantities.is_empty() {
        "Reading".to_string()
    } else {
        record.quantities.join(", ")
    }
}

fn value_with_unit(record: &RecordOutput) -> String {
    let value = value_display(record);
    match &record.unit {
        Some(unit) => format!("{value} {unit}"),
        None => value,
    }
}

fn reading(record: &RecordOutput) -> String {
    format!("{}: {}", quantity_label(record), value_with_unit(record))
}

fn push_text(
    output: &mut String,
    indent: &str,
    text: &str,
    width: usize,
    style: TableStyle,
    cell: Cell,
) {
    for line in wrap_display(text, width - indent.len()) {
        output.push_str(indent);
        style.write(output, cell, &line);
        output.push('\n');
    }
}

fn summary_fields(rows: &[(String, String)], width: usize, style: TableStyle) -> String {
    let mut output = String::new();
    let label_width = rows
        .iter()
        .map(|(key, _)| UnicodeWidthStr::width(key.as_str()))
        .max()
        .unwrap_or(0);
    for (key, value) in rows {
        let key_width = UnicodeWidthStr::width(key.as_str());
        let cell = if key == "Website" {
            Cell::Website(value)
        } else {
            Cell::Text
        };
        if !value.contains('\n')
            && label_width + 2 + UnicodeWidthStr::width(value.as_str()) <= width
        {
            style.write(&mut output, Cell::Label, key);
            output.push_str(&" ".repeat(label_width + 2 - key_width));
            style.write(&mut output, cell, value);
            output.push('\n');
        } else {
            push_text(&mut output, "", key, width, style, Cell::Label);
            push_text(&mut output, "  ", value, width, style, cell);
        }
    }
    output
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
        for (index, word) in source_line.split(' ').enumerate() {
            let word_width = UnicodeWidthStr::width(word);
            if index > 0 {
                if line_width + 1 + word_width > width {
                    lines.push(std::mem::take(&mut line));
                    line_width = 0;
                } else {
                    line.push(' ');
                    line_width += 1;
                }
            }
            if word_width <= width {
                line.push_str(word);
                line_width += word_width;
                continue;
            }
            for character in word.chars() {
                let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
                if line_width + character_width > width {
                    lines.push(std::mem::take(&mut line));
                    line_width = 0;
                }
                line.push(character);
                line_width += character_width;
            }
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
    if let Some(mask) = &decoded.transport.secondary_address_mask {
        output.push_str(&format!(
            "        SLAVE_SELECTION[\"Secondary address mask · {}\"]\n",
            mermaid_escape(mask)
        ));
        frame_nodes.push("SLAVE_SELECTION");
    }
    if let Some(application) = &decoded.application {
        output.push_str(&format!(
            "        APPLICATION[\"Application · {}\"]\n",
            mermaid_escape(&application.protocol)
        ));
        frame_nodes.push("APPLICATION");
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
    m_bus_core::hex::encode_upper(data, false)
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
        assert_eq!(decode_hex_bytes("68,3D").unwrap(), [0x68, 0x3d]);
        assert!(decode_hex_bytes("0x6").is_err());
    }

    #[test]
    fn canonical_json_has_versioned_layers() {
        let rendered =
            render_hex(WIRED_FRAME, OutputFormat::Json, &RenderOptions::default()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["schema_version"], 3);
        assert_eq!(value["protocol"], "wired");
        assert!(value["records"].is_array());
        assert!(value.get("summary").is_none());
    }

    #[test]
    fn canonical_units_use_an_unlabelled_standardized_value() {
        assert_eq!(
            unit_output(
                [
                    Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    },
                    Unit {
                        name: UnitName::Hour,
                        exponent: -1,
                    },
                ]
                .into_iter()
            ),
            "m3.h-1"
        );
        assert_eq!(
            unit_output(
                [Unit {
                    name: UnitName::ReactiveWatt,
                    exponent: 1,
                }]
                .into_iter()
            ),
            "W (reactive)"
        );
        assert_eq!(
            unit_output(
                [
                    Unit {
                        name: UnitName::Meter,
                        exponent: 1
                    },
                    Unit {
                        name: UnitName::ReactiveWatt,
                        exponent: 1
                    },
                    Unit {
                        name: UnitName::Hour,
                        exponent: -1
                    },
                ]
                .into_iter()
            ),
            "mW (reactive)h⁻¹"
        );
        assert_eq!(unit_output(core::iter::empty()), "");
    }

    #[test]
    fn canonical_names_follow_m_bus_terms_and_preserve_acronyms() {
        assert_eq!(quantity_name(&ValueLabel::VolumeFlow), "Volume flow");
        assert_eq!(quantity_name(&ValueLabel::RFPowerLevel), "RF power level");
        assert_eq!(
            quantity_name(&ValueLabel::ObisDeclaration),
            "OBIS declaration"
        );
    }

    #[test]
    fn standalone_records_use_the_canonical_record_schema() {
        let records = decode_data_records(&[0x03, 0x13, 0x15, 0x31, 0x00]).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].function, "Instantaneous value");
        assert_eq!(records[0].quantities, ["Volume"]);
        assert_eq!(records[0].unit.as_deref(), Some("m3"));
        assert_eq!(records[0].data_coding, "24-bit integer");
        assert_eq!(
            records[0]
                .value
                .value
                .as_ref()
                .and_then(|value| value.as_str()),
            Some("12.565")
        );
    }

    #[cfg(feature = "decryption-mode-5")]
    #[test]
    fn zenner_monthly_water_profile_uses_the_stored_volume_as_its_base() {
        let frame = concat!(
            "6644496A3100015514377203926314496A00075000500598A78E0D71AA6358EEBD",
            "0B20BFDF99EDA2D22FA25314F3F1B84470898E495303923770BA8DDA97C964F0EA",
            "6CE24F5650C0A6CDF3DE37DE33FBFBEBACE4009BB0D8EBA2CBE80433FF131328206020B1BF",
        );
        let options = DecodeOptions {
            key: Some(
                hex::decode("F8B24F12F9D113F680BEE765FDE67EC0")
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
            ..DecodeOptions::default()
        };
        let decoded = decode_hex(frame, &options).unwrap();
        let expected = serde_json::json!({
            "interval": "1 month",
            "readings": ["60.063", "55.912", "52.342", "48.965", "43.99", "40.374", "34.701",
                         "29.11", "23.315", "17.584", "13.356", "8.638", "4.677", "0.421"]
        });
        assert_eq!(decoded.records.len(), 6);
        let profile = &decoded.records[4];
        assert_eq!(profile.unit.as_deref(), Some("m3"));
        assert_eq!(profile.value.kind, "inverse_compact_profile");
        assert_eq!(profile.value.value.as_ref(), Some(&expected));
        assert_eq!(decoded.records[5].quantities, ["Error flags"]);
        let xml = render_hex(
            frame,
            OutputFormat::Xml,
            &RenderOptions {
                decode: options,
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert!(xml.contains("60.063"));
        assert!(xml.contains("0.421"));
    }

    #[test]
    fn inverse_profile_keeps_differences_when_the_base_is_for_another_register() {
        let records =
            decode_data_records_hex("4C 13 00 01 00 00 8D 04 93 13 04 FA FE 23 F1").unwrap();
        assert_eq!(
            records[1].value.value,
            Some(serde_json::json!({
                "interval": "1 month", "differences": ["-0.123"]
            }))
        );
    }

    #[test]
    fn unreadable_inverse_samples_do_not_produce_invented_readings() {
        let records =
            decode_data_records_hex("8C 04 13 00 10 00 00 8D 04 93 13 05 F9 FE 03 AA 01").unwrap();
        assert_eq!(
            records[1].value.value,
            Some(serde_json::json!({
                "interval": "1 month", "readings": ["0.997", null, null]
            }))
        );
        for data in ["FB FE 12 34", "00 01 02", "FA 01 02", "FD FE"] {
            let size = data.split_whitespace().count();
            let record =
                decode_data_records_hex(&format!("8D 04 93 13 {size:02X} {data}")).unwrap();
            assert_eq!(
                record[0].value.value,
                Some(serde_json::json!({
                    "raw_hex": data.replace(' ', "")
                }))
            );
        }
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
