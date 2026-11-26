#![cfg(all(feature = "std", feature = "oms-gen2"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{decode_bytes, decode_fragments, DecodeOptions, DecodedOutput, OutputError};
use serde::Deserialize;

#[derive(Deserialize)]
struct Example {
    pages: Vec<String>,
    key: String,
    frames: Vec<String>,
    expected: String,
    records: Option<usize>,
}

impl Example {
    fn frames(&self) -> Vec<Vec<u8>> {
        self.frames
            .iter()
            .map(|frame| hex::decode(frame).unwrap())
            .collect()
    }

    fn options(&self) -> DecodeOptions {
        DecodeOptions {
            key: (!self.key.is_empty())
                .then(|| hex::decode(&self.key).unwrap().try_into().unwrap()),
            ..DecodeOptions::default()
        }
    }

    fn decode(&self) -> Result<DecodedOutput, OutputError> {
        let frames = self.frames();
        let options = self.options();
        match frames.as_slice() {
            [frame] => decode_bytes(frame, &options),
            _ => decode_fragments(
                &frames.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                &options,
            ),
        }
    }
}

fn examples() -> Vec<Example> {
    serde_json::from_str(include_str!("fixtures/oms-annex-n.json")).unwrap()
}

fn enabled(page: &str) -> bool {
    let (edition, _) = page.split_once('/').unwrap();
    [
        ("3.0.1", cfg!(feature = "oms-gen3")),
        ("4.0.2", cfg!(feature = "oms-gen4-0")),
        ("4.1.2", cfg!(feature = "oms-gen4-1")),
        ("4.2.1", cfg!(feature = "oms-gen4-2")),
        ("4.3.3", cfg!(feature = "oms-gen4-3")),
        ("4.4.2", cfg!(feature = "oms-gen4-4")),
        ("4.5.1", cfg!(feature = "oms-gen4-5")),
        ("5.0.1", cfg!(feature = "oms-gen5")),
    ]
    .contains(&(edition, true))
}

#[test]
fn published_frames_for_each_enabled_edition() {
    for example in examples()
        .into_iter()
        .filter(|e| e.pages.iter().any(|p| enabled(p)))
    {
        let result = example.decode();
        let pages = &example.pages;
        match example.expected.as_str() {
            "complete" => {
                assert!(result.is_ok(), "{pages:?} {:?}", result.as_ref().err());
                let output = result.unwrap();
                assert_eq!(
                    output.decode_state, "complete",
                    "{pages:?} {:?}",
                    output.diagnostics
                );
                assert_eq!(Some(output.records.len()), example.records, "{pages:?}");
                if example.frames.len() > 1 {
                    assert_eq!(
                        output.records[0].value.value,
                        Some("411.979".into()),
                        "{pages:?}"
                    );
                    for frame in example.frames() {
                        let fragment = decode_bytes(&frame, &example.options()).unwrap();
                        assert_eq!(
                            fragment.security.decryption_state, "awaiting_fragments",
                            "{pages:?}"
                        );
                        assert!(fragment.records.is_empty());
                    }
                }
            }
            "incomplete" => assert!(
                !result.is_ok_and(|v| v.decode_state == "complete"),
                "{pages:?}"
            ),
            code => assert_eq!(result.unwrap_err().code(), code, "{pages:?}"),
        }
    }
}

#[cfg(feature = "oms-gen3")]
#[test]
fn gen3_sml_entries_keep_the_obis_code_and_scale() {
    let output = examples()
        .into_iter()
        .find(|e| e.pages.iter().any(|p| p == "3.0.1/94"))
        .unwrap()
        .decode()
        .unwrap();
    let value = output.application.unwrap().decoded.unwrap();
    let entries = &value["messages"][0]["body"]["entries"];
    assert_eq!(entries[2]["object_name_hex"], "0100010801FF");
    assert_eq!(entries[2]["scaler"], -1);
    assert_eq!(entries[2]["value"], "2.1");
}

