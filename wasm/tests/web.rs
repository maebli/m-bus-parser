//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1 + 1, 2);
}

#[wasm_bindgen_test]
fn test_m_bus_parse_table() {
    let input = "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16";
    let output = m_bus_parser_wasm_pack::m_bus_parse(input, "table_format");
    assert!(output.contains("Long Frame"));
    assert!(output.contains("RspUd (ACD: false, DFC: false)"));
    assert!(output.contains("02205100"));
    assert!(output.contains("[Wh]"));
}

#[wasm_bindgen_test]
fn test_m_bus_parse_hexview_ci_78_annotations() {
    let input = "1444AE0C7856341201078C2027780B134365877AC5";
    let output = m_bus_parser_wasm_pack::m_bus_parse(input, "hexview");
    let segments: serde_json::Value =
        serde_json::from_str(&output).expect("hexview should return annotated JSON");
    let segments = segments.as_array().expect("hexview should return an array");

    assert!(segments.iter().any(|seg| {
        seg.get("kind").and_then(|v| v.as_str()) == Some("CiField")
            && seg
                .get("detail")
                .and_then(|v| v.as_str())
                .is_some_and(|detail| detail.contains("0x78"))
    }));
    assert!(segments.iter().any(|seg| {
        seg.get("kind").and_then(|v| v.as_str()) == Some("DataPayload")
            && seg.get("detail").and_then(|v| v.as_str()) == Some("876543")
    }));
    assert!(!segments.iter().any(|seg| {
        seg.get("kind").and_then(|v| v.as_str()) == Some("Unknown")
            || seg
                .get("detail")
                .and_then(|v| v.as_str())
                .is_some_and(|detail| detail.contains("Unparseable data record bytes"))
    }));
}

#[wasm_bindgen_test]
fn test_m_bus_parse_hexview_with_key_displays_decrypted_payload() {
    let input = "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A";
    let key = "0102030405060708090A0B0C0D0E0F11";
    let output = m_bus_parser_wasm_pack::m_bus_parse_with_key(input, "hexview", key);
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("decrypted hexview should return JSON");

    assert_eq!(
        parsed.get("decrypted").and_then(|v| v.as_bool()),
        Some(true)
    );
    let bytes = parsed
        .get("bytes")
        .and_then(|v| v.as_array())
        .expect("decrypted hexview should include display bytes");
    let display_hex = bytes
        .iter()
        .map(|byte| format!("{:02X}", byte.as_u64().unwrap_or_default()))
        .collect::<String>();
    assert!(display_hex.contains("0C1427048502046D32371F1502FD170000"));

    let segments = parsed
        .get("segments")
        .and_then(|v| v.as_array())
        .expect("decrypted hexview should include annotation segments");
    assert!(segments.iter().any(|seg| {
        seg.get("kind").and_then(|v| v.as_str()) == Some("DataPayload")
            && seg
                .get("detail")
                .and_then(|v| v.as_str())
                .is_some_and(|detail| detail.contains("2850427"))
    }));
    assert!(!segments
        .iter()
        .any(|seg| seg.get("kind").and_then(|v| v.as_str()) == Some("EncryptedPayload")));
}

#[wasm_bindgen_test]
fn test_m_bus_parse_with_key_preserves_utf8_text() {
    let input = "2E44931578563412330333637A2A00202557FB8016CA78E1243700B52E981E1918233AFE5E826DD0D4AD7854C697E7C8EB";
    let key = "0102030405060708090A0B0C0D0E0F11";
    let output = m_bus_parser_wasm_pack::m_bus_parse_with_key(input, "json", key);

    assert!(output.contains("\"Text\": \"m³\""), "{output}");
    assert!(!output.contains("mÂ³"), "{output}");
}
