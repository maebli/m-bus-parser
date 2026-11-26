#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

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

fn report(
    device: u8,
    cf: u16,
    ext: u16,
    availability: RecordAvailability,
    records: Vec<Option<Reading<&[u8]>>>,
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
        nominal_transmission_interval_ms: Some(6000),
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
    assert_eq!(outcome(&all, "TAF7 measurement count"), Some("failed"));
    let mut right_count = records();
    right_count[0].as_mut().unwrap().data = &[7, 0x41, 6, 1, 2, 3, 4, 5];
    let sized = parse(Some(parameters), right_count, RecordAvailability::Complete);
    assert_eq!(outcome(&sized, "TAF7 measurement count"), Some("passed"));
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

#[test]
fn descriptor_index_selects_its_register_and_rejects_contradictory_definitions() {
    let make = |data: &'static [u8]| {
        let mut reading = record(&[0x81, 0x40, 0xfd, 0x23], data).unwrap();
        reading.subunit = 1;
        Some(reading)
    };
    let water = report(
        7,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![make(&[1]), make(&[9])],
    );
    assert_eq!(
        outcome(&water, "Descriptor index for medium"),
        Some("failed")
    );
    assert_eq!(
        outcome(&water, "Stable descriptor definition"),
        Some("failed")
    );
    let malformed = report(
        7,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![make(&[128])],
    );
    assert_eq!(
        outcome(&malformed, "Descriptor register address"),
        Some("failed")
    );
}

#[test]
fn readout_list_actions_allow_the_final_compact_profile_modifier() {
    let regs = [0x00, 0x93, 0x8c, 0x13];
    let request = telegram::assess_telegram(
        Link::Wireless(0x53),
        None,
        LayerFields {
            application_ci: Some(0x51),
            ..Default::default()
        },
        RecordAvailability::Complete,
        vec![record(&regs, &[])],
    );
    assert_eq!(outcome(&request, "Command encoding"), Some("passed"));
    let request = telegram::assess_telegram(
        Link::Wireless(0x53),
        None,
        LayerFields {
            application_ci: Some(0x51),
            ..Default::default()
        },
        RecordAvailability::Complete,
        vec![record(&[1, 0x93, 0x8c, 0x13], &[0])],
    );
    assert_eq!(outcome(&request, "Command encoding"), Some("failed"));
}

#[test]
fn electricity_maximum_register_pairs_require_the_same_direction_and_tariff() {
    let power = || record(&[0x11, 0x28], &[1]);
    let date = || record(&[0x12, 0x6c], &[1, 1]);
    let matched = report(
        2,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![power(), date()],
    );
    assert_eq!(
        outcome(&matched, "Power maximum and time of maximum"),
        Some("passed")
    );
    let wrong_direction = report(
        2,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![power(), record(&[0x12, 0xec, 0x3c], &[1, 1])],
    );
    assert!(wrong_direction
        .findings
        .iter()
        .filter(|f| f.rule == "Power maximum and time of maximum")
        .all(|f| f.status.as_str() == "failed"));
    let mut tariff = date().unwrap();
    tariff.tariff = 1;
    let wrong_tariff = report(
        2,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![power(), Some(tariff)],
    );
    assert_eq!(
        outcome(&wrong_tariff, "Power maximum and time of maximum"),
        Some("failed")
    );
    let partial = report(2, 0x0702, 0x10, RecordAvailability::Partial, vec![power()]);
    assert_eq!(
        outcome(&partial, "Power maximum and time of maximum"),
        Some("could_not_check")
    );
}

#[test]
fn ambiguous_protection_reports_missing_context() {
    use m_bus_oms_rules::readings::RecordProtection;
    let mut timestamp = record(&[0xc4, 1, 0x6d], &[0; 4]).unwrap();
    timestamp.storage = 3;
    timestamp.protection = Some(RecordProtection::Clear);
    let ambiguous = report(
        0x1f,
        0x0702,
        0x10,
        RecordAvailability::Complete,
        vec![Some(timestamp)],
    );
    assert_eq!(
        outcome(&ambiguous, "Data point encryption"),
        Some("could_not_check")
    );
}

