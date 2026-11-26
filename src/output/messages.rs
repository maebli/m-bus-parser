use super::*;
use user_data::{ApplicationPayload, ApplicationProtocol, UserDataBlock};

#[cfg(feature = "decryption-mode-7")]
fn verify_sitp(body: &[u8], key: &[u8; 16]) -> Result<(), OutputError> {
    m_bus_core::decryption::verify_sitp_cmac(body, key).map_err(|error| OutputError::Decryption {
        code: "security.application_authentication_failed",
        message: error.to_string(),
    })
}

#[cfg(feature = "decryption-mode-7")]
impl DecodedOutput {
    pub fn authenticate_application(&mut self, key: &[u8; 16]) -> Result<(), OutputError> {
        let Some(application) = self.application.as_mut() else {
            return Ok(());
        };
        let Some(fields) = application
            .decoded
            .as_mut()
            .and_then(serde_json::Value::as_object_mut)
        else {
            return Ok(());
        };
        if application.protocol != "security_information_transfer"
            || fields
                .get("data_selector")
                .and_then(serde_json::Value::as_u64)
                != Some(0x32)
        {
            return Ok(());
        }
        let raw = decode_hex_bytes(
            self.security
                .decrypted_payload_hex
                .as_deref()
                .unwrap_or(&application.payload_hex),
        )?;
        let payload = if self.security.mode_code == Some(5)
            && self.security.decrypted_payload_hex.is_some()
        {
            raw.strip_prefix(&[0x2f, 0x2f]).unwrap_or(&raw)
        } else {
            &raw
        };
        let Ok(ApplicationPayload::SecurityTransfer {
            data_selector: 0x32,
            body,
            ..
        }) = user_data::application_message::parse_sitp(payload)
        else {
            return Err(OutputError::Decryption {
                code: "security.application_mac_unavailable",
                message: "The decoded SITP command does not contain a complete ASP10 container."
                    .into(),
            });
        };
        let result = verify_sitp(body, key);
        fields.insert("application_mac_verified".into(), result.is_ok().into());
        result
    }
}

pub(super) fn verify_application(
    block: Option<&UserDataBlock<'_>>,
    security: &SecurityContext,
    key: &[u8; 16],
) -> Result<bool, OutputError> {
    #[cfg(feature = "decryption-mode-7")]
    if let Some(UserDataBlock::ApplicationMessage(message)) = block {
        let payload = match security.decrypted_payload.as_deref() {
            Some(bytes) => message.decode_payload(bytes),
            None => message.decode(),
        };
        if let Ok(ApplicationPayload::SecurityTransfer {
            data_selector: 0x32,
            body,
            ..
        }) = payload
        {
            verify_sitp(body, key)?;
            return Ok(true);
        }
    }
    #[cfg(not(feature = "decryption-mode-7"))]
    let _ = (block, security, key);
    Err(OutputError::Decryption {
        code: "security.application_mac_unavailable",
        message: "Application authentication requires a decoded SITP AES-CMAC container".into(),
    })
}

pub(super) fn application_output(
    block: Option<&UserDataBlock<'_>>,
    security: &SecurityContext,
) -> (Option<ApplicationOutput>, Option<Diagnostic>) {
    let Some(UserDataBlock::ApplicationMessage(message)) = block else {
        return (None, None);
    };
    let mut output = ApplicationOutput {
        control_information: format!("{:02X}", message.control_information),
        protocol: message.protocol.name().to_string(),
        payload_hex: hex_string(message.payload),
        decoded: None,
    };
    if security.encrypted && !security.decrypted {
        return (Some(output), None);
    }
    #[cfg(feature = "oms-gen3")]
    if message.protocol == ApplicationProtocol::Sml {
        let payload = security
            .decrypted_payload
            .as_deref()
            .unwrap_or(message.payload);
        if let Some(0x76) = payload.iter().find(|byte| **byte != 0x2f) {
            return match super::sml::decode(payload) {
                Ok(value) => {
                    output.decoded = Some(value);
                    (Some(output), None)
                }
                Err(reason) => (
                    Some(output),
                    Some(application_diagnostic(
                        "application.payload_invalid",
                        reason.into(),
                    )),
                ),
            };
        }
    }
    let result = if let Some(plaintext) = &security.decrypted_payload {
        message.decode_payload(plaintext)
    } else {
        message.decode()
    };
    let mut diagnostic = None;
    match result {
        Ok(payload) => {
            if let ApplicationPayload::Clock { operation, .. } = &payload {
                if !matches!(
                    operation,
                    user_data::application_message::ClockOperation::Set
                ) {
                    output.protocol = ApplicationProtocol::ClockAdjust.name().into();
                }
            }
            output.decoded = match &payload {
                ApplicationPayload::SecurityTransfer {
                    block_id,
                    block_control,
                    recipient,
                    data_selector,
                    key_id,
                    key_version,
                    body,
                } => {
                    let (mut value, warning) = security_transfer_fields(
                        *data_selector,
                        body,
                        security.application_mac_verified,
                    );
                    diagnostic = warning;
                    for (name, field) in [
                        ("block_id", *block_id),
                        ("block_control", *block_control),
                        ("recipient", *recipient),
                        ("data_selector", *data_selector),
                        ("key_id", *key_id),
                        ("key_version", *key_version),
                    ] {
                        value.insert(name.into(), field.into());
                    }
                    Some(value.into())
                }
                ApplicationPayload::External(_) => None,
                ApplicationPayload::Error { code, details } => Some(serde_json::json!({
                    "error_code": code,
                    "description": application_error_description(*code),
                    "details_hex": hex_string(details),
                })),
                ApplicationPayload::Format(format) => Some(serde_json::json!({
                    "signature": format.signature,
                    "length": format.length,
                    "records": format.records().flatten().collect::<Vec<_>>(),
                })),
                _ => serde_json::to_value(&payload).ok(),
            };
            if message.protocol.has_external_payload() && !message.payload.is_empty() {
                diagnostic = Some(application_diagnostic(
                    "application.external_protocol",
                    "M-Bus header decoded; payload bytes belong to an external or manufacturer-defined protocol".to_string(),
                ));
            } else if message.protocol == ApplicationProtocol::Compact {
                diagnostic = Some(application_diagnostic(
                    "application.format_required",
                    "compact data requires a matching format frame to verify and reconstruct its records".to_string(),
                ));
            }
        }
        Err(error) => {
            diagnostic = Some(application_diagnostic(
                "application.payload_invalid",
                error.to_string(),
            ))
        }
    }
    (Some(output), diagnostic)
}

