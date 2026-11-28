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
mod authentication;

pub(super) fn application_output(
    block: Option<&UserDataBlock<'_>>,
    security: &SecurityContext,
    protocol_data: &mut Option<Box<ProtocolData>>,
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
    let payload = security
        .decrypted_payload
        .as_deref()
        .unwrap_or(message.payload);
    let mut parsed = ProtocolData::default();
    let decoded: Option<Result<serde_json::Value, &'static str>> = match message.protocol {
        #[cfg(feature = "sml")]
        ApplicationProtocol::Sml => super::sml::decode(payload, &mut parsed),
        ApplicationProtocol::DlmsCosem => {
            let mode = security.mode.map(|mode| mode.to_bits());
            let padded = security.decrypted_payload.is_some() && matches!(mode, Some(5 | 7));
            let bytes = if padded && mode == Some(5) {
                payload.strip_prefix(&[0x2f, 0x2f]).unwrap_or(payload)
            } else {
                payload
            };
            parsed.evidence.association =
                m_bus_oms_rules::telegram::DlmsAssociation::from_mbus_payload(bytes);
            #[cfg(feature = "dlms-cosem")]
            {
                super::dlms::decode(bytes, padded, &mut parsed)
            }
            #[cfg(not(feature = "dlms-cosem"))]
            {
                None
            }
        }
        _ => None,
    };
    if decoded.is_some()
        || parsed.evidence.association.is_some()
        || parsed.evidence.dlms_service.is_some()
    {
        *protocol_data = Some(Box::new(parsed));
    }
    if let Some(result) = decoded {
        return match result {
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
                    let (mut value, warning) =
                        security_transfer_fields(*data_selector, body, false);
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
                ApplicationPayload::SecurityTransferBlocks(raw) => {
                    let mut values = Vec::new();
                    for block in m_bus_application_layer::sitp::SitpBlocks::new(raw).flatten() {
                        let (mut fields, warning) =
                            security_transfer_fields(block.data_selector, block.body, false);
                        diagnostic = diagnostic.or(warning);
                        for (name, value) in [
                            ("block_id", block.block_id),
                            ("block_control", block.block_control),
                            ("recipient", block.recipient),
                            ("data_selector", block.data_selector),
                            ("key_id", block.key_id),
                            ("key_version", block.key_version),
                        ] {
                            fields.insert(name.into(), value.into());
                        }
                        values.push(fields);
                    }
                    Some(serde_json::json!({"blocks":values}))
                }
                ApplicationPayload::External(data)
                    if message.protocol == ApplicationProtocol::ImageTransfer =>
                {
                    Some(serde_json::json!({"image_transfer": firmware::decode(data)}))
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
    if selector == 0x32 && body.len() >= 18 {
        return authenticated_application_fields(body, mac_verified);
    }
    if selector == 0x23 && body.len().is_multiple_of(6) && body.len() <= 256 * 6 {
        let counters:Vec<_> = body.chunks_exact(6).filter_map(|entry| {
            let [key_id,key_version,a,b,c,d] = entry else {return None};
            Some(serde_json::json!({"key_id":key_id,"key_version":key_version,"key_counter":u32::from_le_bytes([*a,*b,*c,*d])}))
        }).collect();
        fields.insert("key_counters".into(), counters.into());
        return (fields, None);
    }
    if matches!(selector, 0x00..=0x03 | 0x10..=0x13 | 0x20..=0x21 | 0x30..=0x31 | 0x33..=0x37) {
        fields.insert("structure_length".into(), body.len().into());
        if matches!(selector, 1..=3 | 0x20 | 0x30..=0x31 | 0x33..=0x37) {
            fields.insert("protected_data_hex".into(), hex_string(body).into());
        }
        return (fields, None);
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

fn authenticated_application_fields(
    body: &[u8],
    mac_verified: bool,
) -> (
    serde_json::Map<String, serde_json::Value>,
    Option<Diagnostic>,
) {
    let mut fields = serde_json::Map::new();
    if let Some((counter, timing, protocol, data, mac)) = (|| {
        let (counter, rest) = body.split_first_chunk::<4>()?;
        let (timing, rest) = rest.split_first_chunk::<5>()?;
        let (protocol, rest) = rest.split_first()?;
        let (data, mac) = rest.split_last_chunk::<8>()?;
        Some((counter, timing, protocol, data, mac))
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
        if *protocol == 6 {
            fields.insert("image_transfer".into(), firmware::decode(data));
            return (fields, None);
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
    (fields, None)
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

impl DecodedOutput {
    pub(super) fn application_meter(
        &self,
        app: &ApplicationOutput,
        downlink: bool,
    ) -> Option<[u8; 8]> {
        if matches!(app.control_information.as_str(), "C0" | "C2" | "C3" | "C5") {
            let prefix = self
                .raw
                .payload_hex
                .as_ref()?
                .strip_suffix(&app.payload_hex)?;
            let header = prefix.get(prefix.len().checked_sub(26)?..)?;
            if header.get(..2)? != app.control_information {
                return None;
            }
            let bytes = decode_hex_bytes(header.get(2..18)?).ok()?;
            let [a, b, c, d, m, n, v, t] = *bytes.as_slice() else {
                return None;
            };
            return Some([m, n, a, b, c, d, v, t]);
        }
        if let Some(mac) = self.lpwan.as_ref() {
            let llc = mac.llc.as_ref()?;
            return if downlink { llc.receiver } else { llc.sender };
        }
        if self.frame.kind == "reassembled" {
            return super::fragments::application_meter(&self.raw.fragment_frames_hex, downlink);
        }
        let exchange = super::oms::exchange_frame(self)?;
        if downlink {
            exchange.receiver
        } else {
            Some(exchange.sender)
        }
    }
}
