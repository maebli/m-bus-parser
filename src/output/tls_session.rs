use super::{decode_hex_bytes, OutputError};
use m_bus_core::decryption::mode13::{TlsCipherSuite, TlsDirection, TlsSession};

pub struct ImportedTlsSession {
    cipher_suite: TlsCipherSuite,
    direction: TlsDirection,
    key: zeroize::Zeroizing<Vec<u8>>,
    fixed_iv: [u8; 4],
    sequence_number: u64,
}

fn invalid_session(message: &str) -> OutputError {
    OutputError::InvalidOption {
        option: "tls_session",
        message: message.to_string(),
    }
}

impl ImportedTlsSession {
    pub fn from_json(json: &str) -> Result<Self, OutputError> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|_| invalid_session("session must be a JSON object"))?;
        let field = |name: &str| -> Result<&str, OutputError> {
            value
                .get(name)
                .and_then(serde_json::Value::as_str)
                .filter(|text| !text.is_empty())
                .ok_or_else(|| {
                    invalid_session(&format!("missing or invalid {name}; provide a string"))
                })
        };
        let cipher_suite = match field("cipher_suite")? {
            "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256" => TlsCipherSuite::EcdheEcdsaAes128GcmSha256,
            "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384" => TlsCipherSuite::EcdheEcdsaAes256GcmSha384,
            _ => {
                return Err(invalid_session(
                    "unsupported cipher_suite; select ECDHE-ECDSA AES-128-GCM or AES-256-GCM",
                ))
            }
        };
        let direction = match field("direction")? {
            "client_to_server" => TlsDirection::ClientToServer,
            "server_to_client" => TlsDirection::ServerToClient,
            _ => {
                return Err(invalid_session(
                    "direction must be client_to_server or server_to_client",
                ))
            }
        };
        let key = zeroize::Zeroizing::new(
            decode_hex_bytes(field("key_hex")?)
                .map_err(|_| invalid_session("key_hex must contain complete hexadecimal bytes"))?,
        );
        if key.len() != cipher_suite.key_len() {
            return Err(invalid_session(
                "key_hex length does not match the cipher suite",
            ));
        }
        let fixed_iv = decode_hex_bytes(field("fixed_iv_hex")?)
            .map_err(|_| invalid_session("fixed_iv_hex must contain four hexadecimal bytes"))?
            .try_into()
            .map_err(|_| invalid_session("fixed_iv_hex must contain four hexadecimal bytes"))?;
        let sequence = field("sequence_number")?;
        if !sequence.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid_session("sequence_number must be a decimal string"));
        }
        let sequence_number = sequence
            .parse::<u64>()
            .map_err(|_| invalid_session("sequence_number must fit an unsigned 64-bit counter"))?;
        Ok(Self {
            cipher_suite,
            direction,
            key,
            fixed_iv,
            sequence_number,
        })
    }

    pub fn session(&self) -> TlsSession<'_> {
        TlsSession {
            cipher_suite: self.cipher_suite,
            direction: self.direction,
            key: &self.key,
            fixed_iv: self.fixed_iv,
            sequence_number: self.sequence_number,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn input() -> serde_json::Value {
        serde_json::json!({
            "cipher_suite": "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
            "direction": "client_to_server",
            "key_hex": "000102030405060708090a0b0c0d0e0f",
            "fixed_iv_hex": "01020304",
            "sequence_number": "18446744073709551615"
        })
    }

    #[test]
    fn counters_preserve_all_64_bits() {
        let parsed = ImportedTlsSession::from_json(&input().to_string()).unwrap();
        assert_eq!(parsed.sequence_number, u64::MAX);
    }

    #[test]
    fn invalid_fields_are_rejected_without_echoing_secrets() {
        for field in [
            "cipher_suite",
            "direction",
            "key_hex",
            "fixed_iv_hex",
            "sequence_number",
        ] {
            let mut value = input();
            value[field] = "secret-invalid-value".into();
            let error = ImportedTlsSession::from_json(&value.to_string())
                .err()
                .unwrap();
            assert!(!error.to_string().contains("secret-invalid-value"));
            value.as_object_mut().unwrap().remove(field);
            let error = ImportedTlsSession::from_json(&value.to_string())
                .err()
                .unwrap();
            assert!(error.to_string().contains(field));
        }
    }
}
