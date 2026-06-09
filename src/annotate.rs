//! Byte-level annotation of M-Bus frames.
//!
//! This module provides [`annotate_frame`], which takes raw frame bytes and returns
//! a flat `Vec<ByteSegment>` labeling every byte with its protocol role. This enables
//! UIs to render hex views with hover tooltips, coloring by layer, and grouping of
//! related fields (e.g., all bytes of a data record).

use crate::user_data::data_record::DataRecord;
use crate::MbusError;
use std::borrow::Cow;
use std::fmt;
use wired_mbus_link_layer::WiredFrame;

/// The protocol role of a byte range within an M-Bus frame.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum SegmentKind {
    // Link layer
    StartByte,
    Length,
    CField,
    AField,
    Checksum,
    StopByte,
    // Application layer header
    CiField,
    IdentificationNumber,
    ManufacturerCode,
    Version,
    DeviceType,
    AccessNumber,
    Status,
    ConfigurationField,
    EncryptionConfigByte,
    // Data record fields
    Dif,
    Dife,
    Vif,
    Vife,
    PlaintextVif,
    DataPayload,
    // Wireless
    LField,
    WirelessManufacturerId,
    Crc,
    // Extended link layer
    ExtendedLinkLayer,
    // Opaque payloads
    EncryptedPayload,
    ManufacturerSpecific,
    // Fallback
    IdleFiller,
    Unknown,
}

impl fmt::Display for SegmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartByte => write!(f, "Start Byte"),
            Self::Length => write!(f, "Length"),
            Self::CField => write!(f, "C Field"),
            Self::AField => write!(f, "A Field"),
            Self::Checksum => write!(f, "Checksum"),
            Self::StopByte => write!(f, "Stop Byte"),
            Self::CiField => write!(f, "CI Field"),
            Self::IdentificationNumber => write!(f, "Identification Number"),
            Self::ManufacturerCode => write!(f, "Manufacturer Code"),
            Self::Version => write!(f, "Version"),
            Self::DeviceType => write!(f, "Device Type"),
            Self::AccessNumber => write!(f, "Access Number"),
            Self::Status => write!(f, "Status"),
            Self::ConfigurationField => write!(f, "Configuration Field"),
            Self::EncryptionConfigByte => write!(f, "Encryption Config Byte"),
            Self::Dif => write!(f, "DIF"),
            Self::Dife => write!(f, "DIFE"),
            Self::Vif => write!(f, "VIF"),
            Self::Vife => write!(f, "VIFE"),
            Self::PlaintextVif => write!(f, "Plaintext VIF"),
            Self::DataPayload => write!(f, "Data"),
            Self::LField => write!(f, "L Field"),
            Self::WirelessManufacturerId => write!(f, "Manufacturer ID"),
            Self::Crc => write!(f, "CRC"),
            Self::ExtendedLinkLayer => write!(f, "Extended Link Layer"),
            Self::EncryptedPayload => write!(f, "Encrypted Payload"),
            Self::ManufacturerSpecific => write!(f, "Manufacturer Specific"),
            Self::IdleFiller => write!(f, "Idle Filler"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The protocol layer a segment belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Layer {
    Frame,
    AppHeader,
    RecordField,
}

/// A labeled byte range within an M-Bus frame.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ByteSegment {
    /// Inclusive byte offset in the original frame.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
    /// Type-safe field identifier.
    pub kind: SegmentKind,
    /// Hover tooltip text.
    pub detail: Cow<'static, str>,
    /// Data record index, linking related DIF/VIF/data segments.
    pub group: Option<usize>,
    /// Protocol layer.
    pub layer: Layer,
}

/// Annotate every byte in a raw M-Bus frame.
///
/// Returns a contiguous `Vec<ByteSegment>` where each segment's `end` equals the
/// next segment's `start`, covering the entire input. The `start`/`end` offsets
/// always refer to positions in `data` (the original input).
///
/// # Errors
///
/// Returns `MbusError` if the frame cannot be parsed as either wired or wireless.
pub fn annotate_frame(data: &[u8]) -> Result<Vec<ByteSegment>, MbusError> {
    // Try wired first
    if let Ok(segments) = annotate_wired(data) {
        return Ok(segments);
    }

    // Try wireless with Format A CRC stripping
    let mut crc_buf = [0u8; 512];
    if let Some(stripped) = wireless_mbus_link_layer::strip_format_a_crcs(data, &mut crc_buf) {
        if let Ok(segments) = annotate_wireless_format_a(data, stripped) {
            return Ok(segments);
        }
    }

    // Try wireless without CRC stripping (already stripped / Format B)
    if let Ok(segments) = annotate_wireless_inner(data) {
        return Ok(segments);
    }

    Err(MbusError::FrameError(
        m_bus_core::FrameError::InvalidStartByte,
    ))
}

// ── Wired frame annotation ──────────────────────────────────────────────────

fn annotate_wired(data: &[u8]) -> Result<Vec<ByteSegment>, MbusError> {
    let frame = WiredFrame::try_from(data)?;
    let mut segments = Vec::new();

    match frame {
        WiredFrame::SingleCharacter { character } => {
            segments.push(ByteSegment {
                start: 0,
                end: 1,
                kind: SegmentKind::StartByte,
                detail: Cow::Owned(format!("Single Character: 0x{:02X}", character)),
                group: None,
                layer: Layer::Frame,
            });
        }
        WiredFrame::ShortFrame { function, address } => {
            // [0x10] [C] [A] [CS] [0x16]
            segments.push(ByteSegment {
                start: 0,
                end: 1,
                kind: SegmentKind::StartByte,
                detail: Cow::Borrowed("Start: 0x10"),
                group: None,
                layer: Layer::Frame,
            });
            segments.push(ByteSegment {
                start: 1,
                end: 2,
                kind: SegmentKind::CField,
                detail: Cow::Owned(format!("C Field: {}", function)),
                group: None,
                layer: Layer::Frame,
            });
            segments.push(ByteSegment {
                start: 2,
                end: 3,
                kind: SegmentKind::AField,
                detail: Cow::Owned(format!("Address: {}", address)),
                group: None,
                layer: Layer::Frame,
            });
            segments.push(ByteSegment {
                start: 3,
                end: 4,
                kind: SegmentKind::Checksum,
                detail: Cow::Owned(format!(
                    "Checksum: 0x{:02X}",
                    data.get(3).copied().unwrap_or(0)
                )),
                group: None,
                layer: Layer::Frame,
            });
            segments.push(ByteSegment {
                start: 4,
                end: 5,
                kind: SegmentKind::StopByte,
                detail: Cow::Borrowed("Stop: 0x16"),
                group: None,
                layer: Layer::Frame,
            });
        }
        WiredFrame::LongFrame {
            function,
            address,
            data: user_data_slice,
        } => {
            let is_control = false;
            annotate_long_or_control_frame(
                &mut segments,
                data,
                &function,
                &address,
                user_data_slice,
                is_control,
            );
        }
        WiredFrame::ControlFrame {
            function,
            address,
            data: user_data_slice,
        } => {
            let is_control = true;
            annotate_long_or_control_frame(
                &mut segments,
                data,
                &function,
                &address,
                user_data_slice,
                is_control,
            );
        }
        _ => {
            segments.push(ByteSegment {
                start: 0,
                end: data.len(),
                kind: SegmentKind::Unknown,
                detail: Cow::Borrowed("Unknown frame type"),
                group: None,
                layer: Layer::Frame,
            });
        }
    }

    Ok(segments)
}

