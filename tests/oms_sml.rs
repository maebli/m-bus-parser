#![cfg(all(feature = "std", feature = "sml"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_oms_rules::telegram::TelegramReport;

fn outcome<'a>(report: &'a TelegramReport, name: &str) -> Option<&'a str> {
    report
        .findings
        .iter()
        .find(|f| f.rule == name)
        .map(|f| f.status.as_str())
}

#[cfg(feature = "decryption-mode-5")]
#[test]
fn published_sml_response() {
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
}

#[test]
fn framed_sml_files_have_valid_structure() {
    use m_bus_parser::{decode_hex, DecodeOptions};
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
    let decode = |data: &[u8]| {
        decode_hex(
            &format!(
                "{:02X}4493157856341233077F01000000{}",
                14 + data.len(),
                hex::encode(data)
            ),
            &DecodeOptions::default(),
        )
        .unwrap()
    };
    let packet = decode(&frame);
    let decoded = packet
        .application
        .as_ref()
        .unwrap()
        .decoded
        .as_ref()
        .unwrap();
    assert_eq!(decoded["messages"].as_array().unwrap().len(), 2);
    let result = packet.assess_oms_5_0_1();
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
        outcome(
            &decode(&frame).assess_oms_5_0_1(),
            "SML version 1 transport"
        ),
        Some("failed")
    );
}

#[test]
fn sml_version_two_negotiation_checks_acknowledgements_and_limits() {
    use m_bus_parser::{
        decode_bytes,
        output::{render_oms_capture, OmsFormat},
        DecodeOptions,
    };
    let packet = |control: u8, block: u8, timeout: u16, size: u16| {
        let mut wire = vec![0x1b, 0x1b, 0x1b, 0x1b, 2, block, 3, 1];
        wire.extend([0x1b, 0x1b, 0x1b, 0x1b, 3, 0]);
        wire.extend(timeout.to_be_bytes());
        wire.extend([0x1b, 0x1b, 0x1b, 0x1b, 4, 0]);
        wire.extend(size.to_be_bytes());
        wire.extend([0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0]);
        wire.extend(crc16::State::<crc16::X_25>::calculate(&wire).to_le_bytes());
        let mut body = vec![
            control,
            4,
            if control == 0x53 { 0x65 } else { 0x7f },
            0,
            0,
            0,
            0,
        ];
        body.extend(wire);
        let len = body.len() as u8;
        let check = body.iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
        let mut frame = vec![0x68, len, len, 0x68];
        frame.extend(body);
        frame.extend([check, 0x16]);
        decode_bytes(&frame, &DecodeOptions::default()).unwrap()
    };
    let check = |timeout, size| {
        let reports = render_oms_capture(
            &[packet(0x53, 0, 5000, 64), packet(8, 0x80, timeout, size)],
            OmsFormat::Json,
            None,
            true,
        )
        .unwrap();
        serde_json::from_str::<serde_json::Value>(&reports).unwrap()
    };
    for (timeout, size, result) in [
        (5000, 32, "passed"),
        (2000, 64, "failed"),
        (5000, 96, "failed"),
    ] {
        let report = check(timeout, size);
        let find = |rule| {
            report[1]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["rule"] == rule)
                .map(|f| f["status"].as_str().unwrap())
        };
        assert_eq!(find("SML version 2 block"), Some("passed"));
        assert_eq!(find("SML block acknowledgement"), Some("passed"));
        assert_eq!(find("SML negotiated transfer limits"), Some(result));
    }
}

