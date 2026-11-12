#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{decode_hex, DecodeOptions};

mod fixtures {
    #![allow(dead_code)]
    include!("../crates/m-bus-core/tests/fixtures/mode13.rs");
}

fn frame() -> String {
    format!(
        "2f4493157856341233037a0100ff0d01{}",
        fixtures::OPENSSL_AES128_RECORD
    )
}

#[cfg(not(feature = "decryption-mode-13"))]
#[test]
fn disabled_mode13_does_not_decode_records() {
    let decoded = decode_hex(&frame(), &DecodeOptions::default()).unwrap();
    assert_eq!(decoded.security.mode_code, Some(13));
    assert!(decoded.records.is_empty());
    assert!(decoded.security.tls.is_none());
}

#[cfg(feature = "decryption-mode-13")]
mod enabled {
    use super::*;
    use m_bus_parser::{decryption::mode13::*, output::*, OutputFormat, RenderOptions};

    const KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    fn session() -> TlsSession<'static> {
        TlsSession {
            cipher_suite: TlsCipherSuite::EcdheEcdsaAes128GcmSha256,
            direction: TlsDirection::ClientToServer,
            key: &KEY,
            fixed_iv: [1, 2, 3, 4],
            sequence_number: 5,
        }
    }

    #[test]
    fn missing_session_preserves_tls_record_metadata() {
        let decoded = decode_hex(&frame(), &DecodeOptions::default()).unwrap();
        assert_eq!(decoded.security.decryption_state, "session_required");
        let tls = decoded.security.tls.unwrap();
        assert_eq!(tls.records[0].content_type, "application_data");
        assert_eq!(tls.records[0].length, 27);
        assert!(decoded.records.is_empty());
    }

    #[test]
    fn imported_session_authenticates_and_parses_records_without_exposing_keys() {
        let decoded =
            decode_hex_with_tls_session(&frame(), &DecodeOptions::default(), &session()).unwrap();
        assert_eq!(decoded.security.decryption_state, "decrypted_authenticated");
        assert_eq!(decoded.records.len(), 1);
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some("01132A")
        );
        let json = serde_json::to_string(&decoded).unwrap();
        assert!(!json.contains("000102030405060708090a0b0c0d0e0f"));
        assert!(!json.contains("key_hex"));
        for format in [
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Csv,
            OutputFormat::Table,
            OutputFormat::Mermaid,
        ] {
            assert!(!render_hex_with_tls_session(
                &frame(),
                format,
                &RenderOptions::default(),
                &session()
            )
            .unwrap()
            .is_empty());
        }
        assert_eq!(
            render_hex_with_tls_session(
                &frame(),
                OutputFormat::Xml,
                &RenderOptions::default(),
                &session()
            )
            .unwrap_err()
            .code(),
            "tls.unsupported_output"
        );
    }

    #[test]
    fn wired_long_headers_decrypt_the_same_tls_record() {
        let application = hex::decode(format!(
            "7278563412931533030100ff0d01{}",
            fixtures::OPENSSL_AES128_RECORD
        ))
        .unwrap();
        let length = u8::try_from(application.len() + 2).unwrap();
        let mut frame = vec![0x68, length, length, 0x68, 0x08, 1];
        frame.extend(application);
        let checksum = frame[4..]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        frame.extend([checksum, 0x16]);
        let decoded =
            decode_hex_with_tls_session(&hex::encode(frame), &DecodeOptions::default(), &session())
                .unwrap();
        assert_eq!(decoded.records.len(), 1);
        assert_eq!(decoded.security.decryption_state, "decrypted_authenticated");
    }

    #[test]
    fn a_session_cannot_silently_decode_a_different_security_mode() {
        let clear = "114493157856341233037a0100000001132a";
        assert_eq!(
            decode_hex_with_tls_session(clear, &DecodeOptions::default(), &session())
                .unwrap_err()
                .code(),
            "security.tls_transport_required"
        );
    }

    #[test]
    fn wrong_direction_and_counter_are_rejected() {
        let mut wrong = session();
        wrong.direction = TlsDirection::ServerToClient;
        assert_eq!(
            decode_hex_with_tls_session(&frame(), &DecodeOptions::default(), &wrong)
                .unwrap_err()
                .code(),
            "security.tls_direction_mismatch"
        );
        wrong = session();
        wrong.sequence_number += 1;
        assert_eq!(
            decode_hex_with_tls_session(&frame(), &DecodeOptions::default(), &wrong)
                .unwrap_err()
                .code(),
            "security.authentication_failed"
        );
    }

    #[test]
    fn application_only_input_uses_the_same_session_and_missing_session_metadata() {
        let application = format!("7a0100ff0d01{}", fixtures::OPENSSL_AES128_RECORD);
        let output = render_application_hex_with_tls_session(
            &application,
            OutputFormat::Json,
            &RenderOptions::default(),
            &session(),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["records"].as_array().unwrap().len(), 1);
        let output =
            render_application_hex(&application, OutputFormat::Json, &RenderOptions::default())
                .unwrap();
        assert!(output.contains("session_required"));
    }
}
