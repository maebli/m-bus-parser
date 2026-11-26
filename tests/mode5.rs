#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::annotate::{annotate_frame, SegmentKind};
use m_bus_parser::{decode_hex, DecodeOptions};

// OMS Vol. 2 Annex N, N.6.3: heat cost allocator behind a radio adapter.
// Link layer, ELL and long TPL header up to the configuration field.
const HEADER: &str = "4493444433221155378C00757288776655934455080004";
// The one encrypted block of N.6.3 and the same block decrypted with KEY.
const ENCRYPTED_BLOCK: &str = "00DFE2A782146D1513581CD2F83F3904";
const DECRYPTED_BLOCK: &str = "2F2F0B6E341200426CFE044B6E563402";
// Plaintext record after the encrypted block (customer location 12345678).
const PLAINTEXT_TAIL: &str = "0CFD1078563412";
const KEY: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
];

/// Builds the N.6.3 frame with the given configuration field and payload.
fn frame(configuration: &str, payload: &[&str]) -> String {
    let body = format!("{HEADER}{configuration}{}", payload.concat());
    format!("{:02X}{body}", body.len() / 2)
}

fn options() -> DecodeOptions {
    DecodeOptions {
        key: Some(KEY),
        include_enrichment: false,
    }
}

#[test]
fn zero_encrypted_blocks_decode_without_a_key() {
    // Configuration field 00 05: mode 5 with N = 0, nothing is encrypted.
    let frame = frame("0005", &[DECRYPTED_BLOCK, PLAINTEXT_TAIL]);
    for options in [DecodeOptions::default(), options()] {
        let decoded = decode_hex(&frame, &options).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(decoded.records.len(), 4);
        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["records"][3]["value"]["value"], "12345678");
    }

    // The hex view annotates the records instead of an encrypted payload.
    let segments = annotate_frame(&hex::decode(&frame).unwrap()).unwrap();
    assert!(segments.iter().any(|s| matches!(s.kind, SegmentKind::Dif)));
    assert!(!segments
        .iter()
        .any(|s| matches!(s.kind, SegmentKind::EncryptedPayload)));
}

#[cfg(not(feature = "decryption-mode-5"))]
#[test]
fn disabled_mode5_rejects_decryption() {
    let frame = frame("1005", &[ENCRYPTED_BLOCK, PLAINTEXT_TAIL]);
    let error = decode_hex(&frame, &options()).unwrap_err();

    #[cfg(feature = "decryption-api")]
    assert_eq!(error.code(), "security.unsupported_mode");
    #[cfg(not(feature = "decryption-api"))]
    assert_eq!(error.code(), "security.feature_unavailable");
}

#[cfg(feature = "decryption-mode-5")]
mod enabled {
    use super::*;

    #[test]
    fn oms_annex_n_vector_decodes_encrypted_and_plaintext_records() {
        // Configuration field 10 05: mode 5 with N = 1.
        let decoded = decode_hex(
            &frame("1005", &[ENCRYPTED_BLOCK, PLAINTEXT_TAIL]),
            &options(),
        )
        .unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(decoded.security.mode_code, Some(5));
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(format!("{DECRYPTED_BLOCK}{PLAINTEXT_TAIL}").as_str())
        );
        assert_eq!(decoded.records.len(), 4);

        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["meter"]["identity"]["id"], "55667788");
        assert_eq!(value["records"][0]["value"]["value"], "1234");
        assert_eq!(value["records"][2]["value"]["value"], "23456");
        assert_eq!(value["records"][3]["value"]["value"], "12345678");
    }

    #[test]
    fn plaintext_after_the_encrypted_blocks_is_not_decrypted() {
        // A second plaintext record plus filler makes the tail a full block.
        let second_record = "0CFD10214365872F2F";
        let decoded = decode_hex(
            &frame("1005", &[ENCRYPTED_BLOCK, PLAINTEXT_TAIL, second_record]),
            &options(),
        )
        .unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(format!("{DECRYPTED_BLOCK}{PLAINTEXT_TAIL}{second_record}").as_str())
        );

        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["records"][3]["value"]["value"], "12345678");
        assert_eq!(value["records"][4]["value"]["value"], "87654321");
    }

    #[test]
    fn block_count_15_decrypts_the_whole_payload() {
        // Configuration field F0 05: N = 15, no plaintext follows.
        let decoded = decode_hex(&frame("F005", &[ENCRYPTED_BLOCK]), &options()).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(DECRYPTED_BLOCK)
        );
        assert_eq!(decoded.records.len(), 3);
    }

    #[test]
    fn encrypted_block_count_beyond_the_payload_is_rejected() {
        // N = 2 claims 32 encrypted bytes, but only 23 follow the header.
        let frame = frame("2005", &[ENCRYPTED_BLOCK, PLAINTEXT_TAIL]);
        let error = decode_hex(&frame, &options()).unwrap_err();
        assert_eq!(error.code(), "security.decryption_failed");
    }
}
