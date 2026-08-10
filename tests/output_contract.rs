#![cfg(feature = "std")]
#![allow(clippy::unwrap_used)]

use std::str::FromStr;

use m_bus_parser::{
    decode_hex, render_hex, DecodeOptions, OutputError, OutputFormat, RenderOptions,
};

const WIRED_FRAME: &str = concat!(
    "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 ",
    "04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B ",
    "00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C ",
    "8C 11 02 27 37 0D 0F 60 00 67 16"
);

#[test]
fn canonical_json_has_a_versioned_stable_root() {
    let rendered = render_hex(WIRED_FRAME, OutputFormat::Json, &RenderOptions::default()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    let object = value.as_object().unwrap();

    for field in [
        "schema_version",
        "decode_state",
        "protocol",
        "frame",
        "meter",
        "transport",
        "security",
        "records",
        "raw",
        "diagnostics",
        "enrichment",
    ] {
        assert!(
            object.contains_key(field),
            "missing canonical field {field}"
        );
    }
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["protocol"], "wired");
    assert!(!object.contains_key("summary"));
    assert_eq!(
        value["frame"]["function"],
        "RSP_UD (ACD: false, DFC: false)"
    );
    assert_eq!(value["records"][2]["function"], "Instantaneous value");
    assert_eq!(value["records"][3]["quantities"][0], "Volume flow");
    assert_eq!(value["records"][4]["quantities"][0], "Flow temperature");
    assert_eq!(value["records"][2]["data_coding"], "6-digit BCD");
    assert_eq!(value["records"][9]["function"], "Manufacturer specific");
    assert_eq!(value["records"][2]["unit"], "W");
    assert!(value["records"][2]["unit"].is_string());
    assert_eq!(value["records"][3]["unit"], "m3.h-1");
}

#[test]
fn enrichment_is_optional_without_changing_the_schema() {
    let decoded = decode_hex(
        WIRED_FRAME,
        &DecodeOptions {
            key: None,
            include_enrichment: false,
        },
    )
    .unwrap();
    let value = serde_json::to_value(decoded).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert!(value.get("enrichment").is_none());
}

#[test]
fn formats_and_compatibility_aliases_share_one_parser() {
    assert_eq!(OutputFormat::from_str("yml").unwrap(), OutputFormat::Yaml);
    assert_eq!(
        OutputFormat::from_str("table_format").unwrap(),
        OutputFormat::Table
    );
    assert_eq!(
        OutputFormat::from_str("hexview").unwrap(),
        OutputFormat::Annotated
    );

    let csv = render_hex(WIRED_FRAME, OutputFormat::Csv, &RenderOptions::default()).unwrap();
    assert!(csv.starts_with("schema_version,protocol,frame_kind"));
    assert_eq!(
        csv.lines().count(),
        2,
        "one input frame must produce one CSV data row"
    );
    assert!(csv.lines().next().unwrap().contains("record_0_value"));
    assert!(csv.contains("Instantaneous value"));
    assert!(csv.contains("Volume flow"));
    assert!(csv.contains("6-digit BCD"));

    let yaml = render_hex(WIRED_FRAME, OutputFormat::Yaml, &RenderOptions::default()).unwrap();
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(yaml_value["records"][3]["quantities"][0], "Volume flow");
    assert_eq!(yaml_value["records"][3]["unit"], "m3.h-1");
}

#[test]
fn mermaid_restores_semantic_color_and_grouping() {
    let diagram = render_hex(
        WIRED_FRAME,
        OutputFormat::Mermaid,
        &RenderOptions::default(),
    )
    .unwrap();

    assert!(diagram.contains("subgraph FRAME_SG"));
    assert!(diagram.contains("subgraph RECORDS_SG"));
    assert!(diagram.contains("classDef frame"));
    assert!(diagram.contains("classDef record0"));
    assert!(diagram.contains("class R0 record0"));
}

#[test]
fn typed_errors_have_stable_codes_and_offsets() {
    let error = decode_hex("68,3D", &DecodeOptions::default()).unwrap_err();
    assert_eq!(error.code(), "input.invalid_hex");
    assert_eq!(error.layer(), "input");
    assert!(error.byte_offset().is_some());

    let error = OutputFormat::from_str("html").unwrap_err();
    assert_eq!(error.code(), "format.unsupported");

    let invalid_frame = decode_hex("0102", &DecodeOptions::default()).unwrap_err();
    assert!(matches!(invalid_frame, OutputError::InvalidFrame { .. }));
    assert_eq!(invalid_frame.code(), "frame.invalid");
}
