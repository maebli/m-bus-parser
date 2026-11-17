use super::*;
use user_data::{ApplicationPayload, ApplicationProtocol, UserDataBlock};

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
    let result = if let Some(plaintext) = &security.decrypted_payload {
        message.decode_payload(plaintext)
    } else {
        message.decode()
    };
    let mut diagnostic = None;
    match result {
        Ok(payload) => {
            output.decoded = match &payload {
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
