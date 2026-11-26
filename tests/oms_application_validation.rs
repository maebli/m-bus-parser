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

#[test]
fn taf7_checks_registers_and_timing_only_for_a_declared_use_case() {
    use m_bus_oms_rules::readings::Taf7Parameters;
    let at = |header: &'static [u8], data: &'static [u8], storage| {
        let mut reading = record(header, data).unwrap();
        reading.storage = storage;
        Some(reading)
    };
    let records = || {
        vec![
            at(&[0xcd, 1, 0x93, 0x13], &[4, 0x41, 6, 1, 2], 3),
            at(&[0xc4, 1, 0x6d], &[0x80, 0, 0, 0], 3),
            at(&[0xc1, 1, 0x74], &[2], 3),
            at(&[0xc1, 1, 0x13], &[8], 3),
        ]
    };
    let parse = |parameters, records, availability| {
        telegram::assess_telegram_with_taf7(
            Link::Wireless(0x44),
            None,
            LayerFields::default(),
            availability,
            records,
            parameters,
        )
    };
    let parameters = Taf7Parameters {
        measuring_period_ms: Some(3000),
        acceptance_window_ms: Some(9000),
        registration_period_ms: Some(900_000),
    };
    let all = parse(Some(parameters), records(), RecordAvailability::Complete);
    for rule in [
        "TAF7 profile direction",
        "TAF7 profile storage number",
        "TAF7 differential base value",
        "TAF7 base time",
        "TAF7 actuality duration",
        "TAF7 profile interval and measuring period",
        "TAF7 profile interval and acceptance window",
        "TAF7 measuring period and acceptance window",
        "TAF7 acceptance window and registration period",
    ] {
        assert_eq!(outcome(&all, rule), Some("passed"), "{rule}");
    }
    let mut wrong_register = records();
    wrong_register[3].as_mut().unwrap().tariff = 1;
    let wrong = parse(
        Some(Taf7Parameters {
            measuring_period_ms: Some(4000),
            ..parameters
        }),
        wrong_register,
        RecordAvailability::Complete,
    );
    assert_eq!(
        outcome(&wrong, "TAF7 differential base value"),
        Some("failed")
    );
    assert_eq!(
        outcome(&wrong, "TAF7 profile interval and measuring period"),
        Some("failed")
    );
    let no_context = parse(None, records(), RecordAvailability::Complete);
    assert_eq!(outcome(&no_context, "TAF7 profile direction"), None);
    let encrypted = parse(
        Some(Default::default()),
        vec![],
        RecordAvailability::Encrypted,
    );
    assert_eq!(
        outcome(&encrypted, "TAF7 inverse compact profile"),
        Some("could_not_check")
    );
}

#[test]
fn compact_profile_checks_ignore_primary_vif_and_validate_complete_elements() {
    let collision = report(
        7,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![
            record(&[1, 0x13], &[0]),
            record(&[1, 0xfd, 0xfd, 0x13], &[0]),
        ],
    );
    assert_eq!(outcome(&collision, "Compact profile element length"), None);
    let bad = report(
        7,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![record(&[0xd, 0x93, 0x93, 0x20], &[3, 2, 6, 1])],
    );
    assert_eq!(
        outcome(&bad, "Compact profile modifier position"),
        Some("failed")
    );
    assert_eq!(
        outcome(&bad, "Compact profile element length"),
        Some("failed")
    );
}
