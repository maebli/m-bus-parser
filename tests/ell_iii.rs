#![cfg(feature = "std")]
#![allow(clippy::unwrap_used)]

use m_bus_parser::{decode_bytes, render_bytes, DecodeOptions, OutputFormat, RenderOptions};
use serde_json::Value;

fn wireless_frame(application: &[u8]) -> Vec<u8> {
    let mut frame = vec![
        0, 0x44, 0xAE, 0x0C, 1, 0, 0, 0, 1, 7, 0x8E, 0x20, 0x27, 0x2D, 0x2C, 0x78, 0x56, 0x34,
        0x12, 1, 7,
    ];
    frame.extend_from_slice(application);
    let length = u8::try_from(frame.len() - 1).unwrap();
    *frame.first_mut().unwrap() = length;
    frame
}

#[test]
fn ell_iii_wireless_records_keep_header_and_annotation_boundaries() {
    for (application, payload_start) in [
        (&[0x78, 0x01, 0x13, 42][..], 24),
        (&[0x7A, 0x99, 8, 0, 0, 0x01, 0x13, 42][..], 28),
    ] {
        let bytes = wireless_frame(application);
        let decoded = decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
        assert_eq!(decoded.decode_state, "complete");
        let value = serde_json::to_value(decoded).unwrap();
        assert_eq!(value["security"]["payload_protection"], "cleartext");
        assert_eq!(value["records"][0]["value"]["value"], "0.042");
        assert_eq!(value["records"].as_array().unwrap().len(), 1);
        assert_eq!(value["transport"]["ell"]["access_number"], 0x27);
        assert_eq!(value["transport"]["ell"]["receiver_manufacturer"], "2C2D");
        assert_eq!(
            value["transport"]["ell"]["receiver_address"],
            "785634120107"
        );
        assert!(value["transport"]["ell"]["session_number"].is_null());
        assert!(value["transport"]["ell"]["payload_crc"].is_null());
        if application.first() == Some(&0x7A) {
            assert_eq!(value["transport"]["access_numbers"][0]["value"], 0x99);
            assert_eq!(value["transport"]["statuses"][0]["raw"], "08");
        } else {
            assert!(value["transport"]["configuration"].is_null());
        }

        let output =
            render_bytes(&bytes, OutputFormat::Annotated, &RenderOptions::default()).unwrap();
        let annotated: Value = serde_json::from_str(&output).unwrap();
        let segments = annotated["segments"].as_array().unwrap();
        let ell = segments
            .iter()
            .find(|segment| segment["kind"] == "extended_link_layer")
            .unwrap();
        assert_eq!(ell["start"], 11);
        assert_eq!(ell["end"], 21);
        assert!(segments
            .iter()
            .any(|segment| segment["kind"] == "ci_field" && segment["start"] == 21));
        let payload = segments
            .iter()
            .find(|segment| segment["kind"] == "data_payload")
            .unwrap();
        assert_eq!(payload["start"], payload_start);
        assert_eq!(payload["end"], bytes.len());
        assert!(!segments
            .iter()
            .any(|segment| segment["kind"] == "unknown" || segment["kind"] == "encrypted_payload"));
    }
}

#[test]
fn ell_iii_preserves_encryption_declared_by_the_inner_transport_header() {
    let mut application = vec![0x7A, 0x99, 0, 0x10, 0x05];
    application.extend_from_slice(&[0; 16]);
    let decoded = decode_bytes(&wireless_frame(&application), &DecodeOptions::default()).unwrap();
    assert_eq!(decoded.decode_state, "partial");
    assert!(decoded.records.is_empty());
    let value = serde_json::to_value(decoded).unwrap();
    assert_eq!(value["security"]["mode_code"], 5);
    assert_eq!(value["security"]["decryption_state"], "key_missing");
    assert!(value["transport"]["ell"]["session_number"].is_null());
    assert!(value["transport"]["ell"]["payload_crc"].is_null());
}