#[cfg(feature = "oms-gen5")]
#[test]
fn gen5_electricity_profiles_and_breaker_status_match_annex_n() {
    let cases = examples();
    let output = |page: &str| {
        cases
            .iter()
            .find(|e| e.pages.iter().any(|p| p == page))
            .unwrap()
            .decode()
            .unwrap()
    };
    let readings = output("5.0.1/97").records;
    let json = serde_json::to_value(readings).unwrap();
    assert_eq!(json[2]["value"]["value"]["ticks"], "3");
    for (index, first, second, last) in [(4, "1994", "1991", "1872"), (6, "305", "305", "215")] {
        let profile = &json[index]["value"]["value"];
        assert_eq!(profile["spacing"], 8);
        assert_eq!(profile["spacing_unit"], "second");
        assert_eq!(profile["values"].as_array().unwrap().len(), 46);
        assert_eq!(profile["values"][0], first);
        assert_eq!(profile["values"][1], second);
        assert_eq!(profile["values"][45], last);
    }
    for page in ["5.0.1/87", "5.0.1/91"] {
        let block = output(page).application.unwrap().decoded.unwrap();
        assert_eq!(block["records"][0]["quantities"][0], "Remote control");
        assert_eq!(block["key_counter"], 15);
        assert_eq!(block["application_mac_verified"], false);
    }
    let breaker = cases
        .iter()
        .find(|e| e.pages.iter().any(|p| p == "5.0.1/87"))
        .unwrap();
    let bytes = breaker.frames();
    let key = [0x08, 0x15, 0x47, 0x11].repeat(4).try_into().unwrap();
    let verified = m_bus_parser::output::decode_bytes_with_application_key(
        &bytes[0],
        &breaker.options(),
        &key,
    )
    .unwrap();
    assert_eq!(
        verified.application.unwrap().decoded.unwrap()["application_mac_verified"],
        true
    );
    assert_eq!(
        m_bus_parser::output::decode_bytes_with_application_key(
            &bytes[0],
            &breaker.options(),
            &[0; 16]
        )
        .unwrap_err()
        .code(),
        "security.application_authentication_failed"
    );
    assert_eq!(
        output("5.0.1/94").application.unwrap().decoded.unwrap()["status_code"],
        0x19
    );
}

#[cfg(feature = "oms-gen5")]
#[test]
fn fragment_exchanges_reject_loss_reordering_and_wrong_keys() {
    for example in examples().into_iter().filter(|e| e.frames.len() > 1) {
        let buffers = example.frames();
        let [a, b, c]: &[Vec<u8>; 3] = buffers.as_slice().try_into().unwrap();
        let options = example.options();
        for frames in [[a, b, b], [a, c, b], [b, b, c]] {
            assert!(decode_fragments(&frames.map(Vec::as_slice), &options).is_err());
        }
        assert!(decode_fragments(&[a, c], &options).is_err());
        let output = example.decode().unwrap();
        let inputs = [a.as_slice(), b.as_slice(), c.as_slice()];
        let missing = decode_fragments(&inputs, &DecodeOptions::default()).unwrap();
        assert!(missing.records.is_empty());
        let wrong = DecodeOptions {
            key: Some([0x55; 16]),
            ..options
        };
        let expected = if output.security.decryption_state == "decrypted_authenticated" {
            "security.authentication_failed"
        } else {
            "security.decryption_failed"
        };
        assert_eq!(
            decode_fragments(&inputs, &wrong).unwrap_err().code(),
            expected
        );
    }
}

