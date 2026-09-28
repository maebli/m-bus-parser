//! Explicit application-only rendering for the browser's final parse attempt.
use super::*;

/// Render a CI-prefixed application block or bare DIF/VIF records.
///
/// This is explicit so existing frame APIs keep their strict link-layer contract.
/// Recognized CI headers are never reinterpreted as records if malformed. Bare
/// records must parse completely and contain at least one record.
pub fn render_application_hex(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    let data = decode_hex_bytes(input)?;
    let block = match user_data::parse_application_layer(&data) {
        Ok(block) => Some(block),
        Err(m_bus_core::ApplicationLayerError::InvalidControlInformation { byte })
            if data.first() == Some(&byte) =>
        {
            None
        }
        Err(error) => {
            return Err(OutputError::Rendering {
                code: "application.invalid",
                message: error.to_string(),
            })
        }
    };
    let mut buffer = [0u8; 512];
    let security = prepare_security(
        block.as_ref(),
        None,
        options.decode.key.as_ref(),
        &mut buffer,
    )?;
    if security.encrypted && !security.decrypted {
        return Err(OutputError::Decryption {
            code: "security.key_missing",
            message: "application payload is encrypted; supply an AES key to decode its records"
                .to_string(),
        });
    }
    let records = if let Some(decrypted) = security.decrypted_payload.as_deref() {
        decrypted_records(block.as_ref(), decrypted)
    } else if let Some(block) = &block {
        block.data_records()
    } else {
        Some(user_data::parse_data_records(&data))
    };
    let (parsed_records, error) = collect_records(records.as_ref());
    if let Some((offset, message)) = error {
        return Err(OutputError::Rendering {
            code: "application.records_invalid",
            message: format!("failed to parse data record at byte offset {offset}: {message}"),
        });
    }
    if block.is_none() && parsed_records.is_empty() {
        return Err(OutputError::Rendering {
            code: "application.records_empty",
            message: "no application data records found".to_string(),
        });
    }
    // Reset messages have exactly a CI and subcode. Do not silently swallow
    // trailing garbage when using this API for automatic detection.
    if matches!(
        block,
        Some(user_data::UserDataBlock::ResetAtApplicationLevel { .. })
    ) && data.len() != 2
    {
        return Err(OutputError::Rendering {
            code: "application.invalid",
            message: "application reset must contain exactly two bytes".to_string(),
        });
    }
    if matches!(
        block,
        Some(user_data::UserDataBlock::FixedDataStructure { .. })
    ) && data.len() != 17
    {
        return Err(OutputError::Rendering {
            code: "application.invalid",
            message: "fixed application data must contain exactly 17 bytes".to_string(),
        });
    }
    if format == OutputFormat::Xml {
        return Ok(crate::rscada_xml::render_application(
            block.as_ref(),
            records.as_ref(),
        ));
    }
    if matches!(
        format,
        OutputFormat::Annotated | OutputFormat::AnnotatedText
    ) {
        // Keep offsets relative to the actual submitted application payload.
        let mut segments = Vec::new();
        if block.is_some() {
            crate::annotate::annotate_application_layer(&mut segments, &data, 0, &data);
        } else {
            crate::annotate::annotate_data_records(&mut segments, 0, &data);
        }
        if format == OutputFormat::AnnotatedText {
            return Ok(crate::annotate::render_annotations(&segments, &data));
        }
        return serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "protocol": "application",
            "bytes": data,
            "segments": segments,
            "decrypted": false,
        }))
        .map_err(|error| OutputError::Serialization {
            format: "annotated",
            message: error.to_string(),
        });
    }
    let decoded = build_output(
        "application",
        FrameOutput {
            kind: if block.is_some() {
                "application_layer"
            } else {
                "data_records"
            }
            .to_string(),
            function: None,
            address: None,
            control_field: None,
        },
        &data,
        &data,
        false,
        Some(&data),
        block.as_ref(),
        records.as_ref(),
        None,
        None,
        security.clone(),
        options.decode.include_enrichment,
    );
    render_decoded(&decoded, format, options)
}
