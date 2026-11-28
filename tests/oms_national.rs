#![cfg(feature = "std")]
#![allow(clippy::unwrap_used)]
use m_bus_parser::{
    decode_hex,
    output::{OmsCaptureOptions, OmsNationalProfile},
};

const WIRELESS_CLEAR: &str = "114493157856341233037a0100000001132a";
const WIRELESS_B: &str="434493157856341233038C2075900F002C25B30A000021924D4F2FB66E017A75002007109058475F4BC91DF878B80A1B0F98B629024AAC727942BFC549233C0140829B93";

#[test]
fn national_checks_only_run_for_the_declared_interface() {
    let clear = decode_hex(WIRELESS_CLEAR, &Default::default()).unwrap();
    assert!(!clear
        .assess_oms_5_0_1()
        .findings
        .iter()
        .any(|f| f.rule.starts_with("National profile")));
    for (frame, profile, expected) in [
        (WIRELESS_CLEAR, OmsNationalProfile::Idis, "failed"),
        (WIRELESS_B, OmsNationalProfile::BsiUnidirectional, "passed"),
        (WIRELESS_B, OmsNationalProfile::BsiBidirectional, "passed"),
    ] {
        let decoded = decode_hex(frame, &Default::default()).unwrap();
        let report = decoded.assess_oms_with_options(OmsCaptureOptions {
            national_profile: Some(profile),
            ..Default::default()
        });
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule == "National profile data protection"
                    && f.status.as_str() == expected),
            "{frame}"
        );
    }
}

#[test]
fn annex_e_uses_the_meter_types_from_its_referenced_edition() {
    let frame = decode_hex(
        &WIRELESS_B.replacen("123303", "12331a", 1),
        &Default::default(),
    )
    .unwrap();
    let report = frame.assess_oms_with_options(OmsCaptureOptions {
        national_profile: Some(OmsNationalProfile::BsiUnidirectional),
        ..Default::default()
    });
    assert!(report
        .findings
        .iter()
        .any(|f| f.rule == "National profile meter type" && f.status.as_str() == "failed"));
}