fn annotate_long_or_control_frame(
    segments: &mut Vec<ByteSegment>,
    data: &[u8],
    function: &m_bus_core::Function,
    address: &wired_mbus_link_layer::Address,
    user_data_slice: &[u8],
    is_control: bool,
) {
    let l = data.get(1).copied().unwrap_or(0) as usize;

    // [0x68] [L] [L] [0x68] [C] [A] [UserData...] [CS] [0x16]
    segments.push(ByteSegment {
        start: 0,
        end: 1,
        kind: SegmentKind::StartByte,
        detail: Cow::Borrowed("Start: 0x68"),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 1,
        end: 3,
        kind: SegmentKind::Length,
        detail: Cow::Owned(format!("Length: {} (repeated)", l)),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 3,
        end: 4,
        kind: SegmentKind::StartByte,
        detail: Cow::Borrowed("Start: 0x68 (repeated)"),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 4,
        end: 5,
        kind: SegmentKind::CField,
        detail: Cow::Owned(format!(
            "C Field: {}{}",
            function,
            if is_control { " (Control)" } else { "" }
        )),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 5,
        end: 6,
        kind: SegmentKind::AField,
        detail: Cow::Owned(format!("Address: {}", address)),
        group: None,
        layer: Layer::Frame,
    });

    // Annotate the user data (application layer) within the frame
    let user_data_start = 6;
    let user_data_end = user_data_start + user_data_slice.len();

    if !is_control && !user_data_slice.is_empty() {
        annotate_application_layer(segments, data, user_data_start, user_data_slice);
    } else if !user_data_slice.is_empty() {
        segments.push(ByteSegment {
            start: user_data_start,
            end: user_data_end,
            kind: SegmentKind::Unknown,
            detail: Cow::Borrowed("Control frame data"),
            group: None,
            layer: Layer::AppHeader,
        });
    }

    // Checksum
    let cs_offset = user_data_end;
    if cs_offset < data.len() {
        segments.push(ByteSegment {
            start: cs_offset,
            end: cs_offset + 1,
            kind: SegmentKind::Checksum,
            detail: Cow::Owned(format!(
                "Checksum: 0x{:02X}",
                data.get(cs_offset).copied().unwrap_or(0)
            )),
            group: None,
            layer: Layer::Frame,
        });
    }
    // Stop byte
    let stop_offset = cs_offset + 1;
    if stop_offset < data.len() {
        segments.push(ByteSegment {
            start: stop_offset,
            end: stop_offset + 1,
            kind: SegmentKind::StopByte,
            detail: Cow::Borrowed("Stop: 0x16"),
            group: None,
            layer: Layer::Frame,
        });
    }
}

// ── Application layer annotation ────────────────────────────────────────────

