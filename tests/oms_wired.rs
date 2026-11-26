#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::output::{render_oms_capture, OmsFormat};
use m_bus_parser::{decode_bytes, DecodeOptions};

fn long(control: u8, address: u8, body: &[u8]) -> Vec<u8> {
    let len = (2 + body.len()) as u8;
    let mut bytes = vec![0x68, len, len, 0x68, control, address];
    bytes.extend_from_slice(body);
    let sum = bytes[4..].iter().copied().fold(0u8, u8::wrapping_add);
    bytes.extend([sum, 0x16]);
    bytes
}

#[test]
fn secondary_addressing_examples_still_require_the_slaves_own_response_address() {
    let application = [0x72, 0x78, 0x56, 0x34, 0x12, 0xa7, 0x3d, 1, 7, 5, 0, 0, 0];
    for (address, expected) in [
        (0xfd, "failed"),
        (0xfe, "failed"),
        (1, "passed"),
        (0, "passed"),
    ] {
        let frame = decode_bytes(&long(8, address, &application), &Default::default()).unwrap();
        let report = frame.assess_oms_5_0_1();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "Wired response primary address" && f.status.as_str() == expected));
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "Wired application address" && f.status.as_str() == "passed"));
    }
}

#[test]
fn wired_capture_checks_require_an_ordered_exchange() {
    let options = DecodeOptions::default();
    let broadcast = decode_bytes(&[0x10, 0x40, 0xff, 0x3f, 0x16], &options).unwrap();
    let ack = decode_bytes(&[0xe5], &options).unwrap();
    let frames = [broadcast, ack];
    assert!(render_oms_capture(&frames, OmsFormat::Json, Some(1), true)
        .unwrap()
        .contains("Wired silent broadcast"));
    assert!(
        !render_oms_capture(&frames, OmsFormat::Json, Some(1), false)
            .unwrap()
            .contains("Wired silent broadcast")
    );
    let request = decode_bytes(&[0x10, 0x5b, 2, 0x5d, 0x16], &options).unwrap();
    let reply = decode_bytes(&long(8, 3, &[0x78, 1, 0x13, 3]), &options).unwrap();
    let json = render_oms_capture(&[request, reply], OmsFormat::Json, Some(1), true).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["rule"] == "Wired response source" && f["status"] == "failed"));
}

#[test]
fn only_the_current_din_address_is_compared_with_the_application_address() {
    let manufacturer: u16 = (17 << 10) | (4 << 5) | 19;
    for (category, vif, status) in [
        (b'7', 0x79, Some("passed")),
        (b'8', 0x79, Some("failed")),
        (b'8', 0x78, None),
    ] {
        let mut data = vec![0x72, 0x44, 0x33, 0x22, 0x11];
        data.extend(manufacturer.to_le_bytes());
        data.extend([1, 3, 1, 0, 0, 0, 0x0d, vif, 14]);
        data.extend(b"4433221110SDQ");
        data.push(category);
        let frame = decode_bytes(&long(8, 1, &data), &Default::default()).unwrap();
        let report = frame.assess_oms_5_0_1();
        let result = report
            .findings
            .iter()
            .find(|f| f.rule == "Current DIN address conversion")
            .map(|f| f.status.as_str());
        assert_eq!(result, status);
    }
}
