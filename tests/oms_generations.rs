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
