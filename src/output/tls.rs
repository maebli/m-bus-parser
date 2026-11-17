use super::*;
use m_bus_core::decryption::mode13::{Mode13Error, TlsContentType, TlsDirection};

pub fn decode_hex_with_tls_session(
    input: &str,
    options: &DecodeOptions,
    session: &TlsSession<'_>,
) -> Result<DecodedOutput, OutputError> {
    let bytes = decode_hex_bytes(input)?;
    let mut decoded = decode_bytes_with_session(&bytes, options, Some(session))?;
    decoded.raw.input = Some(input.to_string());
    Ok(decoded)
}

pub fn render_hex_with_tls_session(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
    session: &TlsSession<'_>,
) -> Result<String, OutputError> {
    require_tls_output_format(format)?;
    let decoded = decode_hex_with_tls_session(input, &options.decode, session)?;
    render_decoded(&decoded, format, options)
}

pub(super) fn require_tls_output_format(format: OutputFormat) -> Result<(), OutputError> {
    if matches!(
        format,
        OutputFormat::Xml | OutputFormat::Annotated | OutputFormat::AnnotatedText
    ) {
        return Err(OutputError::Rendering {
            code: "tls.unsupported_output",
            message: "TLS session decoding supports JSON, YAML, Table, CSV, and Diagram output"
                .to_string(),
        });
    }
    Ok(())
}

fn tls_error(error: Mode13Error) -> OutputError {
    OutputError::Decryption {
        code: match error {
            Mode13Error::AuthenticationFailed => "security.authentication_failed",
            Mode13Error::IncompleteRecord => "security.tls_incomplete_record",
            Mode13Error::InvalidConfiguration => "security.unsupported_configuration",
            Mode13Error::InvalidKeyLength => "security.invalid_key",
            Mode13Error::NotApplicationData => "security.tls_session_management_required",
            _ => "security.tls_invalid_record",
        },
        message: error.to_string(),
    }
}

pub(super) fn prepare_mode13(
    user_data: Option<&user_data::UserDataBlock<'_>>,
    session: Option<&TlsSession<'_>>,
) -> Result<SecurityContext, OutputError> {
    let block = user_data.ok_or_else(|| tls_error(Mode13Error::InvalidConfiguration))?;
    let payload = block
        .mode13_payload()
        .map_err(tls_error)?
        .ok_or_else(|| tls_error(Mode13Error::InvalidConfiguration))?;
    let tls = TlsOutput {
        protocol: if payload.is_channel_request() {
            "channel_request"
        } else {
            "TLS1.2"
        }
        .to_string(),
        records: payload
            .records()
            .map(|record| TlsRecordOutput {
                content_type: match record.content_type {
                    TlsContentType::ChannelRequest => "channel_request",
                    TlsContentType::ChangeCipherSpec => "change_cipher_spec",
                    TlsContentType::Alert => "alert",
                    TlsContentType::Handshake => "handshake",
                    TlsContentType::ApplicationData => "application_data",
                }
                .to_string(),
                length: record.payload.len(),
            })
            .collect(),
    };
    let mut context = SecurityContext {
        encrypted: true,
        mode: Some(SecurityMode::SpecificUsage13),
        ell_encrypted: false,
        decrypted: false,
        key_supplied: session.is_some(),
        decrypted_payload: None,
        tls: Some(tls),
    };
    let Some(session) = session else {
        return Ok(context);
    };
    let (header, _) = block
        .transport_payload()
        .ok_or_else(|| tls_error(Mode13Error::InvalidConfiguration))?;
    let expected_direction = match header
        .direction()
        .map_err(|_| tls_error(Mode13Error::InvalidConfiguration))?
    {
        user_data::Direction::SlaveToMaster => TlsDirection::ClientToServer,
        user_data::Direction::MasterToSlave => TlsDirection::ServerToClient,
        _ => return Err(tls_error(Mode13Error::InvalidConfiguration)),
    };
    if session.direction != expected_direction {
        return Err(OutputError::Decryption {
            code: "security.tls_direction_mismatch",
            message: "TLS direction does not match the M-Bus transport header; OMS meters are TLS clients".to_string(),
        });
    }
    let mut plaintext = vec![0; payload.encoded_records().len()];
    let length = payload
        .decrypt_into(session, &mut plaintext)
        .map_err(tls_error)?;
    plaintext.truncate(length);
    context.decrypted = true;
    context.decrypted_payload = Some(plaintext);
    Ok(context)
}
