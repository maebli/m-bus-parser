use super::*;
use m_bus_oms_rules::lpwan::LpwanHeader;

pub(super) fn decode(
    frame: &[u8],
    options: &DecodeOptions,
    session: Option<&TlsSession<'_>>,
    application_key: Option<&[u8; 16]>,
) -> Result<DecodedOutput, OutputError> {
    let (header, remaining) =
        LpwanHeader::parse(frame).map_err(|message| OutputError::InvalidFrame {
            wired: "LPWAN MAC frame".into(),
            wireless: message.into(),
        })?;
    let control = header.llc.as_ref().and_then(|llc| llc.control);
    let meter = header.llc.as_ref().and_then(|llc| {
        if header.downlink() {
            llc.receiver
        } else {
            llc.sender
        }
    });
    let wireless_id = meter.and_then(manufacturer_id);
    let payload = header
        .llc
        .as_ref()
        .filter(|llc| llc.flags & 0x10 != 0)
        .map(|_| remaining);
    let parsed = payload.map(user_data::parse_application_layer).transpose();
    let (block, error) = match parsed {
        Ok(block) => (block, None),
        Err(error) => (None, Some(error)),
    };
    let mut buffer = vec![0; remaining.len()];
    let security = prepare_security(
        block.as_ref(),
        wireless_id.as_ref(),
        options.key.as_ref(),
        session,
        &mut buffer,
    )?
    .verify_application(block.as_ref(), application_key)?;
    let records = if let Some(plain) = security.decrypted_payload.as_deref() {
        decrypted_records(block.as_ref(), plain)
    } else if security.encrypted {
        None
    } else {
        block.as_ref().and_then(|block| block.data_records())
    };
    let mut output = build_output(
        "oms_lpwan",
        FrameOutput {
            kind: "lpwan".into(),
            function: control
                .and_then(|c| Function::try_from(c).ok())
                .map(|f| frame_function_name(&f)),
            address: None,
            control_field: control.map(|c| format!("{c:02X}")),
        },
        frame,
        frame,
        false,
        payload,
        block.as_ref(),
        records.as_ref(),
        error.as_ref(),
        wireless_id.as_ref(),
        security.clone(),
        options.include_enrichment,
    );
    output.lpwan = Some(header);
    Ok(output)
}

pub(super) fn manufacturer_id(bytes: [u8; 8]) -> Option<wireless::ManufacturerId> {
    let mut address = [0; 9];
    address[..8].copy_from_slice(&bytes);
    wireless::ManufacturerId::try_from(address.as_slice()).ok()
}
