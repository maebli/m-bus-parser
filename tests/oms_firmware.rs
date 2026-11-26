#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::output::{render_application_oms_with_evidence, OmsFormat};
use serde_json::Value;

const PREPARE: &str = "C6010000003D0000200032230001000000000000003006250001000D0200FA0000A73DFFFF0000FFFF041800452A116F9AD5CECFA7CB94CD233859F67A64112D0880EBCD";

fn check(input: &str, image: Option<&[u8]>, key: Option<&[u8; 16]>) -> Value {
    let output = render_application_oms_with_evidence(
        input,
        OmsFormat::Json,
        &Default::default(),
        Default::default(),
        None,
        key,
        |frame| {
            if let Some(data) = image {
                frame.verify_firmware_image(data, &core::array::from_fn(|i| i as u8))?;
            }
            Ok(())
        },
    )
    .unwrap();
    serde_json::from_str(&output).unwrap()
}

fn status<'a>(report: &'a Value, rule: &str) -> &'a str {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == rule)
        .unwrap()["status"]
        .as_str()
        .unwrap()
}

#[test]
fn complete_image_and_separate_firmware_key_verify_the_prepare_cmac() {
    let input = check(PREPARE, Some(&[0; 500]), Some(&[0x11; 16]));
    for rule in [
        "Firmware transfer fields",
        "Firmware transfer size declaration",
        "Firmware image identifier",
        "Firmware image authentication fields",
        "Firmware image CMAC",
        "Firmware prepare application protection",
        "Firmware command application protection",
        "Firmware target time",
    ] {
        assert_eq!(status(&input, rule), "passed", "{rule}");
    }
    assert_eq!(
        status(
            &check(PREPARE, Some(&[1; 500]), None),
            "Firmware image CMAC"
        ),
        "failed"
    );
    assert_eq!(
        status(&check(PREPARE, None, None), "Firmware image CMAC"),
        "could_not_check"
    );
}

#[test]
fn prepare_and_other_operations_follow_their_own_protection_rules() {
    let prepare = check("C7010000000500010000", None, None);
    for rule in [
        "Firmware prepare application protection",
        "Firmware transfer size declaration",
        "Firmware image identifier",
        "Firmware image authentication fields",
    ] {
        assert_eq!(status(&prepare, rule), "failed");
    }
    let state = check("C7010000000D00010402A73DFFFF0000FFFF", None, None);
    assert_eq!(status(&state, "Firmware transfer direction"), "passed");
    assert_eq!(status(&state, "Firmware transfer fields"), "passed");
    let reply = check("C1010000000F0001830CA73DFFFF0000FFFF0000", None, None);
    assert_eq!(status(&reply, "Firmware lost block list"), "passed");
    let missing = check("C1010000000D00018308A73DFFFF0000FFFF", None, None);
    assert_eq!(status(&missing, "Firmware lost block list"), "failed");
    let malformed = check("C7010000000C00010482A73DFFFF0000FF", None, None);
    assert_eq!(
        status(&malformed, "Firmware reserved subfunction bits"),
        "failed"
    );
    assert_eq!(status(&malformed, "Firmware transfer fields"), "failed");
}

#[test]
fn ordered_blocks_use_the_prepare_from_the_same_meter_and_image() {
    use m_bus_parser::output::{render_oms_capture_with_options, OmsCaptureOptions};
    use m_bus_parser::{decode_hex, DecodeOptions};
    let radio = |ci: u8, id: &str, body: &str| {
        let bytes = format!("433A63665544330A31{ci:02X}{id}9315330301000000{body}");
        decode_hex(
            &format!("{:02X}{bytes}", bytes.len() / 2),
            &DecodeOptions::default(),
        )
        .unwrap()
    };
    let prep = radio(0xc3, "78563412", &PREPARE[10..]);
    let transfer = radio(
        0xc0,
        "78563412",
        &format!("1700010208A73DFFFF0000FFFF0000{}", "AA".repeat(8)),
    );
    let other = radio(0xc0, "88563412", "1200010218A73DFFFF0000FFFF02000400AA");
    let capture = render_oms_capture_with_options(
        &[prep, transfer, other],
        OmsFormat::Json,
        None,
        OmsCaptureOptions {
            consecutive: true,
            ..Default::default()
        },
    )
    .unwrap();
    let results: Value = serde_json::from_str(&capture).unwrap();
    assert_eq!(
        status(&results[1], "Firmware transfer block number"),
        "passed"
    );
    assert_eq!(
        status(&results[1], "Firmware transfer block size"),
        "failed"
    );
    assert_eq!(
        status(&results[2], "Firmware transfer block number"),
        "could_not_check"
    );
}

#[test]
fn each_segment_has_a_length_id_and_matching_response() {
    use m_bus_parser::output::{render_oms_capture_with_options, OmsCaptureOptions};
    use m_bus_parser::{decode_hex, DecodeOptions};
    let radio = |ci: u8, segments: &str| {
        let raw = format!("433A63665544330A31{ci:02X}785634129315330301000000{segments}");
        decode_hex(
            &format!("{:02X}{raw}", raw.len() / 2),
            &DecodeOptions::default(),
        )
        .unwrap()
    };
    let request = radio(0xc0, "0D00010402A73DFFFF0000FFFF0D00020502A73DFFFF0000FFFF");
    let valid = radio(0xc2, "0D0001841FA73DFFFF0000FFFF0D00028519A73DFFFF0000FFFF");
    let bad = radio(0xc2, "0D0009841FA73DFFFF0000FFFF");
    let cap = render_oms_capture_with_options(
        &[request.clone(), valid, request, bad],
        OmsFormat::Json,
        None,
        OmsCaptureOptions {
            consecutive: true,
            ..Default::default()
        },
    )
    .unwrap();
    let values: Value = serde_json::from_str(&cap).unwrap();
    assert_eq!(status(&values[1], "Firmware response segment"), "passed");
    assert_eq!(status(&values[3], "Firmware response segment"), "failed");
    assert_eq!(
        status(
            &check("C7010000002A00010402A73DFFFF0000FFFF", None, None),
            "Firmware transfer segments"
        ),
        "failed"
    );
}