#[test]
fn sml_version_two_checks_the_file_across_retries_and_missing_blocks() {
    use m_bus_parser::{
        decode_bytes,
        output::{render_oms_capture, OmsFormat},
        DecodeOptions,
    };
    fn packet(control: u8, payload: &[u8]) -> m_bus_parser::output::DecodedOutput {
        let mut wire = vec![0x1b, 0x1b, 0x1b, 0x1b, 2, control, 9, 1];
        if control == 0 {
            wire.extend([
                0x1b, 0x1b, 0x1b, 0x1b, 3, 0, 0x13, 0x88, 0x1b, 0x1b, 0x1b, 0x1b, 4, 0, 0, 64,
            ]);
        }
        wire.extend_from_slice(payload);
        let padding = (4 - wire.len() % 4) % 4;
        wire.resize(wire.len() + padding, 0);
        wire.extend([0x1b, 0x1b, 0x1b, 0x1b, 0x1a, padding as u8]);
        wire.extend(crc16::State::<crc16::X_25>::calculate(&wire).to_le_bytes());
        let mut body = vec![8, 4, 0x7f, 0, 0, 0, 0];
        body.extend(wire);
        let len = body.len() as u8;
        let mut frame = vec![0x68, len, len, 0x68];
        let checksum = body.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        frame.extend(body);
        frame.extend([checksum, 0x16]);
        decode_bytes(&frame, &DecodeOptions::default()).unwrap()
    }
    fn message(tag: u16, fields: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x76, 2, 1, 0x62, 0, 0x62, 0, 0x72, 0x63];
        bytes.extend(tag.to_be_bytes());
        bytes.extend(fields);
        let crc = crc16::State::<crc16::X_25>::calculate(&bytes);
        bytes.push(0x63);
        bytes.extend(crc.to_le_bytes());
        bytes.push(0);
        bytes
    }
    let mut file = message(0x0101, &[0x76, 1, 1, 2, 0x42, 2, 0x24, 1, 1]);
    file.extend(message(0x0201, &[0x71, 1]));
    let verify = |last, bytes: &[u8]| {
        let (first, tail) = bytes.split_at(12);
        let frames = [
            packet(0, &[]),
            packet(1, first),
            packet(1, first),
            packet(last, tail),
        ];
        let output = render_oms_capture(&frames, OmsFormat::Json, Some(3), true).unwrap();
        serde_json::from_str::<serde_json::Value>(&output).unwrap()
    };
    let status = |result: &serde_json::Value, rule: &str| {
        result["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["rule"] == rule)
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let good = verify(0x42, &file);
    for rule in [
        "Reassembled SML message checksums",
        "Reassembled SML message bodies",
        "Reassembled SML file open and close",
    ] {
        assert_eq!(status(&good, rule), "passed");
    }
    assert_eq!(
        status(&verify(0x43, &file), "Reassembled SML message checksums"),
        "could_not_check"
    );
    file[2] ^= 1;
    assert_eq!(
        status(&verify(0x42, &file), "Reassembled SML message checksums"),
        "failed"
    );
}

fn sml_message(tag: u16, fields: &[u8], group: u8, abort: u8) -> Vec<u8> {
    let mut message = vec![0x76, 2, 1, 0x62, group, 0x62, abort, 0x72, 0x63];
    message.extend(tag.to_be_bytes());
    message.extend_from_slice(fields);
    let crc = crc16::State::<crc16::X_25>::calculate(&message);
    message.push(0x63);
    message.extend(crc.to_le_bytes());
    message.push(0);
    message
}

fn assess_sml(bytes: &[u8]) -> TelegramReport {
    use m_bus_parser::{decode_hex, DecodeOptions};
    let hex = format!(
        "{:02X}4493157856341233077F01000000{}",
        14 + bytes.len(),
        hex::encode(bytes)
    );
    decode_hex(&hex, &DecodeOptions::default())
        .unwrap()
        .assess_oms_5_0_1()
}

#[test]
fn sml_checks_recursive_trees_local_times_and_cosem_descriptors() {
    let check = |tag, fields: &[u8]| {
        let report = assess_sml(&sml_message(tag, fields, 1, 0));
        outcome(&report, "SML message body schema")
            .unwrap()
            .to_owned()
    };
    let mut tree = vec![
        0x73, 1, 0x71, 2, 1, 0x73, 2, 1, 1, 0x71, 0x73, 2, 2, 0x72, 0x62, 4, 0x72, 0x62, 3, 0x73,
        0x62, 100, 0x52, 1, 0x52, 2, 1,
    ];
    assert_eq!(check(0x0501, &tree), "passed");
    tree[22] = 0x62;
    assert_eq!(check(0x0501, &tree), "failed");
    tree[22] = 0x52;
    tree[15] = 7;
    assert_eq!(check(0x0501, &tree), "failed");
    let mut get = vec![
        0x78, 1, 1, 1, 1, 1, 0x52, 1, 0x52, 1, 0x71, 0x72, 0x52, 3, 0x72, 0x62, 1, 0x71, 0x62, 1,
    ];
    assert_eq!(check(0x0800, &get), "passed");
    get[12] = 0x62;
    assert_eq!(check(0x0800, &get), "failed");
    let action = [0x79, 1, 1, 1, 1, 1, 0x52, 1, 0x52, 0, 0x62, 8, 0x52, 5];
    assert_eq!(check(0x0a00, &action), "passed");
    let mut time_list = vec![
        0x77, 1, 1, 1, 1, 0x71, 0x77, 2, 1, 1, 1, 1, 1, 0x72, 0x62, 1, 0x72, 0x62, 2, 0x62, 20, 1,
        1, 1,
    ];
    assert_eq!(check(0x0701, &time_list), "passed");
    time_list[15] = 5;
    assert_eq!(check(0x0701, &time_list), "failed");
}

#[test]
fn sml_profile_periods_match_the_headers_and_keep_seconds_in_order() {
    let mut body = vec![
        0x78, 1, 0x72, 0x62, 1, 0x62, 20, 0x62, 15, 0x71, 2, 1, 0x71, 0x73, 2, 1, 0x62, 2, 0x52, 1,
        0x72,
    ];
    let period = |seconds| {
        [
            0x74, 0x72, 0x62, 1, 0x62, seconds, 0x62, 0, 0x71, 0x72, 0x52, 4, 1, 1,
        ]
    };
    body.extend(period(12));
    body.extend(period(18));
    body.extend([1, 1]);
    let status = |data: &[u8]| {
        outcome(
            &assess_sml(&sml_message(0x0301, data, 0, 0)),
            "SML message body schema",
        )
        .unwrap()
        .to_owned()
    };
    assert_eq!(status(&body), "passed");
    body[40] = 10;
    assert_eq!(status(&body), "failed");
    body[40] = 18;
    body.splice(12..20, [0x70]);
    assert_eq!(status(&body), "failed");
}

#[test]
fn sml_accepts_attention_and_combined_files_and_checks_group_and_address_consistency() {
    let file = |messages: &[Vec<u8>]| {
        let mut bytes = vec![0x1b, 0x1b, 0x1b, 0x1b, 1, 1, 1, 1];
        for message in messages {
            bytes.extend(message);
        }
        let pad = (4 - bytes.len() % 4) % 4;
        bytes.resize(bytes.len() + pad, 0);
        bytes.extend([0x1b, 0x1b, 0x1b, 0x1b, 0x1a, pad as u8]);
        bytes.extend(crc16::State::<crc16::X_25>::calculate(&bytes).to_le_bytes());
        assess_sml(&bytes)
    };
    let open = || sml_message(0x0100, &[0x77, 1, 2, 1, 2, 8, 2, 3, 1, 1, 1], 1, 0);
    let get = |addressed| {
        if addressed {
            sml_message(0x0700, &[0x75, 2, 1, 2, 3, 1, 1, 1], 2, 1)
        } else {
            sml_message(0x0700, &[0x75, 2, 1, 1, 1, 1, 1], 2, 1)
        }
    };
    let close = |group| sml_message(0x0200, &[0x71, 1], group, 0);
    let attention = || sml_message(0xff01, &[0x74, 2, 3, 2, 1, 1, 1], 4, 0);
    let response = || sml_message(0x0201, &[0x71, 1], 5, 0);
    let combined = file(&[open(), get(true), close(3), attention(), response()]);
    for name in [
        "SML file open and close",
        "SML file message groups",
        "SML file request addressing",
    ] {
        assert_eq!(outcome(&combined, name), Some("passed"));
    }
    let mixed = file(&[open(), get(false), close(1)]);
    assert_eq!(outcome(&mixed, "SML file message groups"), Some("failed"));
    assert_eq!(
        outcome(&mixed, "SML file request addressing"),
        Some("failed")
    );
    assert_eq!(
        outcome(
            &file(&[open(), open(), close(3)]),
            "SML file open and close"
        ),
        Some("failed")
    );
    assert_eq!(
        outcome(
            &assess_sml(&sml_message(0x0200, &[0x71, 1], 2, 5)),
            "SML message structure and checksum"
        ),
        Some("failed")
    );
}
