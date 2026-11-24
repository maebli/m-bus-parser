#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{decode_hex, DecodeOptions};

// OMS Vol. 2 Annex N, N.6.3: heat cost allocator behind a radio adapter. The
// configuration field 10 05 marks one encrypted block (N = 1); the last
// record (0C FD 10 ...) is sent in plaintext.
const OMS_FRAME: &str = "304493444433221155378C00757288776655934455080004100500DFE2A782146D1513581CD2F83F39040CFD1078563412";
const OMS_PLAINTEXT: &str = "2F2F0B6E341200426CFE044B6E5634020CFD1078563412";

fn options() -> DecodeOptions {
    DecodeOptions {
        key: Some([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F,
        ]),
        include_enrichment: false,
    }
}

#[cfg(not(feature = "decryption-mode-5"))]
#[test]
fn disabled_mode5_rejects_decryption() {
    let error = decode_hex(OMS_FRAME, &options()).unwrap_err();

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
        let decoded = decode_hex(OMS_FRAME, &options()).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(decoded.security.mode_code, Some(5));
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(OMS_PLAINTEXT)
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
        // Same frame with a full 16-byte plaintext block after the encrypted one.
        let tail = "0CFD10785634120CFD10214365872F2F";
        let frame = format!("39{}{}", &OMS_FRAME[2..OMS_FRAME.len() - 14], tail);
        let decoded = decode_hex(&frame, &options()).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(format!("{}{}", &OMS_PLAINTEXT[..32], tail).as_str())
        );

        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["records"][3]["value"]["value"], "12345678");
        assert_eq!(value["records"][4]["value"]["value"], "87654321");
    }

    #[test]
    fn zero_encrypted_blocks_pass_the_payload_through() {
        // Configuration field 00 05: mode 5 with N = 0, so the whole payload
        // is plaintext.
        let frame = format!("{}0005{}", &OMS_FRAME[..48], OMS_PLAINTEXT);
        let decoded = decode_hex(&frame, &options()).unwrap();

        assert_eq!(decoded.decode_state, "complete");
        assert_eq!(
            decoded.security.decrypted_payload_hex.as_deref(),
            Some(OMS_PLAINTEXT)
        );
        assert_eq!(decoded.records.len(), 4);
    }

    #[test]
    fn encrypted_block_count_beyond_the_payload_is_rejected() {
        // N = 2 claims 32 encrypted bytes, but only 23 follow the header.
        let frame = OMS_FRAME.replacen("04100500", "04200500", 1);
        let error = decode_hex(&frame, &options()).unwrap_err();
        assert_eq!(error.code(), "security.decryption_failed");
    }
}
