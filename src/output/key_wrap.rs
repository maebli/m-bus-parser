use super::{decode_hex_bytes, OmsReport};
use m_bus_oms_rules::key_wrap::{is_wrapped_structure, Evidence};
use serde_json::Value;

fn selector(block: &Value) -> Option<(u8, u8, u8)> {
    let selector = block.get("data_selector")?.as_u64()?.try_into().ok()?;
    if !is_wrapped_structure(selector) {
        return None;
    }
    Some((
        selector,
        block.get("block_control")?.as_u64()?.try_into().ok()?,
        block.get("key_id")?.as_u64()?.try_into().ok()?,
    ))
}

fn clear_structure(data: &[u8]) -> Option<&[u8]> {
    let (header, padded) = data.split_first_chunk::<8>()?;
    let [0xa6, 0x59, 0x59, 0xa6, a, b, c, d] = *header else {
        return None;
    };
    let len = u32::from_be_bytes([a, b, c, d]) as usize;
    let (plain, padding) = padded.split_at_checked(len)?;
    (len > 0
        && padded.len().is_multiple_of(8)
        && padding.len() < 8
        && padding.iter().all(|b| *b == 0))
    .then_some(plain)
}

fn split_counter(selector: u8, body: &[u8]) -> (Option<&[u8; 4]>, &[u8]) {
    if selector == 0x30 {
        if let Some((count, rest)) = body.split_first_chunk::<4>() {
            return (Some(count), rest);
        }
    }
    (None, body)
}

pub(super) fn assess(block: &Value, report: &mut OmsReport) {
    let Some((selector, control, key)) = selector(block) else {
        return;
    };
    let evidence = if key == 0xff {
        let data = block
            .get("protected_data_hex")
            .and_then(Value::as_str)
            .and_then(|h| decode_hex_bytes(h).ok());
        data.as_deref().map(|data| {
            let (counter, body) = split_counter(selector, data);
            clear_structure(body)
                .map(|plain| Evidence::from_plaintext(selector, control, key, plain, counter))
                .unwrap_or_default()
        })
    } else {
        block
            .get("key_wrap_checks")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    };
    Evidence::assess(evidence.as_ref(), selector, key, report);
}

#[cfg(feature = "decryption-mode-7")]
impl super::DecodedOutput {
    pub fn verify_key_wrap(&mut self, wrapping_key: &[u8; 16]) -> Result<(), super::OutputError> {
        use aes_kw::{KeyInit, KwpAes128};
        let Some(decoded) = self
            .application
            .as_mut()
            .filter(|app| app.protocol == "security_information_transfer")
            .and_then(|a| a.decoded.as_mut())
        else {
            return Ok(());
        };
        let verify = |block: &mut Value| -> Result<(), super::OutputError> {
            let Some((selector, control, key)) = selector(block) else {
                return Ok(());
            };
            if key == 0xff {
                return Ok(());
            }
            let Some(hex) = block.get("protected_data_hex").and_then(Value::as_str) else {
                return Ok(());
            };
            let data = decode_hex_bytes(hex)?;
            let (counter, ciphertext) = split_counter(selector, &data);
            let mut buffer = zeroize::Zeroizing::new(vec![0u8; ciphertext.len().saturating_sub(8)]);
            let result = if ciphertext.len() >= 16 {
                KwpAes128::new(wrapping_key.into())
                    .unwrap_key(ciphertext, &mut buffer)
                    .ok()
                    .map(|plain| Evidence::from_plaintext(selector, control, key, plain, counter))
            } else {
                None
            };
            block["key_wrap_checks"] = serde_json::json!(result.unwrap_or_default());
            Ok(())
        };
        if let Some(blocks) = decoded.get_mut("blocks").and_then(Value::as_array_mut) {
            blocks.iter_mut().try_for_each(verify)
        } else {
            verify(decoded)
        }
    }
}
