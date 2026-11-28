#![cfg(feature = "std")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::output::{render_oms_capture, OmsFormat};
use m_bus_parser::{decode_hex, DecodeOptions};
use serde_json::Value;

fn report(input: &str) -> Value {
    let frame = decode_hex(input, &DecodeOptions::default()).unwrap();
    let json = render_oms_capture(&[frame], OmsFormat::ValidationJson, None, false).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn finding<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rule"] == name)
        .unwrap()
}

#[test]
fn record_failure_keeps_the_frame_result_and_record_offset() {
    let report = report("164493157856341233038C00117A1100000001132A0413");
    assert_eq!(
        finding(&report, "Wireless frame length")["status"],
        "passed"
    );
    assert_eq!(
        finding(&report, "Wireless CRC")["status"],
        "could_not_check"
    );
    let record = finding(&report, "Application decoding");
    assert_eq!(record["status"], "failed");
    assert_eq!(record["offset_start"], 3);
}

#[test]
fn wired_integrity_and_oms_counts_are_reported_together() {
    let report = report("1040FF3F16");
    assert_eq!(finding(&report, "Wired checksum")["status"], "passed");
    assert_eq!(
        finding(&report, "Transport authentication")["status"],
        "not_applicable"
    );
    let rows = report["findings"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["group"] == "OMS"));
    assert_eq!(
        report["assessed"].as_u64().unwrap() as usize,
        rows.iter()
            .filter(|row| matches!(row["status"].as_str(), Some("passed" | "failed")))
            .count()
    );
}

#[test]
fn format_a_block_crcs_are_checked_when_present() {
    let data = hex::decode("164493157856341233038C00117A1100000002FD170000").unwrap();
    let (header, payload) = data.split_at(10);
    let mut wire = Vec::new();
    for block in [header, payload] {
        wire.extend_from_slice(block);
        wire.extend_from_slice(&wireless_mbus_link_layer::crc16_en13757(block).to_be_bytes());
    }
    assert_eq!(
        finding(&report(&hex::encode(wire)), "Wireless CRC")["status"],
        "passed"
    );
}
