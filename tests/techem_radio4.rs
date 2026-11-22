#![cfg(feature = "std")]

use m_bus_parser::{decode_hex, DecodeOptions};

#[test]
fn legacy_radio4_has_two_volume_counters_and_no_short_tpl_header() {
    let frame = "2F446850200141029562A206702901006017030004000300000000000000000000000000000000000000000000000000";
    let output = decode_hex(frame, &DecodeOptions::default()).unwrap();
    assert_eq!(output.security.payload_protection, "cleartext");
    assert!(output.transport.header_kind.is_none());
    let values = output.application.unwrap().decoded.unwrap();
    assert_eq!(values["month_start_volume_m3"], "0.1");
    assert_eq!(values["volume_since_month_start_m3"], "0.3");
    assert_eq!(values["total_volume_m3"], "0.4");
    let near_match = frame.replacen("9562", "9563", 1);
    let other = decode_hex(&near_match, &DecodeOptions::default()).unwrap();
    assert!(other.application.is_none());
}

#[test]
fn truncated_radio4_does_not_invent_a_reading() {
    let error = decode_hex("0C446850200141029562A20670", &DecodeOptions::default()).unwrap_err();
    assert_eq!(error.code(), "application.radio4_truncated");
}
