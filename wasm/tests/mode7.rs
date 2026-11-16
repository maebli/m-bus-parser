#![cfg(all(target_arch = "wasm32", feature = "decryption-mode-7"))]

use js_sys::Reflect;
use m_bus_parser_wasm_pack::{m_bus_decode, m_bus_render};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn example_value(property: &str) -> &str {
    let html = include_str!("../../docs/index.html");
    let mode7 = html
        .split("7: {")
        .nth(1)
        .unwrap()
        .split("10: {")
        .next()
        .unwrap();
    let prefix = format!("{property}: '");
    mode7
        .split(&prefix)
        .nth(1)
        .unwrap()
        .split('\'')
        .next()
        .unwrap()
}

#[wasm_bindgen_test]
fn site_examples_authenticate_and_render_in_every_format() {
    for (protocol, count) in [("wired", 2), ("wireless", 3)] {
        let input = example_value(protocol);
        let key = example_value("key");
        let decoded = m_bus_decode(input, Some(key.to_string()), None).unwrap();
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(decoded).unwrap();
        assert_eq!(parsed["protocol"], protocol);
        assert_eq!(
            parsed["security"]["decryption_state"],
            "decrypted_authenticated"
        );
        assert_eq!(parsed["security"]["key_id"], 0);
        assert_eq!(parsed["security"]["message_counter"], 2739);
        assert_eq!(parsed["records"].as_array().unwrap().len(), count);
        assert_eq!(parsed["records"][0]["value"]["value"], "28504.27");
        for format in [
            "json",
            "yaml",
            "table",
            "csv",
            "mermaid",
            "xml",
            "annotated",
        ] {
            let output =
                m_bus_render(input, format, Some(key.to_string()), Some(100), None).unwrap();
            assert!(!output.is_empty());
        }
    }
}

#[wasm_bindgen_test]
fn site_examples_require_the_correct_master_key() {
    for protocol in ["wired", "wireless"] {
        let input = example_value(protocol);
        let error = m_bus_decode(input, Some("0".repeat(32)), None).unwrap_err();
        assert_eq!(
            Reflect::get(&error, &JsValue::from_str("code"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("security.authentication_failed")
        );
        let inspected = m_bus_decode(input, None, None).unwrap();
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(inspected).unwrap();
        assert_eq!(parsed["security"]["decryption_state"], "key_missing");
        assert!(parsed["records"].as_array().unwrap().is_empty());
    }
}
