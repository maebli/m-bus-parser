#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{decode_hex, DecodeOptions};

const OMS_FRAME: &str = "304493157856341233038C20757A7500112A1001B30A0000C882038E223BBC1091CEB8C0A1009D388A5AF7883A5B5A7FD6";

fn options() -> DecodeOptions {
    DecodeOptions {
        key: Some([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F,
        ]),
        include_enrichment: false,
    }
}

#[test]
fn missing_key_preserves_mode10_metadata_without_records() {
    let decoded = decode_hex(OMS_FRAME, &DecodeOptions::default()).unwrap();

    assert_eq!(decoded.decode_state, "partial");
    assert_eq!(decoded.security.mode_code, Some(10));
    assert_eq!(decoded.security.decryption_state, "key_missing");
    assert!(decoded.security.decrypted_payload_hex.is_none());
    assert!(decoded.records.is_empty());
}

#[cfg(not(feature = "decryption-mode-10"))]
#[test]
fn disabled_mode10_rejects_decryption() {
    let error = decode_hex(OMS_FRAME, &options()).unwrap_err();

    #[cfg(feature = "decryption-api")]
    assert_eq!(error.code(), "security.unsupported_mode");
    #[cfg(not(feature = "decryption-api"))]
    assert_eq!(error.code(), "security.feature_unavailable");
}

#[cfg(feature = "decryption-mode-10")]
mod enabled {
    use super::*;
    use m_bus_parser::decode_bytes;

    const PLAINTEXT: &str = "0C1427048502046D32371F1502FD170000";
    const METER_ADDRESS: [u8; 8] = [0x93, 0x15, 0x78, 0x56, 0x34, 0x12, 0x33, 0x03];

    fn wireless_frame(application: &[u8], address: &[u8; 8]) -> Vec<u8> {
        let length = u8::try_from(application.len() + 9).unwrap();
        let mut frame = vec![length, 0x44];
        frame.extend_from_slice(address);
        frame.extend_from_slice(application);
        frame
    }

    fn wired_frame(application: &[u8]) -> Vec<u8> {
        let length = u8::try_from(application.len() + 2).unwrap();
        let mut frame = vec![0x68, length, length, 0x68, 0x08, 0x01];
        frame.extend_from_slice(application);
        let checksum = frame[4..]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        frame.extend_from_slice(&[checksum, 0x16]);
        frame
    }

    #[test]
    fn oms_annex_n_vector_authenticates_and_decodes_records() {
        let decoded = decode_hex(OMS_FRAME, &options()).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(decoded.security.mode_code, Some(10));
        assert_eq!(decoded.security.decryption_state, "decrypted_authenticated");
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(PLAINTEXT)
        );
        assert_eq!(decoded.records.len(), 3);
        assert!(decoded
            .diagnostics
            .iter()
            .any(|item| item.code == "security.authenticated"));
        assert!(!decoded
            .diagnostics
            .iter()
            .any(|item| item.code == "security.decrypted_unverified"));

        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["meter"]["identity"]["id"], "12345678");
        assert_eq!(value["records"][0]["value"]["value"], "28504.27");
        assert_eq!(value["records"][0]["unit"], "m3");
        assert_eq!(value["records"][2]["value"]["value"], "0");
    }

    #[test]
    fn incorrect_key_and_modified_authenticated_fields_are_rejected() {
        let frame = hex::decode(OMS_FRAME).unwrap();
        let mut wrong_key = options();
        wrong_key.key.as_mut().unwrap()[0] ^= 1;
        let error = decode_bytes(&frame, &wrong_key).unwrap_err();
        assert_eq!(error.code(), "security.authentication_failed");

        for (field, index) in [
            ("meter identity", 4),
            ("control information", 13),
            ("access number", 14),
            ("status", 15),
            ("message counter", 20),
            ("ciphertext", 24),
            ("authentication tag", 48),
        ] {
            let mut altered = frame.clone();
            altered[index] ^= 1;
            let error = decode_bytes(&altered, &options()).unwrap_err();
            assert_eq!(error.code(), "security.authentication_failed", "{field}");
        }
    }

    #[test]
    fn long_transport_address_controls_wireless_and_wired_decryption() {
        let application = hex::decode(concat!(
            "7278563412931533037500112A1001B30A0000",
            "C882038E223BBC1091CEB8C0A1009D388AC0A1B2EEF92EA7D4"
        ))
        .unwrap();
        let relay_address = [0x24, 0x23, 0x56, 0x34, 0x12, 0x90, 0x08, 0x31];

        for frame in [
            wireless_frame(&application, &relay_address),
            wired_frame(&application),
        ] {
            let decoded = decode_bytes(&frame, &options()).unwrap();
            assert_eq!(decoded.decode_state, "complete");
            assert_eq!(decoded.security.decryption_state, "decrypted_authenticated");
            assert_eq!(
                decoded.security.decrypted_payload_hex.as_deref(),
                Some(PLAINTEXT)
            );
            assert_eq!(decoded.records.len(), 3);
            let value = serde_json::to_value(decoded).unwrap();
            assert_eq!(value["meter"]["identity"]["id"], "12345678");
        }
    }

    #[test]
    fn authenticated_application_reports_need_no_cbc_verification_prefix() {
        let short_application = hex::decode("6E7500012A1001B30A0000CC314FBA3971DBB767").unwrap();
        let long_application =
            hex::decode("6F78563412931533037500012A1001B30A0000CC7A4E1AE0B3627DE3").unwrap();

        for frame in [
            wireless_frame(&short_application, &METER_ADDRESS),
            wired_frame(&long_application),
        ] {
            let decoded = decode_bytes(&frame, &options()).unwrap();
            assert_eq!(decoded.decode_state, "complete");
            assert_eq!(decoded.security.decryption_state, "decrypted_authenticated");
            assert_eq!(
                decoded.security.decrypted_payload_hex.as_deref(),
                Some("08")
            );
            assert!(decoded.records.is_empty());
            let application = decoded.application.unwrap();
            assert_eq!(application.protocol, "application_error");
            assert_eq!(application.decoded.unwrap()["error_code"], 8);
        }
    }
}