#[test]
fn elster_oms_2_0_0_annex_b() {
    const COMMANDS: [&str; 7] = [
        "1040014116",
        "105A015B16",
        "105B015C16",
        "680303685301BB0F16",
        "680303685301B80C16",
        "68030368530150A416",
        "680B0B6853FD5278563412931533039416",
    ];
    const GAS_READINGS: [(&str, &[&str]); 3] = [
        (
            "681515680801727856341293153303010000000C1330120000CF16",
            &["1.23"],
        ),
        (
            "681E1E680801727856341293153303010000000DFD110542413332310C13301200000816",
            &["123AB", "1.23"],
        ),
        (
            "681A1A680801727856341293153303010400000C943A301200000274980DA916",
            &["12.3", "3480"],
        ),
    ];
    let options = DecodeOptions::default();
    for hex in COMMANDS {
        let frame = m_bus_parser::decode_hex(hex, &options).unwrap();
        assert_eq!(frame.decode_state, "complete", "{hex}");
        assert!(frame.records.is_empty());
    }
    for (hex, expected) in GAS_READINGS {
        let frame = m_bus_parser::decode_hex(hex, &options).unwrap();
        assert_eq!(frame.decode_state, "complete");
        let values = frame
            .records
            .iter()
            .map(|r| r.value.value.as_ref().unwrap().as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values, expected);
    }
}

#[cfg(feature = "oms-gen5")]
#[test]
fn annex_b_registrations_keep_their_encoding_and_storage_identity() {
    use m_bus_parser::output::{render_application_hex, OmsFormat};
    use m_bus_parser::{OutputFormat, RenderOptions};

    let report = |hex| {
        let output = render_application_hex(
            hex,
            OutputFormat::Oms(OmsFormat::Json),
            &RenderOptions::default(),
        )
        .unwrap();
        serde_json::from_str::<serde_json::Value>(&output).unwrap()
    };
    let examples = [
        (
            "78C28200FB007B00CB820006002301",
            "Unique Annex B readings",
            "failed",
            2,
        ),
        ("7805130000803F", "VM1!", "failed", 1),
        ("78036D001010010001", "DT5!", "passed", 1),
        ("78036D001010010001", "EW1!", "passed", 2),
        ("7840FD20", "Storage range endpoint", "passed", 1),
        ("7840FD21", "YD3!P", "passed", 1),
        ("7841FD2001", "Storage range endpoint", "failed", 1),
        ("7801FD2401", "Storage descriptor address", "failed", 1),
        ("78C140FD2401", "Storage descriptor address", "failed", 1),
        ("7841FD2401", "Storage descriptor address", "passed", 1),
        (
            "780D780E3434333332323131313053445137",
            "DIN manufacturer code",
            "passed",
            1,
        ),
        (
            "780D780E3434333332323131313073445137",
            "DIN manufacturer code",
            "failed",
            1,
        ),
        ("5140900C", "Command encoding", "passed", 1),
        ("5141900C01", "Command encoding", "failed", 1),
    ];
    for (hex, rule, status, record) in examples {
        let output = report(hex);
        let rows = output["findings"].as_array().unwrap();
        let finding = rows.iter().find(|f| f["rule"] == rule).unwrap();
        assert_eq!(finding["status"], status, "{hex}");
        assert_eq!(finding["record_number"], record);
    }
    let result = report("78041301000000");
    let tag = result["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == "VM1!")
        .unwrap();
    assert!(tag["source_url"]
        .as_str()
        .unwrap()
        .ends_with("AnnexB_G461.pdf#page=31&view=FitH,525"));
}

#[cfg(feature = "oms-gen5")]
#[test]
fn annex_n_validation_identifies_the_documented_breaker_and_address_discrepancies() {
    for example in examples().iter().filter(|e| {
        e.expected == "complete" && e.pages.iter().any(|page| page.starts_with("5.0.1/"))
    }) {
        let report = example.decode().unwrap().assess_oms_5_0_1();
        let failures: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.status.as_str() == "failed")
            .map(|f| f.rule)
            .collect();
        if example.pages.iter().any(|p| p == "5.0.1/83") {
            assert_eq!(failures, ["Physical state (DC2!)"]);
        } else if example
            .pages
            .iter()
            .any(|p| matches!(p.as_str(), "5.0.1/9" | "5.0.1/65"))
        {
            assert_eq!(failures, ["Wired response primary address"]);
        } else {
            assert!(failures.is_empty(), "{:?}: {failures:?}", example.pages);
        }
        assert_eq!(report.assessed, report.passed + failures.len());
        assert!(report
            .findings
            .iter()
            .all(|f| f.source_url.contains(".pdf#page=")));
        if example.pages.iter().any(|p| p == "5.0.1/30") {
            assert!(report
                .findings
                .iter()
                .any(|f| f.rule == "Transport header for link function"
                    && f.status.as_str() == "could_not_check"));
        }
    }
}

