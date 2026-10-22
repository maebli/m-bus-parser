#![cfg(feature = "std")]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{
    decode_bytes, output::render_application_hex, render_bytes, DecodeOptions, OutputFormat,
    RenderOptions,
};
use serde_json::{json, Value};

fn wired_frame(application: &[u8]) -> Vec<u8> {
    let length = u8::try_from(application.len() + 2).unwrap();
    let mut bytes = vec![0x68, length, length, 0x68, 0x53, 0xFE];
    bytes.extend_from_slice(application);
    let checksum = bytes[4..]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    bytes.extend_from_slice(&[checksum, 0x16]);
    bytes
}

#[test]
fn libmbus_baud_frames_decode_with_either_frame_count_bit() {
    for (frame, baud) in [
        ("6803036853FEB80916", 300),
        ("6803036853FEB90A16", 600),
        ("6803036853FEBA0B16", 1200),
        ("6803036853FEBB0C16", 2400),
        ("6803036853FEBC0D16", 4800),
        ("6803036853FEBD0E16", 9600),
        ("6803036853FEBE0F16", 19200),
        ("6803036853FEBF1016", 38400),
    ] {
        let mut bytes = hex::decode(frame).unwrap();
        for frame_count in [false, true] {
            if frame_count {
                bytes[4] += 0x20;
                bytes[7] = bytes[7].wrapping_add(0x20);
            }
            let decoded = decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
            assert_eq!(decoded.decode_state, "complete");
            assert!(decoded.records.is_empty());
            let application = decoded.application.unwrap();
            assert_eq!(application.protocol, "set_baud_rate");
            assert_eq!(application.decoded, Some(json!({"BaudRate": baud})));
        }
    }
}

#[test]
fn commands_reports_and_external_payloads_render_in_all_formats() {
    for (application, protocol) in [
        (&[0x5C][..], "synchronize"),
        (&[0x54][..], "request_selected_application"),
        (&[0x70, 8][..], "application_error"),
        (&[0x71, 0x81][..], "alarm"),
        (&[0xB2, 0xDE, 0xAD][..], "write_ram"),
        (&[0x79, 0, 0, 0, 0, 1][..], "compact"),
    ] {
        let bytes = wired_frame(application);
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
            for output in [
                render_bytes(&bytes, format, &RenderOptions::default()).unwrap(),
                render_application_hex(
                    &hex::encode(application),
                    format,
                    &RenderOptions::default(),
                )
                .unwrap(),
            ] {
                assert!(output.contains(protocol), "{protocol} / {format}: {output}");
            }
        }
        let decoded = decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
        assert!(decoded.records.is_empty());
        match protocol {
            "write_ram" | "compact" => {
                assert_eq!(decoded.decode_state, "partial");
                assert!(!decoded.diagnostics.is_empty());
            }
            _ => assert_eq!(decoded.decode_state, "complete"),
        }
        if protocol == "application_error" {
            assert_eq!(
                decoded.application.unwrap().decoded.unwrap()["error_code"],
                8
            );
        }
    }
}

#[test]
fn ell_wrapped_reports_keep_transport_metadata_and_annotation_offsets() {
    let application = [0x8C, 0x20, 0x12, 0x6E, 3, 0, 0, 0, 8];
    let bytes = wired_frame(&application);
    let decoded = decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
    assert_eq!(decoded.decode_state, "complete");
    let value = serde_json::to_value(decoded).unwrap();
    assert_eq!(value["transport"]["ell"]["access_number"], 0x12);
    assert_eq!(value["application"]["decoded"]["error_code"], 8);
    let output = render_bytes(&bytes, OutputFormat::Annotated, &RenderOptions::default()).unwrap();
    let value: Value = serde_json::from_str(&output).unwrap();
    let segments = value["segments"].as_array().unwrap();
    let error = segments
        .iter()
        .find(|segment| segment["kind"] == "application_error_code")
        .unwrap();
    assert_eq!(error["start"], 14);
    assert_eq!(error["end"], 15);
    assert!(!segments.iter().any(|segment| segment["kind"] == "unknown"));
}

