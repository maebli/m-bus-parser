#![cfg(feature = "std")]

use m_bus_parser::{
    decode_hex, output::render_application_hex, DecodeOptions, OutputFormat, RenderOptions,
};

const RECORDS: &str = "03 13 15 31 00";
const CI_RECORDS: &str = "78 03 13 15 31 00";
const LONG_HEADER: &str = "72 00 51 20 02 82 4D 02 04 00 00 00 00 03 13 15 31 00";

#[test]
fn application_only_inputs_render_in_every_website_format() {
    for input in [RECORDS, CI_RECORDS, LONG_HEADER] {
        assert!(decode_hex(input, &DecodeOptions::default()).is_err());
        for format in [
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Table,
            OutputFormat::Csv,
            OutputFormat::Mermaid,
            OutputFormat::Xml,
            OutputFormat::Annotated,
            OutputFormat::AnnotatedText,
        ] {
            let output = render_application_hex(input, format, &RenderOptions::default()).unwrap();
            assert!(!output.is_empty(), "{input} / {format}");
        }
        let output =
            render_application_hex(input, OutputFormat::Json, &RenderOptions::default()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["protocol"], "application");
        assert_eq!(parsed["decode_state"], "complete");
        assert_eq!(parsed["records"][0]["value"]["value"], "12.565");
        assert_eq!(parsed["records"].as_array().unwrap().len(), 1);
        if input == LONG_HEADER {
            assert_eq!(parsed["meter"]["identity"]["id"], "02205100");
        } else {
            assert!(parsed["meter"]["identity"].is_null());
        }
    }
}

#[test]
fn annotations_keep_offsets_relative_to_the_submitted_payload() {
    for (input, length, payload_start) in
        [(RECORDS, 5, 2), (CI_RECORDS, 6, 3), (LONG_HEADER, 18, 15)]
    {
        let output =
            render_application_hex(input, OutputFormat::Annotated, &RenderOptions::default())
                .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["bytes"].as_array().unwrap().len(), length);
        let segments = parsed["segments"].as_array().unwrap();
        let payload = segments
            .iter()
            .find(|segment| segment["kind"] == "data_payload")
            .unwrap();
        assert_eq!(payload["start"], payload_start);
        assert_eq!(payload["end"], length);
        assert!(!segments.iter().any(|segment| segment["kind"] == "unknown"));
    }
}

#[test]
fn invalid_partial_and_empty_records_do_not_look_like_successful_fallbacks() {
    for input in [
        "",
        "1",
        "zz",
        "03 13 15",
        "78 03 13 15",
        "72 00",
        "2F 2F",
        "03 13 15 31 00 03 13",
        "50 00 FF",
        "73 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF",
    ] {
        for format in [
            OutputFormat::Json,
            OutputFormat::Xml,
            OutputFormat::Annotated,
        ] {
            assert!(
                render_application_hex(input, format, &RenderOptions::default()).is_err(),
                "{input} / {format}"
            );
        }
    }
}

#[test]
fn encrypted_application_data_does_not_silently_report_empty_success() {
    let input =
        "72 00 51 20 02 82 4D 02 04 00 00 10 05 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    let error =
        render_application_hex(input, OutputFormat::Json, &RenderOptions::default()).unwrap_err();
    assert_eq!(error.code(), "security.key_missing");
}