fn radio_telegram(function: u8, sender: &str, payload: &str) -> String {
    let body = format!("{function:02X}{sender}{payload}");
    format!("{:02X}{body}", body.len() / 2)
}

fn capture_report(packets: &[String], consecutive: bool) -> serde_json::Value {
    use m_bus_parser::output::{render_oms_capture, FragmentOutput, OmsFormat};
    let capture =
        m_bus_parser::decode_capture_hex(&packets.join("\n"), &DecodeOptions::default()).unwrap();
    let frames = match capture {
        FragmentOutput::Single(frame) => vec![*frame],
        FragmentOutput::Multiple(frames) => frames,
    };
    serde_json::from_str(&render_oms_capture(&frames, OmsFormat::Json, None, consecutive).unwrap())
        .unwrap()
}

fn status<'a>(report: &'a serde_json::Value, rule: &str) -> Option<&'a str> {
    report["findings"]
        .as_array()?
        .iter()
        .find(|finding| finding["rule"] == rule)?["status"]
        .as_str()
}

#[test]
fn access_counters_follow_each_sender_across_wraparound_and_repeated_telegrams() {
    let sender = "9344443322115507";
    let other = "9344453322115507";
    let frame = |source, cc, acc| {
        radio_telegram(
            0x44,
            source,
            &format!("8C{cc:02X}{acc:02X}7A99000000011301"),
        )
    };
    let capture = [
        frame(sender, 0x20, 255),
        frame(other, 0x20, 23),
        frame(sender, 0x30, 55),
        frame(sender, 0x20, 0),
        frame(sender, 0, 0),
        frame(sender, 0, 9),
        frame(sender, 0x20, 4),
        radio_telegram(0x44, other, "7A18000020011301"),
    ];
    let sync = "Synchronous access number increment";
    let asynchronous = "Asynchronous access number reuse";
    for consecutive in [false, true] {
        let r = capture_report(&capture, consecutive);
        assert_eq!(status(&r[0], sync), Some("could_not_check"));
        assert_eq!(status(&r[1], sync), Some("could_not_check"));
        assert_eq!(status(&r[2], sync), None);
        assert_eq!(status(&r[3], sync), Some("passed"));
        assert_eq!(status(&r[4], asynchronous), Some("passed"));
        let mismatch = Some(if consecutive {
            "failed"
        } else {
            "could_not_check"
        });
        assert_eq!(status(&r[5], asynchronous), mismatch);
        assert_eq!(status(&r[6], sync), mismatch);
        assert_eq!(status(&r[7], sync), Some("passed"));
    }
}

#[test]
fn access_echo_checks_reversed_peer_addresses_and_independent_transport_numbers() {
    let meter = "9344443322115507";
    let gateway = "3A63665544330A31";
    let request = radio_telegram(
        0x5b,
        gateway,
        &format!("8E8012{meter}80443322119344550741000000"),
    );
    let response = |dest, ll, tpl| {
        radio_telegram(
            8,
            meter,
            &format!("8E80{ll:02X}{dest}7A{tpl:02X}000000011301"),
        )
    };
    let check = |reply| capture_report(&[request.clone(), reply], true)[1].clone();
    let good = check(response(gateway, 0x12, 0x41));
    for rule in [
        "Reply link access number",
        "Reply transport access number",
        "Reply destination address",
    ] {
        assert_eq!(status(&good, rule), Some("passed"), "{rule}");
    }
    let bad = check(response(gateway, 0x12, 0x42));
    assert_eq!(
        status(&bad, "Reply transport access number"),
        Some("failed")
    );
    assert_eq!(status(&bad, "Reply link access number"), Some("passed"));
    let unrelated = check(response("3A63775544330A31", 0x12, 0x42));
    assert_eq!(status(&unrelated, "Reply transport access number"), None);
}