#[test]
fn malformed_application_payloads_remain_visible_as_partial_frames() {
    for application in [
        &[0x5C, 0][..],
        &[0x6D, 0][..],
        &[0x69, 1, 0, 0][..],
        &[0x79, 0][..],
    ] {
        let decoded = decode_bytes(&wired_frame(application), &DecodeOptions::default()).unwrap();
        assert_eq!(decoded.decode_state, "partial");
        assert!(!decoded.diagnostics.is_empty());
        assert!(decoded.application.is_none());
        assert!(render_application_hex(
            &hex::encode(application),
            OutputFormat::Json,
            &RenderOptions::default()
        )
        .is_err());
    }
}

#[cfg(feature = "decryption")]
#[test]
fn oms_clock_vector_decrypts_without_becoming_measurement_records() {
    let application =
        hex::decode("6D44227592242329077D0010053A9731FBF434681C415478FBEA0BC66E").unwrap();
    let key: [u8; 16] = hex::decode("82B0551191F51D66EFCDAB8967452301")
        .unwrap()
        .try_into()
        .unwrap();
    let bytes = wired_frame(&application);
    let missing_key = decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
    assert_eq!(missing_key.decode_state, "partial");
    assert!(missing_key.application.unwrap().decoded.is_none());

    let options = RenderOptions {
        decode: DecodeOptions {
            key: Some(key),
            ..Default::default()
        },
        ..Default::default()
    };
    let decoded = decode_bytes(&bytes, &options.decode).unwrap();
    assert_eq!(decoded.decode_state, "complete");
    assert!(decoded.records.is_empty());
    let value = serde_json::to_value(decoded).unwrap();
    assert_eq!(value["meter"]["identity"]["id"], "92752244");
    assert_eq!(value["application"]["decoded"]["Clock"]["operation"], "Add");
    assert_eq!(
        value["application"]["decoded"]["Clock"]["value"]["Time"],
        json!([{"Single": 50}, {"Single": 1}, {"Single": 0}])
    );

    let mut wireless = hex::decode("26532423563412900831").unwrap();
    wireless.extend_from_slice(&application);
    let decoded = decode_bytes(&wireless, &options.decode).unwrap();
    assert_eq!(decoded.decode_state, "complete");
    assert!(decoded.records.is_empty());

    for frame in [&bytes, &wireless] {
        let xml = render_bytes(frame, OutputFormat::Xml, &options).unwrap();
        assert!(xml.contains("encrypted=\"false\""));
        assert!(xml.contains("2F2F013201000000000000002F2F2F2F"));
        let annotated = render_bytes(frame, OutputFormat::Annotated, &options).unwrap();
        let value: Value = serde_json::from_str(&annotated).unwrap();
        assert_eq!(value["decrypted"], true);
        assert!(value["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|segment| segment["kind"] == "clock_time"));
    }
    let output =
        render_application_hex(&hex::encode(&application), OutputFormat::Json, &options).unwrap();
    let value: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["decode_state"], "complete");
    assert_eq!(value["application"]["decoded"]["Clock"]["operation"], "Add");

    let wrong_key = DecodeOptions {
        key: Some([0; 16]),
        ..Default::default()
    };
    match decode_bytes(&bytes, &wrong_key) {
        Err(error) => assert_eq!(error.code(), "security.decryption_failed"),
        Ok(decoded) => {
            assert_eq!(decoded.decode_state, "partial");
            assert!(decoded.application.unwrap().decoded.is_none());
        }
    }

    let mut cleartext = application[..13].to_vec();
    cleartext[11..13].copy_from_slice(&[0, 0xC0]);
    cleartext.extend_from_slice(&hex::decode("2F2F013201000000000000002F2F2F2F").unwrap());
    let decoded = decode_bytes(&wired_frame(&cleartext), &DecodeOptions::default()).unwrap();
    assert_eq!(decoded.decode_state, "complete");
}