#[test]
fn capture_keeps_immutable_identifiers_and_static_billing_dates_per_device() {
    use m_bus_parser::{
        decode_hex,
        output::{render_oms_capture, OmsFormat},
        DecodeOptions,
    };
    let frame = |sender, body: &str| {
        let packet = format!("{:02X}44{sender}{body}", 9 + body.len() / 2);
        decode_hex(&packet, &DecodeOptions::default()).unwrap()
    };
    let sensor = "9315785634123307";
    let other = "9315785634133307";
    let first = "7A010000000D780E3434333332323131313053445137";
    let second = "7A020000000D780E3534333332323131313053445137";
    let frames = [
        frame(sensor, first),
        frame(other, second),
        frame(sensor, second),
    ];
    let report: serde_json::Value =
        serde_json::from_str(&render_oms_capture(&frames, OmsFormat::Json, None, true).unwrap())
            .unwrap();
    let status = |report: &serde_json::Value, name| {
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rule"] == name)
            .map(|f| f["status"].clone())
    };
    assert!(status(&report[1], "Initial DIN address across capture").is_none());
    assert_eq!(
        status(&report[2], "Initial DIN address across capture"),
        Some("failed".into())
    );
    let due = [
        frame(sensor, "7A01000805426C1F11"),
        frame(other, "7A02000000411301"),
        frame(sensor, "7A02000000411301"),
    ];
    let report: serde_json::Value =
        serde_json::from_str(&render_oms_capture(&due, OmsFormat::Json, None, true).unwrap())
            .unwrap();
    assert_eq!(
        status(&report[1], "Due date for billing value"),
        Some("could_not_check".into())
    );
    assert_eq!(
        status(&report[2], "Due date for billing value"),
        Some("passed".into())
    );
}

#[test]
fn application_selection_must_precede_list_edits_and_descriptors_stay_in_blocks_zero_or_one() {
    let select = || record(&[1, 0xfd, 0xfd, 0x80, 0], &[0x70]);
    let add = || record(&[0, 0xfd, 0xbc, 0x0c], &[]);
    let run = |readings| {
        telegram::assess_telegram(
            Link::Wireless(0x53),
            None,
            LayerFields {
                application_ci: Some(0x51),
                ..Default::default()
            },
            RecordAvailability::Complete,
            readings,
        )
    };
    assert_eq!(
        outcome(
            &run(vec![select(), add()]),
            "Readout-list application selection order"
        ),
        Some("passed")
    );
    assert_eq!(
        outcome(
            &run(vec![add(), select()]),
            "Readout-list application selection order"
        ),
        Some("failed")
    );
    let selected = |block| {
        report(
            7,
            0x0702,
            0x10,
            RecordAvailability::Complete,
            vec![
                record(&[1, 0xfd, 0xfd, 0], block),
                record(&[1, 0xfd, 0x23], &[0]),
            ],
        )
    };
    assert_eq!(
        outcome(&selected(&[0x71]), "Static application descriptor block"),
        Some("passed")
    );
    assert_eq!(
        outcome(&selected(&[0x72]), "Static application descriptor block"),
        Some("failed")
    );
}