fn annotate_application_layer(
    segments: &mut Vec<ByteSegment>,
    frame_data: &[u8],
    base: usize,
    app_data: &[u8],
) {
    let Some(&ci) = app_data.first() else {
        return;
    };

    match ci {
        // Long TPL header (CI=0x72 or CI=0x76)
        0x72 | 0x76 => {
            if app_data.len() < 13 {
                segments.push(ByteSegment {
                    start: base,
                    end: base + app_data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Incomplete long TPL header"),
                    group: None,
                    layer: Layer::AppHeader,
                });
                return;
            }
            annotate_long_tpl_header(segments, frame_data, base, app_data);

            // Data records start at offset 13
            let records_start = base + 13;
            let records_data = &app_data[13..];
            let is_encrypted = is_long_tpl_encrypted(app_data);
            if is_encrypted {
                if !records_data.is_empty() {
                    segments.push(ByteSegment {
                        start: records_start,
                        end: records_start + records_data.len(),
                        kind: SegmentKind::EncryptedPayload,
                        detail: Cow::Borrowed("Encrypted variable data"),
                        group: None,
                        layer: Layer::RecordField,
                    });
                }
            } else {
                annotate_data_records(segments, records_start, records_data);
            }
        }

        // Short TPL header (CI=0x7A or encrypted CI=0xA0..=0xAF)
        0x7A | 0xA0..=0xAF => {
            let has_enc_config = ci == 0xA0;
            let skip_count: usize = if has_enc_config { 2 } else { 1 };
            let data_block_offset: usize = if has_enc_config { 6 } else { 5 };

            if app_data.len() < data_block_offset {
                segments.push(ByteSegment {
                    start: base,
                    end: base + app_data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Incomplete short TPL header"),
                    group: None,
                    layer: Layer::AppHeader,
                });
                return;
            }

            // CI field
            segments.push(ByteSegment {
                start: base,
                end: base + 1,
                kind: SegmentKind::CiField,
                detail: Cow::Owned(format!("CI: 0x{:02X} (Short TPL)", ci)),
                group: None,
                layer: Layer::AppHeader,
            });

            let mut offset = base + 1;

            // Extra encryption config byte for CI=0xA0
            if has_enc_config {
                segments.push(ByteSegment {
                    start: offset,
                    end: offset + 1,
                    kind: SegmentKind::EncryptionConfigByte,
                    detail: Cow::Owned(format!(
                        "Encryption config: 0x{:02X}",
                        app_data.get(1).copied().unwrap_or(0)
                    )),
                    group: None,
                    layer: Layer::AppHeader,
                });
                offset += 1;
            }

            // Access Number
            segments.push(ByteSegment {
                start: offset,
                end: offset + 1,
                kind: SegmentKind::AccessNumber,
                detail: Cow::Owned(format!(
                    "Access Number: {}",
                    app_data.get(skip_count).copied().unwrap_or(0)
                )),
                group: None,
                layer: Layer::AppHeader,
            });
            offset += 1;

            // Status
            segments.push(ByteSegment {
                start: offset,
                end: offset + 1,
                kind: SegmentKind::Status,
                detail: Cow::Owned(format!(
                    "Status: 0x{:02X}",
                    app_data.get(skip_count + 1).copied().unwrap_or(0)
                )),
                group: None,
                layer: Layer::AppHeader,
            });
            offset += 1;

            // Configuration Field (2 bytes)
            segments.push(ByteSegment {
                start: offset,
                end: offset + 2,
                kind: SegmentKind::ConfigurationField,
                detail: Cow::Owned(format!(
                    "Configuration: 0x{:02X}{:02X}",
                    app_data.get(skip_count + 2).copied().unwrap_or(0),
                    app_data.get(skip_count + 3).copied().unwrap_or(0),
                )),
                group: None,
                layer: Layer::AppHeader,
            });
            offset += 2;

            // Variable data block
            let records_data = &app_data[data_block_offset..];
            let is_encrypted = is_short_tpl_encrypted(app_data, skip_count);
            if is_encrypted {
                if !records_data.is_empty() {
                    segments.push(ByteSegment {
                        start: offset,
                        end: offset + records_data.len(),
                        kind: SegmentKind::EncryptedPayload,
                        detail: Cow::Borrowed("Encrypted variable data"),
                        group: None,
                        layer: Layer::RecordField,
                    });
                }
            } else {
                annotate_data_records(segments, offset, records_data);
            }
        }

        // Application layer without TPL header (CI=0x78)
        0x78 => {
            segments.push(ByteSegment {
                start: base,
                end: base + 1,
                kind: SegmentKind::CiField,
                detail: Cow::Borrowed("CI: 0x78 (Application Layer, no TPL header)"),
                group: None,
                layer: Layer::AppHeader,
            });

            let records_data = &app_data[1..];
            if !records_data.is_empty() {
                annotate_data_records(segments, base + 1, records_data);
            }
        }

        // Extended Link Layer I (CI=0x8C): CI + 2 ELL bytes, then nested application data
        0x8C => {
            let ell_size = 2;
            let total_header = 1 + ell_size; // CI + ELL
            if app_data.len() < total_header {
                segments.push(ByteSegment {
                    start: base,
                    end: base + app_data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Incomplete ELL I header"),
                    group: None,
                    layer: Layer::AppHeader,
                });
                return;
            }

            // CI
            segments.push(ByteSegment {
                start: base,
                end: base + 1,
                kind: SegmentKind::CiField,
                detail: Cow::Borrowed("CI: 0x8C (ELL I)"),
                group: None,
                layer: Layer::AppHeader,
            });

            // ELL bytes
            segments.push(ByteSegment {
                start: base + 1,
                end: base + total_header,
                kind: SegmentKind::ExtendedLinkLayer,
                detail: Cow::Borrowed("ELL I: CC + Access Number"),
                group: None,
                layer: Layer::AppHeader,
            });

            // The rest is nested application data parsed by the application layer
            let inner_data = &app_data[total_header..];
            let inner_base = base + total_header;
            if !inner_data.is_empty() {
                annotate_application_layer(segments, frame_data, inner_base, inner_data);
            }
        }

        // Extended Link Layer II (CI=0x8D): CI + 8 ELL bytes, then variable data block
        0x8D => {
            let ell_size = 8;
            let total_header = 1 + ell_size;
            if app_data.len() < total_header {
                segments.push(ByteSegment {
                    start: base,
                    end: base + app_data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Incomplete ELL II header"),
                    group: None,
                    layer: Layer::AppHeader,
                });
                return;
            }

            segments.push(ByteSegment {
                start: base,
                end: base + 1,
                kind: SegmentKind::CiField,
                detail: Cow::Borrowed("CI: 0x8D (ELL II)"),
                group: None,
                layer: Layer::AppHeader,
            });

            segments.push(ByteSegment {
                start: base + 1,
                end: base + total_header,
                kind: SegmentKind::ExtendedLinkLayer,
                detail: Cow::Borrowed("ELL II: CC + ACC + SN[4] + CRC[2]"),
                group: None,
                layer: Layer::AppHeader,
            });

            // No inner CI/TPL header; remaining bytes are variable data block
            let remaining = &app_data[total_header..];
            let remaining_base = base + total_header;
            if !remaining.is_empty() {
                annotate_data_records(segments, remaining_base, remaining);
            }
        }

        // Extended Link Layer III (CI=0x8E): CI + 16 ELL bytes, then variable data block
        0x8E => {
            let ell_size = 16;
            let total_header = 1 + ell_size;
            if app_data.len() < total_header {
                segments.push(ByteSegment {
                    start: base,
                    end: base + app_data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Incomplete ELL III header"),
                    group: None,
                    layer: Layer::AppHeader,
                });
                return;
            }

            segments.push(ByteSegment {
                start: base,
                end: base + 1,
                kind: SegmentKind::CiField,
                detail: Cow::Borrowed("CI: 0x8E (ELL III)"),
                group: None,
                layer: Layer::AppHeader,
            });

            segments.push(ByteSegment {
                start: base + 1,
                end: base + total_header,
                kind: SegmentKind::ExtendedLinkLayer,
                detail: Cow::Borrowed("ELL III: CC + ACC + MFR[2] + ADDR[6] + SN[4] + CRC[2]"),
                group: None,
                layer: Layer::AppHeader,
            });

            let remaining = &app_data[total_header..];
            let remaining_base = base + total_header;
            if !remaining.is_empty() {
                annotate_data_records(segments, remaining_base, remaining);
            }
        }

        _ => {
            segments.push(ByteSegment {
                start: base,
                end: base + app_data.len(),
                kind: SegmentKind::Unknown,
                detail: Cow::Owned(format!("Unknown CI: 0x{:02X}", ci)),
                group: None,
                layer: Layer::AppHeader,
            });
        }
    }
}

