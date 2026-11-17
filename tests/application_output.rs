#![cfg(feature = "std")]

use m_bus_parser::{
    decode_hex, output::render_application_hex, render_hex, DecodeOptions, OutputFormat,
    RenderOptions,
};

const RECORDS: &str = "03 13 15 31 00";
const CI_RECORDS: &str = "78 03 13 15 31 00";
const SEND_DATA_RECORDS: &str = "51 03 13 15 31 00";
const LONG_HEADER: &str = "72 00 51 20 02 82 4D 02 04 00 00 00 00 03 13 15 31 00";

#[test]
fn select_slave_renders_the_mask_without_claiming_a_meter_identity() {
    let frame = "680B0B6873FD5278F6F412FFFF01FF3416";
    let application = "5278F6F412FFFF01FF";
    for input in [frame, application] {
        for format in [
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Table,
            OutputFormat::Csv,
            OutputFormat::Mermaid,
        ] {
            let output = if input == frame {
                render_hex(input, format, &RenderOptions::default())
            } else {
                render_application_hex(input, format, &RenderOptions::default())
            }
            .unwrap();
            assert!(output.contains("12F4F678FFFF01FF"), "{input} / {format}");
            if format == OutputFormat::Json {
                let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
                assert_eq!(parsed["decode_state"], "complete");
                assert_eq!(parsed["transport"]["header_kind"], "slave_selection");
                assert_eq!(
                    parsed["transport"]["secondary_address_mask"],
                    "12F4F678FFFF01FF"
                );
                assert!(parsed["meter"]["identity"].is_null());
                assert!(parsed["records"].as_array().unwrap().is_empty());
            }
        }
    }
}

#[test]
fn select_slave_annotations_cover_the_address_fields() {
    for (input, base) in [
        ("680B0B6873FD5278F6F412FFFF01FF3416", 6),
        ("680B0B6853FD5278F6F412FFFF01FF1416", 6),
        ("5278F6F412FFFF01FF", 0),
    ] {
        let output = if base == 0 {
            render_application_hex(input, OutputFormat::Annotated, &RenderOptions::default())
        } else {
            render_hex(input, OutputFormat::Annotated, &RenderOptions::default())
        }
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let segments = parsed["segments"].as_array().unwrap();
        for (kind, start, end, value) in [
            ("identification_number", 1, 5, "12F4F678"),
            ("manufacturer_code", 5, 7, "FFFF"),
            ("version", 7, 8, "01"),
            ("device_type", 8, 9, "FF"),
        ] {
            let segment = segments
                .iter()
                .find(|segment| segment["kind"] == kind)
                .unwrap();
            assert_eq!(segment["start"], base + start);
            assert_eq!(segment["end"], base + end);
            assert!(segment["detail"].as_str().unwrap().ends_with(value));
        }
        assert!(!segments.iter().any(|segment| segment["kind"] == "unknown"));
    }
}

#[test]
fn send_data_frame_renders_written_values_and_record_annotations() {
    let input = "68 06 06 68 53 FE 51 01 7A 08 25 16";
    let output = render_hex(input, OutputFormat::Json, &RenderOptions::default()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["decode_state"], "complete");
    assert_eq!(parsed["frame"]["function"], "SND_UD (FCB: false)");
    assert_eq!(parsed["transport"]["header_kind"], "none");
    assert_eq!(parsed["records"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["records"][0]["value"]["value"], "8");

    let output = render_hex(input, OutputFormat::Annotated, &RenderOptions::default()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let segments = parsed["segments"].as_array().unwrap();
    let payload = segments
        .iter()
        .find(|segment| segment["kind"] == "data_payload")
        .unwrap();
    assert_eq!(payload["start"], 9);
    assert_eq!(payload["end"], 10);
    assert!(!segments.iter().any(|segment| segment["kind"] == "unknown"));
}

#[test]
fn application_only_inputs_render_in_every_website_format() {
    for input in [RECORDS, CI_RECORDS, SEND_DATA_RECORDS, LONG_HEADER] {
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
    for (input, length, payload_start) in [
        (RECORDS, 5, 2),
        (CI_RECORDS, 6, 3),
        (SEND_DATA_RECORDS, 6, 3),
        (LONG_HEADER, 18, 15),
    ] {
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
        "51 03 13 15",
        "52 78 F6 F4 12 FF FF 01",
        "52 78 F6 F4 12 FF FF 01 FF 00",
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
