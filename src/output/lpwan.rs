use super::*;
use m_bus_oms_rules::lpwan::LpwanHeader;

pub(super) fn decode(
    frame: &[u8],
    options: &DecodeOptions,
    session: Option<&TlsSession<'_>>,
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
    )?;
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

#[cfg(feature = "decryption-mode-10")]
impl DecodedOutput {
    pub fn authenticate_lpwan_mac(
        &mut self,
        key: &[u8; 16],
        declared_counter: Option<u8>,
    ) -> Result<(), OutputError> {
        let Some(mac) = self.lpwan.as_mut() else {
            return Ok(());
        };
        let down = mac.downlink();
        let Some(body) = mac.body.as_mut() else {
            return Ok(());
        };
        let Some(tag) = body.authentication_code else {
            return Ok(());
        };
        body.authenticated = false;
        body.blocks.clear();
        let meter = mac
            .llc
            .as_ref()
            .and_then(|llc| if down { llc.receiver } else { llc.sender })
            .ok_or_else(|| OutputError::Decryption {
                code: "security.lpwan_address_required",
                message: "MAC authentication needs the end-device link address.".into(),
            })?;
        let counter = body.derivation_counter.or(declared_counter).ok_or_else(|| OutputError::Decryption { code: "security.lpwan_counter_required", message: "MAC authentication needs the session derivation counter when the frame omits it.".into() })?;
        let usage = match mac.header & 15 {
            0 => 0,
            1 => 2,
            12 | 13 => 3,
            _ => {
                return Err(OutputError::Decryption {
                    code: "security.lpwan_configuration",
                    message: "Unsupported secured MAC frame type.".into(),
                })
            }
        };
        if mac.extension != 0 || body.extension & !1 != 0 {
            return Err(OutputError::Decryption {
                code: "security.lpwan_configuration",
                message: "Unsupported MAC security profile or body extension.".into(),
            });
        }
        let mut controls = vec![body.control];
        if body.control & 0x80 != 0 {
            controls.push(body.extension);
        }
        if let Some(counter) = body.derivation_counter {
            controls.push(counter);
        }
        let mut clear = body.encrypted_blocks.clone();
        let security = m_bus_core::decryption::LpwanMacSecurity {
            meter,
            derivation_counter: counter,
            message_counter: body
                .message_counter
                .ok_or_else(|| OutputError::Decryption {
                    code: "security.lpwan_counter_required",
                    message: "The secured MAC body is missing the message counter.".into(),
                })?,
            usage,
        };
        security
            .authenticate_blocks(key, &controls, &tag, &mut clear)
            .map_err(|_| OutputError::Decryption {
                code: "security.lpwan_authentication_failed",
                message: "LPWAN MAC authentication failed.".into(),
            })?;
        body.blocks = m_bus_oms_rules::lpwan::MacBody::parse_blocks(&clear).map_err(|message| {
            OutputError::InvalidFrame {
                wired: "LPWAN MAC frame".into(),
                wireless: message.into(),
            }
        })?;
        body.authenticated = true;
        Ok(())
    }
}