fn security_transfer_fields(
    selector: u8,
    body: &[u8],
    mac_verified: bool,
) -> (
    serde_json::Map<String, serde_json::Value>,
    Option<Diagnostic>,
) {
    let mut fields = serde_json::Map::new();
    if let (0x22, [status]) = (selector, body) {
        fields.insert("status_code".into(), (*status).into());
        return (fields, None);
    }
    if let Some((counter, timing, protocol, data, mac)) = (|| {
        let (counter, rest) = body.split_first_chunk::<4>()?;
        let (timing, rest) = rest.split_first_chunk::<5>()?;
        let (protocol, rest) = rest.split_first()?;
        let (data, mac) = rest.split_last_chunk::<8>()?;
        (selector == 0x32).then_some((counter, timing, protocol, data, mac))
    })() {
        fields.insert("key_counter".into(), u32::from_le_bytes(*counter).into());
        fields.insert("target_time_hex".into(), hex_string(timing).into());
        fields.insert("protocol_id".into(), (*protocol).into());
        fields.insert("apdu_hex".into(), hex_string(data).into());
        fields.insert("application_mac_hex".into(), hex_string(mac).into());
        fields.insert("application_mac_verified".into(), mac_verified.into());
        if *protocol == 1 {
            let records = user_data::parse_data_records(data);
            let (records, error) = collect_records(Some(&records), None, None);
            fields.insert("records".into(), serde_json::json!(records));
            return (
                fields,
                error.map(|(_, reason)| {
                    application_diagnostic("application.payload_invalid", reason)
                }),
            );
        }
        if *protocol == 4 {
            let diagnostic = match user_data::application_message::parse_sitp_clock(data) {
                Ok((operation, value)) => {
                    fields.insert(
                        "clock".into(),
                        serde_json::json!({ "operation": operation, "value": value }),
                    );
                    None
                }
                Err(error) => Some(application_diagnostic(
                    "application.payload_invalid",
                    error.to_string(),
                )),
            };
            return (fields, diagnostic);
        }
        return (
            fields,
            Some(application_diagnostic(
                "application.sitp_protocol_unsupported",
                format!("Unsupported SITP protocol ID {protocol:#04x}"),
            )),
        );
    }
    fields.insert("body_hex".into(), hex_string(body).into());
    (
        fields,
        Some(application_diagnostic(
            "application.sitp_body_unsupported",
            "Unsupported SITP data selector or block length".into(),
        )),
    )
}

fn application_diagnostic(code: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity: "warning".to_string(),
        code: code.to_string(),
        layer: "application".to_string(),
        message,
        offset_start: None,
        offset_end: None,
    }
}

fn application_error_description(code: u8) -> &'static str {
    match code {
        0 => "Unspecified application error",
        1 => "Unsupported control information",
        2 => "Buffer too long, truncated",
        3 => "Too many records",
        4 => "Premature end of record",
        5 => "Too many data information extensions",
        6 => "Too many value information extensions",
        8 => "Application busy",
        9 => "Too many readouts",
        0x10 => "Command not yet finished",
        0x11 => "Function not implemented",
        0x12 => "Data not available",
        0x15 => "Missing or invalid parameter",
        _ => "Reserved or manufacturer-defined application error",
    }
}
