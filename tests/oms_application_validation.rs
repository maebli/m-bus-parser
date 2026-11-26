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
