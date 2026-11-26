use super::*;

fn direction(frame: &DecodedOutput) -> Option<TlsDirection> {
    let bytes = decode_hex_bytes(frame.raw.payload_hex.as_deref()?).ok()?;
    let block = user_data::parse_application_layer(&bytes).ok()?;
    let direction = block.transport_payload()?.0.direction().ok()?;
    match direction {
        user_data::Direction::SlaveToMaster => Some(TlsDirection::ClientToServer),
        user_data::Direction::MasterToSlave => Some(TlsDirection::ServerToClient),
        _ => None,
    }
}

fn is_peer(a: &DecodedOutput, b: &DecodedOutput) -> bool {
    match (&a.meter.identity, &b.meter.identity) {
        (Some(a), Some(b)) => {
            a.id.is_some()
                && a.id == b.id
                && a.manufacturer_code == b.manufacturer_code
                && a.version == b.version
                && a.device_type_code == b.device_type_code
        }
        (None, None) => {
            a.frame.address.is_some()
                && a.frame.address == b.frame.address
                && a.protocol == b.protocol
        }
        _ => false,
    }
}

fn apply(
    frame: &DecodedOutput,
    options: &DecodeOptions,
    session: &TlsSession<'_>,
) -> Result<DecodedOutput, OutputError> {
    if frame.frame.kind == "reassembled" {
        let application = decode_hex_bytes(&frame.raw.original_frame_hex)?;
        let block = user_data::parse_application_layer(&application)
            .map_err(|_| tls_error(Mode13Error::InvalidConfiguration))?;
        let context = prepare_mode13(Some(&block), Some(session))?;
        let records = context
            .decrypted_payload
            .as_deref()
            .and_then(|data| decrypted_records(Some(&block), data));
        let mut output = build_output(
            &frame.protocol,
            frame.frame.clone(),
            &application,
            &application,
            false,
            Some(&application),
            Some(&block),
            records.as_ref(),
            None,
            None,
            context.clone(),
            options.include_enrichment,
        );
        output.raw = frame.raw.clone();
        output.meter = frame.meter.clone();
        return Ok(output);
    }
    let mut result = decode_hex_with_tls_session(&frame.raw.original_frame_hex, options, session)?;
    result
        .raw
        .fragment_frames_hex
        .clone_from(&frame.raw.fragment_frames_hex);
    Ok(result)
}

pub fn render_capture_hex_with_tls_session(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
    capture_options: OmsCaptureOptions,
    session: &TlsSession<'_>,
) -> Result<String, OutputError> {
    require_tls_output_format(format)?;
    match decode_capture_hex_with_tls_session(input, &options.decode, session)? {
        FragmentOutput::Single(frame) => match format {
            OutputFormat::Oms(format) => render_oms_capture_with_options(
                std::slice::from_ref(frame.as_ref()),
                format,
                None,
                capture_options,
            ),
            _ => render_decoded(&frame, format, options),
        },
        FragmentOutput::Multiple(frames) => match format {
            OutputFormat::Oms(format) => {
                render_oms_capture_with_options(&frames, format, None, capture_options)
            }
            _ => render_capture(FragmentOutput::Multiple(frames), format, options),
        },
    }
}

pub fn decode_capture_hex_with_tls_session(
    input: &str,
    options: &DecodeOptions,
    session: &TlsSession<'_>,
) -> Result<FragmentOutput, OutputError> {
    let mut frames = match decode_capture_hex(input, options)? {
        FragmentOutput::Single(frame) => {
            return apply(&frame, options, session).map(|f| FragmentOutput::Single(Box::new(f)))
        }
        FragmentOutput::Multiple(frames) => frames,
    };
    let candidates: Vec<_> = frames
        .iter_mut()
        .filter_map(|frame| {
            let tls = frame.security.tls.as_ref()?;
            let count = tls.records.len();
            (direction(frame) == Some(session.direction)
                && tls
                    .records
                    .iter()
                    .any(|record| record.content_type == "application_data"))
            .then_some((frame, count))
        })
        .collect();
    let Some((first, _)) = candidates.first() else {
        return Err(OutputError::Decryption {
            code: "security.tls_transport_required",
            message:
                "The capture contains no TLS application data in the supplied session direction."
                    .into(),
        });
    };
    if candidates
        .iter()
        .skip(1)
        .any(|(frame, _)| !is_peer(first, frame))
    {
        return Err(OutputError::Decryption { code: "security.tls_ambiguous_peer", message: "The capture contains TLS application data for multiple or unidentified peers. Supply a capture for one TLS session.".into() });
    }
    let mut sequence = Some(session.sequence_number);
    for (frame, records) in candidates {
        let number = sequence.ok_or_else(|| tls_error(Mode13Error::SequenceNumberOverflow))?;
        let next = TlsSession {
            sequence_number: number,
            ..*session
        };
        *frame = apply(frame, options, &next)?;
        sequence = number.checked_add(records as u64);
    }
    Ok(FragmentOutput::Multiple(frames))
}
