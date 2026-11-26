#![cfg(all(feature = "std", feature = "oms-gen5"))]

use m_bus_core::ConfigurationField;
use m_bus_oms_rules::{
    layers::LayerFields,
    readings::Reading,
    telegram::{self, Link, RecordAvailability, TelegramReport},
};

fn record<'a>(header: &'a [u8], data: &'a [u8]) -> Option<Reading<&'a [u8]>> {
    Some(Reading {
        header,
        data,
        tariff: 0,
        subunit: 0,
        storage: 0,
        protection: None,
        profile_time: None,
    })
}

fn report<'a>(
    device: u8,
    cf: u16,
    ext: u16,
    availability: RecordAvailability,
    records: Vec<Option<Reading<&'a [u8]>>>,
) -> TelegramReport {
    telegram::assess_telegram(
        Link::Wireless(0x44),
        Some(ConfigurationField::from(cf)),
        LayerFields {
            device_type: Some(device),
            configuration_extension: Some(ext),
            communication_control: Some(0x20),
            application_ci: Some(0x7a),
            ..Default::default()
        },
        availability,
        records,
    )
}

fn outcome<'a>(report: &'a TelegramReport, name: &str) -> Option<&'a str> {
    report
        .findings
        .iter()
        .find(|f| f.rule == name)
        .map(|f| f.status.as_str())
}

#[test]
fn consumer_information_uses_content_index_and_accounts_for_incomplete_data() {
    let readings = || vec![record(&[0x01, 0x13], &[0])];
    for (cf, ext) in [(0x0703, 0x18), (0x2a00, 0x0d18)] {
        let water = report(0x07, cf, ext, RecordAvailability::Complete, readings());
        assert_eq!(
            outcome(&water, "Consumer water volume (VM1!)"),
            Some("passed")
        );
        assert_eq!(
            outcome(&water, "Consumer billing volume (VM1!D)"),
            Some("failed")
        );
        assert_eq!(
            outcome(&water, "Consumer information key identifier"),
            Some("passed")
        );
    }
    let encrypted = report(0x07, 0x0703, 0x11, RecordAvailability::Encrypted, vec![]);
    assert_eq!(
        outcome(&encrypted, "Consumer water volume (VM1!)"),
        Some("could_not_check")
    );
    assert_eq!(
        outcome(&encrypted, "Consumer information key identifier"),
        Some("failed")
    );
    let standard = report(0x07, 0x0702, 0x10, RecordAvailability::Complete, readings());
    assert_eq!(outcome(&standard, "Consumer water volume (VM1!)"), None);
}

#[test]
fn sensor_requirements_follow_each_declared_subdevice() {
    let designation = || record(&[0x0d, 0xfd, 0xfd, 0x01], &[0xe3, 5, 3, 1]);
    let humidity = || record(&[1, 0xfb, 0x1a], &[50]);
    let combined = report(
        0x1f,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![humidity(), designation()],
    );
    assert_eq!(outcome(&combined, "OMS sensor designation"), Some("passed"));
    assert_eq!(outcome(&combined, "Humidity (RH1!)"), Some("passed"));
    assert_eq!(outcome(&combined, "Temperature (TC4!)"), Some("failed"));
    assert_eq!(
        outcome(&combined, "Light level (LT1! or LT2!)"),
        Some("failed")
    );
    assert_eq!(
        outcome(&combined, "Standard error flags (MM3!)"),
        Some("failed")
    );
    let unknown = report(0x1f, 0x0702, 0x10, RecordAvailability::Partial, vec![]);
    assert_eq!(
        outcome(&unknown, "Sensor-specific data points"),
        Some("could_not_check")
    );
    let unlisted = report(
        0x1f,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![record(&[13, 0xfd, 0xfd, 1], &[0xe1, 4])],
    );
    assert_eq!(outcome(&unlisted, "OMS sensor designation"), Some("failed"));
}

#[test]
fn pressure_thresholds_are_required_in_static_messages() {
    let static_data = report(0x18, 0x8700, 0x10, RecordAvailability::Complete, vec![]);
    assert_eq!(
        outcome(&static_data, "Static upper pressure threshold (PR3!)"),
        Some("failed")
    );
    assert_eq!(outcome(&static_data, "Pressure state (MM13!)"), None);
    let dynamic = report(
        0x1f,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![record(&[13, 0xfd, 0xfd, 1], &[0xe1, 21])],
    );
    assert_eq!(
        outcome(&dynamic, "Static upper pressure threshold (PR3!)"),
        None
    );
    assert_eq!(outcome(&dynamic, "Pressure state (MM13!)"), Some("failed"));
    assert_eq!(outcome(&dynamic, "Standard error flags (MM3!)"), None);
}

#[test]
fn consumer_conditional_data_needs_evidence_of_the_device_function() {
    let unknown = report(2, 0x0703, 0x10, RecordAvailability::Complete, vec![]);
    for rule in [
        "Consumer export energy (EW2!)",
        "Consumer disconnector state (DC1!)",
    ] {
        assert_eq!(outcome(&unknown, rule), Some("could_not_check"));
    }
    let mut physical = record(&[0x81, 0x40, 0xfd, 0x1a], &[1]).unwrap();
    physical.subunit = 1;
    let mut export_due = record(&[0x41, 0x80, 0x3c], &[1]).unwrap();
    export_due.storage = 1;
    let known = report(
        2,
        0x0703,
        0x10,
        RecordAvailability::Complete,
        vec![Some(physical), Some(export_due)],
    );
    for rule in [
        "Consumer export energy (EW2!)",
        "Consumer disconnector state (DC1!)",
    ] {
        assert_eq!(outcome(&known, rule), Some("failed"));
    }
}
