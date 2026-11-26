#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used)]

use m_bus_parser::{decode_hex, DecodeOptions};

const GET: &str = "C001C1000100002B0100FF0200";
const DATA: &str = "C401C100060000001C";
const RELEASE: &str = "6203800100";
const RELEASE_REPLY: &str = "6315800100BE10040E0800065F1F0400001010010D0007";

fn result(ci: u8, apdu: &str, rule: &str) -> &'static str {
    let frame = format!(
        "{:02X}449344785634120A07{ci:02X}010000001001{apdu}",
        16 + apdu.len() / 2
    );
    decode_hex(&frame, &DecodeOptions::default())
        .unwrap()
        .assess_oms_5_0_1()
        .findings
        .into_iter()
        .find(|f| f.rule == rule)
        .unwrap()
        .status
        .as_str()
}

#[test]
fn gurux_client_get_and_release_exchanges_have_exact_service_boundaries() {
    for (ci, apdu) in [
        (0x61, GET),
        (0x7d, DATA),
        (0x61, RELEASE),
        (0x7d, RELEASE_REPLY),
    ] {
        assert_eq!(result(ci, apdu, "DLMS service APDU encoding"), "passed");
        assert_eq!(result(ci, apdu, "DLMS service direction"), "passed");
        assert_eq!(
            result(ci, &format!("{apdu}00"), "DLMS service APDU encoding"),
            "failed"
        );
        assert_eq!(
            result(ci, &apdu[..apdu.len() - 2], "DLMS service APDU encoding"),
            "failed"
        );
    }
    assert_eq!(result(0x61, GET, "DLMS invoke ID reserved bits"), "passed");
    assert_eq!(
        result(
            0x61,
            &GET.replacen("C1", "F1", 1),
            "DLMS invoke ID reserved bits"
        ),
        "failed"
    );
    assert_eq!(
        result(
            0x7d,
            &RELEASE_REPLY.replace("080006", "F90006"),
            "DLMS service APDU encoding"
        ),
        "failed"
    );
    assert_eq!(result(0x7d, GET, "DLMS service direction"), "failed");
    assert_eq!(
        result(0x7d, "C401C100AA", "DLMS service APDU encoding"),
        "failed"
    );
    assert_eq!(
        result(0x7d, "F9", "DLMS service APDU encoding"),
        "could_not_check"
    );
}
