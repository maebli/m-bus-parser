#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::output::{
    render_oms_capture_with_options, OmsCaptureOptions, OmsFormat, OmsRadioMode,
};
use m_bus_parser::{decode_hex, DecodeOptions, DecodedOutput};
use serde_json::Value;

const METER: &str = "9315785634123303";
const GATEWAY: &str = "3A63665544330A31";
const PROFILE_D: &str = "304493157856341233038C20757A7500112A1001B30A0000C882038E223BBC1091CEB8C0A1009D388A5AF7883A5B5A7FD6";

fn frame(control: u8, sender: &str, body: &str) -> DecodedOutput {
    let packet = format!("{control:02X}{sender}{body}");
    decode_hex(
        &format!("{:02X}{packet}", packet.len() / 2),
        &DecodeOptions::default(),
    )
    .unwrap()
}

fn status<'a>(report: &'a Value, name: &str) -> Option<&'a str> {
    report["findings"]
        .as_array()?
        .iter()
        .find(|finding| finding["rule"] == name)?["status"]
        .as_str()
}

fn report(frame: &DecodedOutput) -> Value {
    serde_json::to_value(frame.assess_oms_5_0_1()).unwrap()
}

#[test]
fn profile_d_reports_transport_counter_and_validates_clear_headers_without_a_key() {
    let decoded = decode_hex(PROFILE_D, &DecodeOptions::default()).unwrap();
    assert_eq!(decoded.security.message_counter, Some(0xab3));
    assert_eq!(decoded.security.key_id, Some(0));
    assert_eq!(decoded.security.key_version, None);
    let good = report(&decoded);
    assert_eq!(status(&good, "CCM encrypted byte count"), Some("passed"));
    assert_eq!(
        status(&good, "Message authentication"),
        Some("could_not_check")
    );
    let bad = frame(0x44, METER, "7A0100202A1901000000000000000000000000");
    let bad = report(&bad);
    for rule in [
        "CCM encrypted byte count",
        "Communication security key ID",
        "Transmitted security counter",
    ] {
        assert_eq!(status(&bad, rule), Some("failed"), "{rule}");
    }
    let versioned = frame(0x44, METER, "7A0100002A510103443322110000000000000000");
    assert_eq!(versioned.security.key_version, Some(3));
    assert_eq!(versioned.security.message_counter, Some(0x11223344));
    assert_eq!(versioned.security.key_id, Some(1));
}

#[test]
fn cbc_partial_and_full_encryption_must_fit_the_body() {
    let short = report(&frame(0x44, METER, "7A0100200710000000"));
    assert_eq!(status(&short, "CBC encrypted block count"), Some("failed"));
    let full = format!("7A0100F00710{}", "00".repeat(16));
    assert_eq!(
        status(
            &report(&frame(0x44, METER, &full)),
            "CBC encrypted block count"
        ),
        Some("passed")
    );
}

#[test]
fn tls_record_type_selects_ci_and_cmac_requirements() {
    let channel = report(&frame(
        0x43,
        GATEWAY,
        "5F78563412931533030100FF0D000000000000",
    ));
    assert_eq!(
        status(&channel, "TLS record CI"),
        Some("passed"),
        "{channel}"
    );
    assert_eq!(status(&channel, "TLS handshake CMAC"), Some("failed"));
    let app = report(&frame(0x44, METER, "7A0100FF0D01170303000100"));
    assert_eq!(status(&app, "TLS record CI"), Some("passed"));
    assert_eq!(status(&app, "TLS application CMAC"), Some("passed"));
    let alert = report(&frame(0x44, METER, "7A0100FF0D0115030300020100"));
    assert_eq!(status(&alert, "TLS alert protection"), Some("failed"));
}

#[test]
fn radio_mode_controls_ell_presence_accessibility_and_mixed_usage() {
    let no_ell = frame(0x44, METER, "7A01000080011301");
    let ell = frame(0x44, METER, "8C20017A01000000011301");
    assert_eq!(status(&report(&no_ell), "Mode C extended link layer"), None);
    let mut options = OmsCaptureOptions {
        radio_mode: Some(OmsRadioMode::C2),
        ..Default::default()
    };
    let c2 = serde_json::to_value(no_ell.assess_oms_with_options(options)).unwrap();
    assert_eq!(status(&c2, "Mode C extended link layer"), Some("failed"));
    options.radio_mode = Some(OmsRadioMode::C1);
    let c1: Value = serde_json::from_str(
        &render_oms_capture_with_options(&[no_ell, ell], OmsFormat::Json, None, options).unwrap(),
    )
    .unwrap();
    assert_eq!(status(&c1[0], "Mode C extended link layer"), Some("passed"));
    assert_eq!(status(&c1[0], "Mode C1 accessibility"), Some("failed"));
    assert_eq!(
        status(&c1[1], "Consistent Mode C ELL usage"),
        Some("failed")
    );
}

#[test]
fn mode_d_response_follows_the_last_addressed_key_version_request() {
    let request = |destination: &str| {
        frame(
            0x43,
            GATEWAY,
            &format!("8E8012{destination}8056341278931533030100002A500107443322110000000000000000"),
        )
    };
    let response = || {
        frame(
            0x08,
            METER,
            &format!("8E8012{GATEWAY}7A0100002A1001443322110000000000000000"),
        )
    };
    let check = |frames: &[DecodedOutput], consecutive| {
        let options = OmsCaptureOptions {
            consecutive,
            ..Default::default()
        };
        serde_json::from_str::<Value>(
            &render_oms_capture_with_options(frames, OmsFormat::Json, Some(1), options).unwrap(),
        )
        .unwrap()
    };
    let rule = "Response key information flag";
    assert_eq!(
        status(&check(&[request(METER), response()], true), rule),
        Some("failed")
    );
    assert_eq!(
        status(&check(&[request(METER), response()], false), rule),
        Some("could_not_check")
    );
    assert_eq!(
        status(
            &check(&[request("9315785634133303"), response()], true),
            rule
        ),
        Some("could_not_check")
    );
}