fn annotate_long_tpl_header(
    segments: &mut Vec<ByteSegment>,
    _frame_data: &[u8],
    base: usize,
    app_data: &[u8],
) {
    let ci = app_data.first().copied().unwrap_or(0);
    // CI(0), ID(1..5), Manufacturer(5..7), Version(7), DeviceType(8),
    // AccessNum(9), Status(10), Config(11..13)
    segments.push(ByteSegment {
        start: base,
        end: base + 1,
        kind: SegmentKind::CiField,
        detail: Cow::Owned(format!(
            "CI: 0x{:02X} ({})",
            ci,
            if ci == 0x72 {
                "Variable Data, Long TPL"
            } else {
                "Variable Data, Long TPL, LSB"
            }
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 1,
        end: base + 5,
        kind: SegmentKind::IdentificationNumber,
        detail: Cow::Owned(format!(
            "ID: {:02X}{:02X}{:02X}{:02X}",
            app_data.get(1).copied().unwrap_or(0),
            app_data.get(2).copied().unwrap_or(0),
            app_data.get(3).copied().unwrap_or(0),
            app_data.get(4).copied().unwrap_or(0),
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 5,
        end: base + 7,
        kind: SegmentKind::ManufacturerCode,
        detail: Cow::Owned(format!(
            "Manufacturer: 0x{:02X}{:02X}",
            app_data.get(5).copied().unwrap_or(0),
            app_data.get(6).copied().unwrap_or(0),
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 7,
        end: base + 8,
        kind: SegmentKind::Version,
        detail: Cow::Owned(format!(
            "Version: {}",
            app_data.get(7).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 8,
        end: base + 9,
        kind: SegmentKind::DeviceType,
        detail: Cow::Owned(format!(
            "Device Type: 0x{:02X}",
            app_data.get(8).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 9,
        end: base + 10,
        kind: SegmentKind::AccessNumber,
        detail: Cow::Owned(format!(
            "Access Number: {}",
            app_data.get(9).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 10,
        end: base + 11,
        kind: SegmentKind::Status,
        detail: Cow::Owned(format!(
            "Status: 0x{:02X}",
            app_data.get(10).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::AppHeader,
    });
    segments.push(ByteSegment {
        start: base + 11,
        end: base + 13,
        kind: SegmentKind::ConfigurationField,
        detail: Cow::Owned(format!(
            "Configuration: 0x{:02X}{:02X}",
            app_data.get(11).copied().unwrap_or(0),
            app_data.get(12).copied().unwrap_or(0),
        )),
        group: None,
        layer: Layer::AppHeader,
    });
}

// ── Data record annotation ──────────────────────────────────────────────────

fn annotate_data_records(segments: &mut Vec<ByteSegment>, base: usize, data: &[u8]) {
    let mut offset = 0usize;
    let mut record_index = 0usize;

    while offset < data.len() {
        let Some(&dif_byte) = data.get(offset) else {
            break;
        };

        // Check for special functions
        if dif_byte & 0x0F == 0x0F {
            match dif_byte {
                0x2F => {
                    // Idle filler - single byte
                    segments.push(ByteSegment {
                        start: base + offset,
                        end: base + offset + 1,
                        kind: SegmentKind::IdleFiller,
                        detail: Cow::Borrowed("Idle Filler: 0x2F"),
                        group: None,
                        layer: Layer::RecordField,
                    });
                    offset += 1;
                    continue;
                }
                0x0F | 0x1F => {
                    // Manufacturer specific / more records follow - consumes all remaining
                    let kind = SegmentKind::ManufacturerSpecific;
                    segments.push(ByteSegment {
                        start: base + offset,
                        end: base + data.len(),
                        kind,
                        detail: Cow::Owned(format!(
                            "{}: 0x{:02X} ({} bytes)",
                            if dif_byte == 0x0F {
                                "Manufacturer Specific"
                            } else {
                                "More Records Follow"
                            },
                            dif_byte,
                            data.len() - offset
                        )),
                        group: Some(record_index),
                        layer: Layer::RecordField,
                    });
                    return;
                }
                _ => {
                    // Other special functions (0x7F etc.)
                    segments.push(ByteSegment {
                        start: base + offset,
                        end: base + offset + 1,
                        kind: SegmentKind::Unknown,
                        detail: Cow::Owned(format!("Special DIF: 0x{:02X}", dif_byte)),
                        group: None,
                        layer: Layer::RecordField,
                    });
                    offset += 1;
                    continue;
                }
            }
        }

        // Regular data record: parse DIF/DIFE/VIF/VIFE/Data

        // Try to parse the data record to get accurate sizes
        let remaining = &data[offset..];
        let record_result = DataRecord::try_from(remaining);

        match record_result {
            Ok(record) => {
                let header_size = record.data_record_header.get_size();
                let total_size = record.get_size();

                // DIF (1 byte)
                let dif = &record
                    .data_record_header
                    .raw_data_record_header
                    .data_information_block;
                segments.push(ByteSegment {
                    start: base + offset,
                    end: base + offset + 1,
                    kind: SegmentKind::Dif,
                    detail: Cow::Owned(format!("DIF: 0x{:02X}", dif_byte)),
                    group: Some(record_index),
                    layer: Layer::RecordField,
                });
                offset += 1;

                // DIFE bytes
                let dife_count = dif.get_size() - 1;
                if dife_count > 0 {
                    segments.push(ByteSegment {
                        start: base + offset,
                        end: base + offset + dife_count,
                        kind: SegmentKind::Dife,
                        detail: Cow::Owned(format!("DIFE: {} byte(s)", dife_count)),
                        group: Some(record_index),
                        layer: Layer::RecordField,
                    });
                    offset += dife_count;
                }

                // VIF/VIFE (if present)
                if let Some(vib) = &record
                    .data_record_header
                    .raw_data_record_header
                    .value_information_block
                {
                    // VIF (1 byte)
                    segments.push(ByteSegment {
                        start: base + offset,
                        end: base + offset + 1,
                        kind: SegmentKind::Vif,
                        detail: Cow::Owned(format!(
                            "VIF: 0x{:02X}",
                            data.get(offset).copied().unwrap_or(0)
                        )),
                        group: Some(record_index),
                        layer: Layer::RecordField,
                    });
                    offset += 1;

                    // VIFE bytes
                    let vife_count = if let Some(ext) = &vib.value_information_extension {
                        ext.len()
                    } else {
                        0
                    };
                    if vife_count > 0 {
                        segments.push(ByteSegment {
                            start: base + offset,
                            end: base + offset + vife_count,
                            kind: SegmentKind::Vife,
                            detail: Cow::Owned(format!("VIFE: {} byte(s)", vife_count)),
                            group: Some(record_index),
                            layer: Layer::RecordField,
                        });
                        offset += vife_count;
                    }

                    // Plaintext VIF
                    if let Some(plaintext) = &vib.plaintext_vife {
                        let pt_size = plaintext.len() + 1; // 1 for length byte
                        segments.push(ByteSegment {
                            start: base + offset,
                            end: base + offset + pt_size,
                            kind: SegmentKind::PlaintextVif,
                            detail: Cow::Owned(format!(
                                "Plaintext VIF: \"{}\"",
                                plaintext.iter().collect::<String>()
                            )),
                            group: Some(record_index),
                            layer: Layer::RecordField,
                        });
                        offset += pt_size;
                    }
                }

                // Data payload
                let data_size = total_size.saturating_sub(header_size);
                if data_size > 0 {
                    let available = data.len().saturating_sub(offset);
                    let emit_size = data_size.min(available);
                    if emit_size > 0 {
                        segments.push(ByteSegment {
                            start: base + offset,
                            end: base + offset + emit_size,
                            kind: SegmentKind::DataPayload,
                            detail: Cow::Owned(format!("{}", record.data)),
                            group: Some(record_index),
                            layer: Layer::RecordField,
                        });
                    }
                    offset += emit_size;
                    if emit_size < data_size {
                        // Truncated frame — stop parsing
                        return;
                    }
                }

                record_index += 1;
            }
            Err(_) => {
                // Parse failure: mark remaining bytes as Unknown
                segments.push(ByteSegment {
                    start: base + offset,
                    end: base + data.len(),
                    kind: SegmentKind::Unknown,
                    detail: Cow::Borrowed("Unparseable data record bytes"),
                    group: None,
                    layer: Layer::RecordField,
                });
                return;
            }
        }
    }
}

// ── Wireless frame annotation ───────────────────────────────────────────────

/// Annotate a wireless Format A frame, mapping parsed fields back to original offsets.
fn annotate_wireless_format_a(
    original: &[u8],
    stripped: &[u8],
) -> Result<Vec<ByteSegment>, MbusError> {
    // Build an offset map: for each byte in the stripped buffer, what's its original offset?
    let offset_map = build_format_a_offset_map(original);

    // First annotate the stripped frame
    let stripped_segments = annotate_wireless_inner(stripped)?;

    let mut segments = Vec::new();

    // Map stripped segments back to original offsets
    for seg in &stripped_segments {
        let orig_start = offset_map.get(seg.start).copied().unwrap_or(seg.start);
        let orig_end = if seg.end <= offset_map.len() {
            // end is exclusive, so we want the offset of the byte *at* seg.end
            // or the end of the last byte in the segment
            if seg.end < offset_map.len() {
                offset_map[seg.end]
            } else {
                // Last byte's original position + 1
                offset_map.last().map(|&o| o + 1).unwrap_or(seg.end)
            }
        } else {
            offset_map.last().map(|&o| o + 1).unwrap_or(seg.end)
        };

        segments.push(ByteSegment {
            start: orig_start,
            end: orig_end,
            kind: seg.kind.clone(),
            detail: seg.detail.clone(),
            group: seg.group,
            layer: seg.layer,
        });
    }

    // Now insert CRC segments at the right positions
    let mut crc_positions = Vec::new();

    // Block 1 CRC: after bytes 0-9 (at original positions 10-11)
    if original.len() >= 12 {
        crc_positions.push((10, 12));
    }

    // Subsequent block CRCs
    let mut pos = 12usize;
    while pos < original.len() {
        let remaining = original.len() - pos;
        if remaining < 3 {
            break;
        }
        let max_data_len = 16.min(remaining - 2);
        let mut found = false;
        for data_len in (1..=max_data_len).rev() {
            let crc_start = pos + data_len;
            if crc_start + 2 > original.len() {
                continue;
            }
            let computed = crc16_en13757(&original[pos..crc_start]);
            let stored = u16::from_be_bytes([original[crc_start], original[crc_start + 1]]);
            if computed == stored {
                crc_positions.push((crc_start, crc_start + 2));
                pos = crc_start + 2;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    // Insert CRC segments, splitting existing segments if needed
    for &(crc_start, crc_end) in &crc_positions {
        // Find the right position to insert
        let crc_seg = ByteSegment {
            start: crc_start,
            end: crc_end,
            kind: SegmentKind::Crc,
            detail: Cow::Owned(format!(
                "CRC: 0x{:02X}{:02X}",
                original.get(crc_start).copied().unwrap_or(0),
                original.get(crc_start + 1).copied().unwrap_or(0),
            )),
            group: None,
            layer: Layer::Frame,
        };

        // Find where to insert based on start position
        let insert_pos = segments
            .iter()
            .position(|s| s.start >= crc_start)
            .unwrap_or(segments.len());

        // Check if we need to split an existing segment
        if insert_pos > 0 {
            let prev = &segments[insert_pos - 1];
            if prev.end > crc_start {
                // Need to split: the previous segment extends past the CRC start
                let orig_end = prev.end;
                let orig_kind = prev.kind.clone();
                let orig_detail = prev.detail.clone();
                let orig_group = prev.group;
                let orig_layer = prev.layer;

                // Truncate previous segment
                segments[insert_pos - 1].end = crc_start;

                // Insert CRC
                segments.insert(insert_pos, crc_seg);

                // Insert remainder if there are bytes after the CRC
                if crc_end < orig_end {
                    segments.insert(
                        insert_pos + 1,
                        ByteSegment {
                            start: crc_end,
                            end: orig_end,
                            kind: orig_kind,
                            detail: orig_detail,
                            group: orig_group,
                            layer: orig_layer,
                        },
                    );
                }
                continue;
            }
        }

        segments.insert(insert_pos, crc_seg);
    }

    // Remove any zero-width segments that may have been created
    segments.retain(|s| s.start < s.end);

    Ok(segments)
}

/// Annotate a wireless frame directly (already stripped or Format B).
fn annotate_wireless_inner(data: &[u8]) -> Result<Vec<ByteSegment>, MbusError> {
    // Validate it parses as wireless
    let _frame = wireless_mbus_link_layer::WirelessFrame::try_from(data)?;

    let mut segments = Vec::new();

    // L-field (byte 0)
    segments.push(ByteSegment {
        start: 0,
        end: 1,
        kind: SegmentKind::LField,
        detail: Cow::Owned(format!("L Field: {}", data.first().copied().unwrap_or(0))),
        group: None,
        layer: Layer::Frame,
    });

    // C-field (byte 1)
    segments.push(ByteSegment {
        start: 1,
        end: 2,
        kind: SegmentKind::CField,
        detail: Cow::Owned(format!(
            "C Field: 0x{:02X}",
            data.get(1).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::Frame,
    });

    // Manufacturer ID (bytes 2-3), ID number (bytes 4-7), Version (byte 8), Device Type (byte 9)
    segments.push(ByteSegment {
        start: 2,
        end: 4,
        kind: SegmentKind::ManufacturerCode,
        detail: Cow::Owned(format!(
            "Manufacturer: 0x{:02X}{:02X}",
            data.get(2).copied().unwrap_or(0),
            data.get(3).copied().unwrap_or(0),
        )),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 4,
        end: 8,
        kind: SegmentKind::IdentificationNumber,
        detail: Cow::Owned(format!(
            "ID: {:02X}{:02X}{:02X}{:02X}",
            data.get(4).copied().unwrap_or(0),
            data.get(5).copied().unwrap_or(0),
            data.get(6).copied().unwrap_or(0),
            data.get(7).copied().unwrap_or(0),
        )),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 8,
        end: 9,
        kind: SegmentKind::Version,
        detail: Cow::Owned(format!("Version: {}", data.get(8).copied().unwrap_or(0))),
        group: None,
        layer: Layer::Frame,
    });
    segments.push(ByteSegment {
        start: 9,
        end: 10,
        kind: SegmentKind::DeviceType,
        detail: Cow::Owned(format!(
            "Device Type: 0x{:02X}",
            data.get(9).copied().unwrap_or(0)
        )),
        group: None,
        layer: Layer::Frame,
    });

    // Application layer starts at byte 10
    if data.len() > 10 {
        let app_data = &data[10..];
        annotate_application_layer(&mut segments, data, 10, app_data);
    }

    Ok(segments)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Build an offset map from stripped buffer position to original buffer position.
/// For Format A: block 1 is bytes 0-9 (same), then CRC at 10-11 is skipped,
/// data blocks of up to 16 bytes with 2-byte CRCs interspersed.
fn build_format_a_offset_map(original: &[u8]) -> Vec<usize> {
    let mut map = Vec::new();

    // Block 1: first 10 bytes map 1:1
    for i in 0..10.min(original.len()) {
        map.push(i);
    }

    if original.len() < 12 {
        return map;
    }

    // Skip block 1 CRC (bytes 10-11)
    let mut pos = 12usize;

    while pos < original.len() {
        let remaining = original.len() - pos;
        if remaining < 3 {
            // Not enough for data + CRC, remaining bytes are data
            for i in 0..remaining {
                map.push(pos + i);
            }
            break;
        }

        let max_data_len = 16.min(remaining - 2);
        let mut found = false;

        for data_len in (1..=max_data_len).rev() {
            let crc_start = pos + data_len;
            if crc_start + 2 > original.len() {
                continue;
            }
            let computed = crc16_en13757(&original[pos..crc_start]);
            let stored = u16::from_be_bytes([original[crc_start], original[crc_start + 1]]);
            if computed == stored {
                // Map data bytes
                for i in 0..data_len {
                    map.push(pos + i);
                }
                // Skip CRC bytes
                pos = crc_start + 2;
                found = true;
                break;
            }
        }

        if !found {
            // No CRC found, copy remaining
            let remaining = original.len() - pos;
            for i in 0..remaining {
                map.push(pos + i);
            }
            break;
        }
    }

    map
}

/// CRC-16/EN-13757 implementation (same as wireless-mbus-link-layer crate).
fn crc16_en13757(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x3D65;
            } else {
                crc <<= 1;
            }
        }
    }
    crc ^ 0xFFFF
}

/// Check if a long TPL header indicates encryption.
fn is_long_tpl_encrypted(app_data: &[u8]) -> bool {
    if app_data.len() < 13 {
        return false;
    }
    // Configuration field is at bytes 11-12 (relative to app_data)
    let config = m_bus_core::ConfigurationField::from_bytes(app_data[11], app_data[12]);
    !matches!(
        config.security_mode(),
        m_bus_core::SecurityMode::NoEncryption
    )
}

/// Check if a short TPL header indicates encryption.
fn is_short_tpl_encrypted(app_data: &[u8], skip_count: usize) -> bool {
    if app_data.len() < skip_count + 4 {
        return false;
    }
    let config = m_bus_core::ConfigurationField::from_bytes(
        app_data[skip_count + 2],
        app_data[skip_count + 3],
    );
    !matches!(
        config.security_mode(),
        m_bus_core::SecurityMode::NoEncryption
    )
}

/// Render annotated segments as a human-readable text visualization.
///
/// Produces a table with hex bytes, segment kinds, and detail labels,
/// grouped by protocol layer and data record index. Suitable for
/// CLI output and WASM text rendering.
pub fn render_annotations(segments: &[ByteSegment], data: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    // Hex dump header
    let _ = writeln!(out, "Hex Dump:");
    let _ = writeln!(
        out,
        "────────────────────────────────────────────────────────────────────"
    );
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        let _ = write!(out, "  {:04X}  ", offset);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                let _ = write!(out, " ");
            }
            let _ = write!(out, "{:02X} ", byte);
        }
        // Pad if short row
        let pad = 16 - chunk.len();
        for _ in 0..pad {
            let _ = write!(out, "   ");
        }
        if chunk.len() <= 8 {
            let _ = write!(out, " ");
        }
        let _ = write!(out, " ");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '·'
            };
            let _ = write!(out, "{}", ch);
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out);

    // Segment table
    let _ = writeln!(out, "{:<12} {:<24} {:<22} Detail", "Offset", "Hex", "Kind");
    let _ = writeln!(
        out,
        "{:<12} {:<24} {:<22} ──────────────────────────",
        "────────────", "────────────────────────", "──────────────────────"
    );

    let mut current_layer = None;
    let mut current_group: Option<usize> = None;

    for seg in segments {
        // Section headers on layer/group transitions
        let layer_changed = current_layer != Some(seg.layer);
        let group_changed = seg.group != current_group;

        if layer_changed {
            let header = match seg.layer {
                Layer::Frame => "Frame",
                Layer::AppHeader => "Application Header",
                Layer::RecordField => "Data Records",
            };
            let _ = writeln!(
                out,
                "┌─ {} ────────────────────────────────────────────────────────",
                header
            );
            current_layer = Some(seg.layer);
            current_group = None;
        }

        if seg.layer == Layer::RecordField && group_changed {
            if let Some(g) = seg.group {
                let _ = writeln!(out, "│  ── Record {} ──", g);
            }
            current_group = seg.group;
        }

        // Offset column
        let offset_str = if seg.end - seg.start == 1 {
            format!("[{:02X}]", seg.start)
        } else {
            format!("[{:02X}..{:02X}]", seg.start, seg.end)
        };

        // Hex bytes column (truncate if too many bytes)
        let max_hex_bytes = 8;
        let byte_count = seg.end - seg.start;
        let hex_str = if seg.start < data.len() {
            let end = seg.end.min(data.len());
            let show = (end - seg.start).min(max_hex_bytes);
            let mut h: String = data[seg.start..seg.start + show]
                .iter()
                .map(|b| format!("{:02X} ", b))
                .collect();
            if byte_count > max_hex_bytes {
                h.push_str("...");
            } else {
                // Remove trailing space
                h.pop();
            }
            h
        } else {
            String::new()
        };

        let kind_str = format!("{}", seg.kind);

        let _ = writeln!(
            out,
            "│  {:<10} {:<24} {:<22} {}",
            offset_str, hex_str, kind_str, seg.detail
        );
    }

    out
}

/// Annotate and render a raw M-Bus frame as human-readable text.
///
/// Convenience wrapper that calls [`annotate_frame`] and then [`render_annotations`].
pub fn annotate_and_render(data: &[u8]) -> Result<String, MbusError> {
    let segments = annotate_frame(data)?;
    Ok(render_annotations(&segments, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid wired long frame from user data bytes.
    /// Computes proper length and checksum fields.
    fn make_long_frame(c_field: u8, address: u8, user_data: &[u8]) -> Vec<u8> {
        let l = (user_data.len() + 2) as u8; // +2 for C and A fields
        let mut frame = vec![0x68, l, l, 0x68, c_field, address];
        frame.extend_from_slice(user_data);
        let checksum: u8 = frame[4..].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        frame.push(checksum);
        frame.push(0x16);
        frame
    }

    /// Verify contiguity: segments sorted by start, each end == next start, covers all bytes.
    fn assert_contiguous(segments: &[ByteSegment], total_len: usize) {
        assert!(!segments.is_empty(), "segments should not be empty");
        assert_eq!(segments[0].start, 0, "first segment should start at 0");
        assert_eq!(
            segments.last().map(|s| s.end).unwrap_or(0),
            total_len,
            "last segment should end at total length"
        );
        for window in segments.windows(2) {
            assert_eq!(
                window[0].end, window[1].start,
                "gap between segments at {}-{}",
                window[0].end, window[1].start
            );
        }
        for seg in segments {
            assert!(seg.start < seg.end, "zero-width segment at {}", seg.start);
        }
    }

    #[test]
    fn test_long_frame() {
        // Example long frame from lib.rs docs
        let data: Vec<u8> = vec![
            0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01,
            0x00, 0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B,
            0x00, 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48,
            0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11,
            0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72,
            0x00, 0x72, 0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
        ];

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        // Check frame layer fields
        assert_eq!(segments[0].kind, SegmentKind::StartByte);
        assert_eq!(segments[0].end, 1);
        assert_eq!(segments[1].kind, SegmentKind::Length);
        assert_eq!(segments[1].start, 1);
        assert_eq!(segments[1].end, 3);
        assert_eq!(segments[2].kind, SegmentKind::StartByte); // repeated start
        assert_eq!(segments[2].start, 3);
        assert_eq!(segments[2].end, 4);
        assert_eq!(segments[3].kind, SegmentKind::CField);
        assert_eq!(segments[4].kind, SegmentKind::AField);

        // Check CI field
        assert_eq!(segments[5].kind, SegmentKind::CiField);
        assert_eq!(segments[5].start, 6);

        // Check last segments are checksum + stop byte
        let last = segments.last().expect("non-empty");
        assert_eq!(last.kind, SegmentKind::StopByte);
        let second_last = &segments[segments.len() - 2];
        assert_eq!(second_last.kind, SegmentKind::Checksum);
    }

    #[test]
    fn test_short_frame() {
        // Short frame: [0x10] [C=0x5B] [A=0x01] [CS=0x5C] [0x16]
        let data: Vec<u8> = vec![0x10, 0x5B, 0x01, 0x5C, 0x16];
        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0].kind, SegmentKind::StartByte);
        assert_eq!(segments[1].kind, SegmentKind::CField);
        assert_eq!(segments[2].kind, SegmentKind::AField);
        assert_eq!(segments[3].kind, SegmentKind::Checksum);
        assert_eq!(segments[4].kind, SegmentKind::StopByte);
    }

    #[test]
    fn test_single_character_frame() {
        let data: Vec<u8> = vec![0xE5];
        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].kind, SegmentKind::StartByte);
    }

    #[test]
    fn test_data_record_offsets() {
        // Minimal long frame with a simple data record: 04 07 00 00 00 00
        // DIF=0x04 (32-bit integer), VIF=0x07 (energy, Wh)
        let user_data: Vec<u8> = vec![
            0x72, // CI = long TPL
            0x01, 0x00, 0x00, 0x00, // ID
            0x96, 0x15, // Manufacturer
            0x01, // Version
            0x00, // Device type
            0x18, // Access number
            0x00, // Status
            0x00, 0x00, // Config
            0x04, 0x07, // DIF=0x04 VIF=0x07
            0x00, 0x00, 0x00, 0x00, // 4 bytes data
        ];
        let data = make_long_frame(0x08, 0x01, &user_data);

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        // Find data record segments
        let dif_seg = segments.iter().find(|s| s.kind == SegmentKind::Dif);
        assert!(dif_seg.is_some());
        let dif_seg = dif_seg.expect("DIF segment");
        assert_eq!(dif_seg.group, Some(0));
        assert_eq!(dif_seg.start, 19); // byte 6 + 13 = 19

        let vif_seg = segments.iter().find(|s| s.kind == SegmentKind::Vif);
        assert!(vif_seg.is_some());
        let vif_seg = vif_seg.expect("VIF segment");
        assert_eq!(vif_seg.group, Some(0));
        assert_eq!(vif_seg.start, 20);

        let data_seg = segments.iter().find(|s| s.kind == SegmentKind::DataPayload);
        assert!(data_seg.is_some());
        let data_seg = data_seg.expect("Data segment");
        assert_eq!(data_seg.group, Some(0));
        assert_eq!(data_seg.start, 21);
        assert_eq!(data_seg.end, 25);
    }

    #[test]
    fn test_encrypted_long_tpl() {
        // Long frame with encrypted config field (non-zero security mode)
        let user_data: Vec<u8> = vec![
            0x72, // CI = long TPL
            0x01, 0x00, 0x00, 0x00, // ID
            0x96, 0x15, // Manufacturer
            0x01, // Version
            0x00, // Device type
            0x18, // Access number
            0x00, // Status
            0x00, 0x05, // Config: security mode 5 (bits 12-8 of MSB = 0x05)
            // Encrypted payload (8 bytes)
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22,
        ];
        let data = make_long_frame(0x08, 0x01, &user_data);

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        // Should have EncryptedPayload instead of DIF/VIF records
        let enc_seg = segments
            .iter()
            .find(|s| s.kind == SegmentKind::EncryptedPayload);
        assert!(enc_seg.is_some(), "should have encrypted payload segment");
        let enc_seg = enc_seg.expect("encrypted payload");
        assert_eq!(enc_seg.start, 19); // 6 + 13
        assert_eq!(enc_seg.end, 27); // 19 + 8

        // Should NOT have any DIF/VIF segments
        assert!(
            !segments.iter().any(|s| s.kind == SegmentKind::Dif),
            "should not parse DIF in encrypted payload"
        );
    }

    #[test]
    fn test_manufacturer_specific_tail() {
        // Long frame with a manufacturer-specific record (DIF=0x0F)
        let user_data: Vec<u8> = vec![
            0x72, // CI
            0x01, 0x00, 0x00, 0x00, // ID
            0x96, 0x15, // Manufacturer
            0x01, // Version
            0x00, // Device type
            0x18, // Access number
            0x00, // Status
            0x00, 0x00, // Config (no encryption)
            // Data: DIF=0x0F (manufacturer specific) + 2 payload bytes
            0x0F, 0x60, 0x00,
        ];
        let data = make_long_frame(0x08, 0x01, &user_data);

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        let mfr_seg = segments
            .iter()
            .find(|s| s.kind == SegmentKind::ManufacturerSpecific);
        assert!(
            mfr_seg.is_some(),
            "should have manufacturer specific segment"
        );
        let mfr_seg = mfr_seg.expect("manufacturer specific");
        // Should consume all remaining data bytes (0x0F, 0x60, 0x00)
        assert_eq!(mfr_seg.start, 19);
        assert_eq!(mfr_seg.end, 22);
    }

    #[test]
    fn test_idle_fillers() {
        // Long frame with idle filler bytes between records
        let user_data: Vec<u8> = vec![
            0x72, // CI
            0x01, 0x00, 0x00, 0x00, // ID
            0x96, 0x15, // Manufacturer
            0x01, // Version
            0x00, // Device type
            0x18, // Access number
            0x00, // Status
            0x00, 0x00, // Config
            // Idle fillers
            0x2F, 0x2F, 0x2F, 0x2F,
        ];
        let data = make_long_frame(0x08, 0x01, &user_data);

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        let filler_count = segments
            .iter()
            .filter(|s| s.kind == SegmentKind::IdleFiller)
            .count();
        assert_eq!(filler_count, 4);
    }

    #[cfg(feature = "plaintext-before-extension")]
    #[test]
    fn test_plaintext_vif() {
        // Use the existing example frame from lib.rs which has plaintext VIF records
        // (0x02 0xFC 0x03 0x48 0x52 0x25 0x74 = DIF VIF VIFE "HR" data)
        let data: Vec<u8> = vec![
            0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01, 0x00, 0x00, 0x00, 0x96, 0x15, 0x01,
            0x00, 0x18, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00, 0x00, 0x01, 0xFD, 0x1B,
            0x00, 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC, 0x03, 0x48,
            0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11,
            0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09, 0x12, 0x65, 0xB7, 0x09, 0x01, 0x72,
            0x00, 0x72, 0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00, 0x1F, 0xB3, 0x16,
        ];

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        let pt_seg = segments
            .iter()
            .find(|s| s.kind == SegmentKind::PlaintextVif);
        assert!(
            pt_seg.is_some(),
            "should have plaintext VIF segment; kinds: {:?}",
            segments.iter().map(|s| &s.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_wireless_parse_directly() {
        // Wireless frame: L=0x19 (25 bytes follow), C=0x44, MFR=0xAE4C (SEN), ID, V=0x68, T=0x07
        let data: Vec<u8> = vec![
            0x19, 0x44, 0xAE, 0x4C, 0x44, 0x55, 0x22, 0x33, 0x68, 0x07, 0x7A, 0x55, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x13, 0x89, 0xE2, 0x01, 0x00, 0x02, 0x3B, 0x00, 0x00,
        ];

        // Verify the wireless parser actually works on this data
        let wf = wireless_mbus_link_layer::WirelessFrame::try_from(data.as_slice());
        assert!(wf.is_ok(), "wireless parse should succeed: {:?}", wf.err());

        // Now test annotation
        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        assert_eq!(segments[0].kind, SegmentKind::LField);
        assert_eq!(segments[1].kind, SegmentKind::CField);
        assert_eq!(segments[2].kind, SegmentKind::ManufacturerCode);
        assert_eq!(segments[3].kind, SegmentKind::IdentificationNumber);
        assert_eq!(segments[4].kind, SegmentKind::Version);
        assert_eq!(segments[5].kind, SegmentKind::DeviceType);
    }

    #[test]
    fn test_wireless_ell_i_application_layer_no_transport_annotation() {
        let data: Vec<u8> = vec![
            0x12, 0x44, 0xAE, 0x0C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x07, 0x8C, 0x20, 0x27, 0x78,
            0x0B, 0x13, 0x43, 0x65, 0x87,
        ];

        let segments = annotate_frame(&data).expect("should parse");
        assert_contiguous(&segments, data.len());

        assert!(segments.iter().any(|s| {
            s.kind == SegmentKind::CiField
                && s.start == 13
                && s.detail.contains("Application Layer, no TPL header")
        }));
        assert!(!segments.iter().any(|s| s.kind == SegmentKind::Unknown));
    }
}