#[test]
fn protocol_envelopes_check_transmitted_lengths_directions_and_checksums() {
    use m_bus_parser::{decode_hex, DecodeOptions};
    let run = |ci: u8, payload: &[u8]| {
        let body = payload
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>();
        let packet = format!(
            "{:02X}449315785634123307{ci:02X}01000000{body}",
            14 + payload.len()
        );
        decode_hex(&packet, &DecodeOptions::default())
            .unwrap()
            .assess_oms_5_0_1()
    };
    let mut sml = vec![0x76, 2, 1, 0x62, 1, 0x62, 0, 0x72, 0x63, 1, 1, 0x70];
    let crc = crc16::State::<crc16::X_25>::calculate(&sml);
    sml.push(0x63);
    sml.extend(crc.to_le_bytes());
    sml.push(0);
    assert_eq!(
        outcome(&run(0x7f, &sml), "SML message structure and checksum"),
        Some("passed")
    );
    sml[2] ^= 1;
    assert_eq!(
        outcome(&run(0x7f, &sml), "SML message structure and checksum"),
        Some("failed")
    );
    let mut aare = vec![
        0x10, 1, 0x61, 15, 0xa1, 3, 6, 1, 42, 0xa2, 3, 2, 1, 0, 0xa3, 3, 2, 1, 0,
    ];
    let good = run(0x7d, &aare);
    assert_eq!(
        outcome(&good, "DLMS association BER length"),
        Some("passed")
    );
    assert_eq!(outcome(&good, "DLMS association fields"), Some("passed"));
    aare[2] = 0x60;
    assert_eq!(
        outcome(&run(0x7d, &aare), "DLMS association direction"),
        Some("failed")
    );
    aare[3] = 30;
    assert_eq!(
        outcome(&run(0x7d, &aare), "DLMS association BER length"),
        Some("failed")
    );
}

#[test]
fn published_sml_response_and_framed_sml_files_have_valid_structure() {
    use m_bus_parser::{decode_hex, DecodeOptions};
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/oms-annex-n.json")).unwrap();
    let case = fixtures
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case["pages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|page| page == "3.0.1/94")
        })
        .unwrap();
    let packet = decode_hex(
        case["frames"][0].as_str().unwrap(),
        &DecodeOptions {
            key: Some(
                hex::decode(case["key"].as_str().unwrap())
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let published = packet.assess_oms_5_0_1();
    assert_eq!(
        outcome(&published, "SML message structure and checksum"),
        Some("passed")
    );
    assert_eq!(
        outcome(&published, "SML message body schema"),
        Some("passed")
    );

    let message = |tag: u16, fields: &[u8]| {
        let mut data = vec![0x76, 2, 1, 0x62, 0, 0x62, 0, 0x72, 0x63];
        data.extend(tag.to_be_bytes());
        data.extend_from_slice(fields);
        let crc = crc16::State::<crc16::X_25>::calculate(&data);
        data.push(0x63);
        data.extend(crc.to_le_bytes());
        data.push(0);
        data
    };
    let mut frame = vec![0x1b, 0x1b, 0x1b, 0x1b, 1, 1, 1, 1];
    frame.extend(message(0x0101, &[0x76, 1, 1, 2, 0x42, 2, 0x24, 1, 1]));
    frame.extend(message(0x0201, &[0x71, 1]));
    let padding = (4 - frame.len() % 4) % 4;
    frame.resize(frame.len() + padding, 0);
    frame.extend([0x1b, 0x1b, 0x1b, 0x1b, 0x1a, padding as u8]);
    frame.extend(crc16::State::<crc16::X_25>::calculate(&frame).to_le_bytes());
    let assess = |data: &[u8]| {
        decode_hex(
            &format!(
                "{:02X}4493157856341233077F01000000{}",
                14 + data.len(),
                hex::encode(data)
            ),
            &DecodeOptions::default(),
        )
        .unwrap()
        .assess_oms_5_0_1()
    };
    let result = assess(&frame);
    for rule in [
        "SML version 1 transport",
        "SML file open and close",
        "SML message body schema",
        "SML message structure and checksum",
    ] {
        assert_eq!(outcome(&result, rule), Some("passed"), "{rule}");
    }
    *frame.last_mut().unwrap() ^= 1;
    assert_eq!(
        outcome(&assess(&frame), "SML version 1 transport"),
        Some("failed")
    );
}
