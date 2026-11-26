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

#[test]
fn tls_hello_extensions_are_assessed_in_the_capture() {
    let send = |client, extensions: &str| {
        let (kind, fields, control, sender, receiver, tpl) = if client {
            (1, "0002C0230100", 8, METER, GATEWAY, "9E")
        } else {
            (2, "C02300", 0x43, GATEWAY, METER, "5F7856341293153303")
        };
        let body = format!(
            "0303{}00{fields}{:04X}{extensions}",
            "00".repeat(32),
            extensions.len() / 2
        );
        let handshake = format!("{kind:02X}{:06X}{body}", body.len() / 2);
        frame(
            control,
            sender,
            &format!(
                "8E8001{receiver}{tpl}0100FF0D01160303{:04X}{handshake}",
                handshake.len() / 2
            ),
        )
    };
    let client = send(true, "000A000600040017001A000B0002010000160000");
    let hello = report(&client);
    for rule in [
        "TLS hello sender",
        "TLS truncated HMAC extension",
        "TLS required elliptic curves",
        "TLS uncompressed point format",
        "TLS encrypt-then-MAC offer",
    ] {
        assert_eq!(status(&hello, rule), Some("passed"), "{rule}");
    }
    assert_eq!(status(&hello, "TLS handshake CMAC"), Some("failed"));
    let server = send(false, "00040000");
    let options = OmsCaptureOptions {
        consecutive: true,
        ..Default::default()
    };
    let response: Value = serde_json::from_str(
        &render_oms_capture_with_options(&[client, server], OmsFormat::Json, Some(1), options)
            .unwrap(),
    )
    .unwrap();
    for rule in [
        "TLS truncated HMAC extension",
        "TLS server encrypt-then-MAC selection",
    ] {
        assert_eq!(status(&response, rule), Some("failed"), "{rule}");
    }
}

#[test]
fn table_42_security_depends_on_protocol_and_link() {
    let clear_select = report(&frame(0x43, GATEWAY, "5378563412931533030100000000"));
    assert_eq!(
        status(&clear_select, "Application protocol encryption"),
        Some("failed")
    );
    assert_eq!(
        status(&clear_select, "Application protocol authentication"),
        Some("failed")
    );
    let tls_push = report(&frame(0x44, METER, "7A0100FF0D01170303000100"));
    assert_eq!(
        status(&tls_push, "Unsolicited TLS transmission"),
        Some("failed")
    );
    let dlms = report(&frame(0x44, METER, "7D0100000000"));
    assert_eq!(
        status(&dlms, "Application protocol encryption"),
        Some("could_not_check")
    );
}

#[test]
fn clock_management_accepts_optional_gateway_fields() {
    use m_bus_parser::user_data::application_message::parse_sitp_clock;
    let prefix = "6C785634129315330301000000";
    for tail in ["", "AABBCCDD", "AABBCCDDEEFF", "AABBCCDDEEFFDEADBEEF"] {
        let result = frame(0x43, GATEWAY, &format!("{prefix}01320000{tail}"));
        assert_eq!(result.application.unwrap().protocol, "clock_adjust");
    }
    assert!(parse_sitp_clock(&[1, 50, 0, 0]).is_ok());
    assert!(parse_sitp_clock(&[1, 50, 0]).is_err());
}

#[test]
fn fresh_authenticated_counters_use_sender_specific_increments() {
    use m_bus_oms_rules::sequence::ExchangeFrame;
    let observation = |function, counter, access| ExchangeFrame {
        sender: [1; 8],
        receiver: Some([2; 8]),
        function,
        link_access: Some(access),
        transport_access: Some(access),
        transport_status: Some(0),
        fragmented: false,
        security_mode: Some(10),
        key_information_present: Some(true),
        security_counter: Some(counter),
        key_id: Some(0),
        key_version: Some(3),
        verified_encryption: true,
        tls_hello: None,
    };
    for (function, change, consecutive, expected) in [
        (0x44, 1, true, "passed"),
        (0x44, 2, true, "failed"),
        (0x53, 100, true, "passed"),
        (0x53, 101, true, "failed"),
        (0x44, 2, false, "could_not_check"),
    ] {
        let mut checked = frame(0x44, METER, "7A01000000011301").assess_oms_5_0_1();
        observation(function, 10, 2).check_reply(
            observation(function, 10 + change, 3),
            consecutive,
            &mut checked,
        );
        let found = checked
            .findings
            .iter()
            .find(|f| f.rule == "Authenticated message counter progression")
            .unwrap();
        assert_eq!(found.status.as_str(), expected);
    }
}
