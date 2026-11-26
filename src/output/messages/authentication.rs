use super::*;
use serde_json::{Map, Value};

impl DecodedOutput {
    pub fn authenticate_application(&mut self, key: &[u8; 16]) -> Result<(), OutputError> {
        self.verify_application_key(key).map(|_| ())
    }

    pub(in crate::output) fn verify_application_key(
        &mut self,
        key: &[u8; 16],
    ) -> Result<bool, OutputError> {
        let Some(application) = self.application.as_ref() else {
            return Ok(false);
        };
        if application.protocol != "security_information_transfer" || application.decoded.is_none()
        {
            return Ok(false);
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
        let downlink = matches!(application.control_information.as_str(), "C3" | "C6");
        let meter = self.application_meter(application, downlink);
        let blocks = m_bus_application_layer::sitp::SitpBlocks::new(payload)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| OutputError::Decryption {
                code: "security.application_mac_unavailable",
                message: err.to_string(),
            })?;
        if !blocks
            .iter()
            .any(|b| matches!(b.data_selector, 0x32 | 0x36))
        {
            return Ok(false);
        }
        let fields = self
            .application
            .as_mut()
            .and_then(|a| a.decoded.as_mut())
            .ok_or(OutputError::EmptyInput)?;
        let fields = match fields {
            Value::Object(obj) if obj.contains_key("blocks") => obj
                .get_mut("blocks")
                .and_then(Value::as_array_mut)
                .map(Vec::as_mut_slice)
                .ok_or(OutputError::EmptyInput)?,
            field => core::slice::from_mut(field),
        };
        if fields.len() != blocks.len() {
            return Err(OutputError::Decryption {
                code: "security.application_mac_unavailable",
                message: "SITP block count mismatch.".into(),
            });
        }
        for (field, block) in fields.iter_mut().zip(&blocks) {
            if block.data_selector == 0x36 {
                if let Some(obj) = field.as_object_mut() {
                    obj.retain(|k, _| {
                        matches!(
                            k.as_str(),
                            "key_id"
                                | "key_version"
                                | "block_id"
                                | "block_control"
                                | "recipient"
                                | "data_selector"
                                | "structure_length"
                                | "protected_data_hex"
                        )
                    });
                }
            }
            if block.data_selector == 0x32 {
                field["application_mac_verified"] = false.into();
            }
        }
        let mut updates = Vec::new();
        for (field, block) in fields.iter_mut().zip(&blocks) {
            let verification = match block.data_selector {
                0x32 => verify_sitp(block.body, key).map(|()| None),
                0x36 => unprotect(block.body, key, meter, downlink).map(Some),
                _ => continue,
            };
            match verification {
                Ok(details) => updates.push((field, details)),
                Err(error) => {
                    if error.code() == "security.application_authentication_failed" {
                        field["application_mac_verified"] = false.into();
                    }
                    return Err(error);
                }
            }
        }
        for (field, details) in updates {
            if let Some((values, warning)) = details {
                if let Some(obj) = field.as_object_mut() {
                    obj.extend(values);
                }
                if let Some(warning) = warning {
                    if !self
                        .diagnostics
                        .iter()
                        .any(|d| d.code == warning.code && d.message == warning.message)
                    {
                        self.diagnostics.push(warning);
                    }
                }
            }
            field["application_mac_verified"] = true.into();
        }
        Ok(true)
    }
}

type AuthenticatedFields = (Map<String, Value>, Option<Diagnostic>);

#[cfg(feature = "decryption-mode-10")]
fn unprotect(
    body: &[u8],
    key: &[u8; 16],
    meter: Option<[u8; 8]>,
    downlink: bool,
) -> Result<AuthenticatedFields, OutputError> {
    let meter = meter.ok_or_else(|| OutputError::Decryption {
        code: "security.application_address_unavailable",
        message: "ASP20 needs the meter address from the transport or link header.".into(),
    })?;
    let mut clear = body.to_vec();
    m_bus_core::decryption::decrypt_asp20(&mut clear, key, &meter, downlink).map_err(|error| {
        OutputError::Decryption {
            code: "security.application_authentication_failed",
            message: error.to_string(),
        }
    })?;
    Ok(authenticated_application_fields(&clear, true))
}

#[cfg(not(feature = "decryption-mode-10"))]
fn unprotect(
    _: &[u8],
    _: &[u8; 16],
    _: Option<[u8; 8]>,
    _: bool,
) -> Result<AuthenticatedFields, OutputError> {
    Err(OutputError::Decryption {
        code: "security.application_mac_unavailable",
        message: "ASP20 support requires AES-CCM.".into(),
    })
}