#[test]
fn integrated_disconnector_uses_subunit_one_and_commands_use_table_m8() {
    let rule = "Integrated disconnector subunit";
    for (sender, header, expected) in [
        ("9344443322115507", "01FD1F", Some("failed")),
        ("9344443322115507", "8140FD1F", Some("passed")),
        ("9344443322115521", "01FD1F", None),
    ] {
        let report = capture_report(
            &[radio_telegram(
                0x44,
                sender,
                &format!("7A01000000{header}01"),
            )],
            false,
        );
        assert_eq!(status(&report, rule), expected);
    }
    for (value, expected) in [(0, "passed"), (1, "passed"), (2, "passed"), (3, "failed")] {
        let output = m_bus_parser::output::render_application_hex(
            &format!("5101FD1F{value:02X}"),
            m_bus_parser::OutputFormat::Oms(m_bus_parser::output::OmsFormat::Json),
            &Default::default(),
        )
        .unwrap();
        let r = serde_json::from_str(&output).unwrap();
        assert_eq!(status(&r, "Disconnector command value"), Some(expected));
        assert_eq!(
            status(&r, "Manual switch-on capability"),
            (value == 2).then_some("could_not_check")
        );
        assert_eq!(status(&r, rule), Some("could_not_check"));
    }
}

#[test]
fn annex_b_encryption_respects_record_boundaries_including_filler() {
    let report = capture_report(
        &[radio_telegram(
            0x44,
            "9344443322115507",
            "7A010000000113012F0C7801020304",
        )],
        false,
    );
    let rows: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["rule"] == "Data point encryption")
        .map(|row| (row["record_number"].as_u64(), row["status"].as_str()))
        .collect();
    assert_eq!(rows, [(Some(1), Some("failed")), (Some(2), Some("passed"))]);
    let published = examples()
        .into_iter()
        .find(|example| example.pages.iter().any(|page| page == "5.0.1/62"))
        .unwrap()
        .decode()
        .unwrap();
    let records = serde_json::to_value(&published.records).unwrap();
    assert_eq!(records[0]["transport_protection"], "encrypted");
    assert_eq!(
        records.as_array().unwrap().last().unwrap()["transport_protection"],
        "clear"
    );
}

#[test]
fn access_only_frames_and_external_adapters_use_the_expected_headers() {
    let meter = "9344443322115507";
    let adapter = "9344443322115537";
    let check = |c, id, payload| capture_report(&[radio_telegram(c, id, payload)], false);
    let empty = check(0x47, meter, "8C2001");
    assert_eq!(status(&empty, "ACC-NR application data"), Some("passed"));
    let data = check(0x47, meter, "8C20017A01000000011301");
    assert_eq!(status(&data, "ACC-NR application data"), Some("failed"));
    let short = check(0x44, adapter, "8C20017A01000000011301");
    assert_eq!(
        status(&short, "RF adapter application address"),
        Some("failed")
    );
    let dlms = check(0x44, adapter, "7D010000000110C401C100060000001C");
    assert_eq!(
        status(&dlms, "RF adapter application address"),
        Some("failed")
    );
    let gateway = "9344111111110131";
    let command = radio_telegram(
        0x53,
        gateway,
        "8E0001934444332211553761010000001001C001C1000100002B0100FF0200",
    );
    assert_eq!(
        status(
            &capture_report(&[command], false),
            "RF adapter application address"
        ),
        Some("failed")
    );
    let long = check(0x44, adapter, "8C200172443322119344550701000000011301");
    assert_eq!(
        status(&long, "RF adapter application address"),
        Some("passed")
    );
}
