#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{decode_bytes, decode_fragments, DecodeOptions, DecodedOutput};
use serde_json::json;

const TR06_READING: [&str; 2] = [
    "3A089344785634120A079009017865B30A000026007A0200200710F076F3A6810C580A18306E68283F0CA970FE9473C3849FAE5DC115ADDB04E3DF",
    "15089344785634120A07900A0204E22CDAB94EB57DCA",
];
const TR06_CLOCK: [&str; 2] = [
    "3C539344785634120A0790050170403500C37856341293440A07310000C026000020003221000F000000000000003004013700000000000000002F2F2F",
    "16539344785634120A07900202002F4EBA2727E96D2FA2",
];
const TR08_CLOCK_TRANSPORT: &str = "6D78563412A73D0107A300100591C25C60DE13CBDC6AA9C47878C87056";
const TR07_PROFILE: &str = "934493447856341201027A288C200A900F002C25B30A00004A8CE08AA142990479627A0A00700710A77E3B8288C0A7691D0BAEA29CE89CD5D4F39FFD4FF98564261A65EE75D6BF4C6ADA797E9E0D4E4643992EDD8EE47D2CE934FFFEA30287A93E72528CA13BE9609D426F02947C42A0C6B30D36F4D72FB96396ED0E508CFBC4885BBEE563922F014ABDBFC3B295E29E912BC9132F5C68E2E0396E6FF7DF5D013F9AAE5BD7D1E822";
const TR12_GRID: &str = "C344A73D67452301010277608C200A900F002C2500000000D4706BB5CAE9CB99DC127A0A00A00710A81D883DAC8C07DF2E05E5175DF4A4386F2EFA1B29333C7BAF8D9D98151086AC83862B2FCCCF09BF0D20D179CAF65D3F51C885E02CE7B1CB34740F6172670394D2B89DCF4C995C9A6E5302183D249521CC51F8107972F66F6014036B9AC94C591A34DC35BACD83B1C132CF24966C5069BD0E5896E8294688B22723A36487C29757784B8969EEB09A6DE25C7AD10248577A53CE9E842107511AD1F5827DC0BC4232C2F4C9FFBE6F4BC2553330651D9E41D62E2534D1E1";
const TEST_KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

fn keyed() -> DecodeOptions {
    DecodeOptions {
        key: Some(TEST_KEY),
        ..Default::default()
    }
}

fn wireless_frame(header: &str, application: &[u8]) -> Vec<u8> {
    let mut frame = vec![0];
    frame.extend(hex::decode(header).unwrap());
    frame.extend(application);
    frame[0] = u8::try_from(frame.len() - 1).unwrap();
    frame
}

fn readings(data: &str) -> DecodedOutput {
    let mut payload = vec![0x78];
    payload.extend(hex::decode(data).unwrap());
    decode_bytes(
        &wireless_frame("449344785634120A07", &payload),
        &DecodeOptions::default(),
    )
    .unwrap()
}

#[test]
fn differential_profiles_require_a_valid_matching_base() {
    let profile = "CD018313054101010203";
    let invalid = readings(&format!("C201030100C201030080{profile}010301"));
    assert_eq!(invalid.decode_state, "partial");
    assert_eq!(
        invalid.diagnostics[0].message,
        "invalid compact profile base reading"
    );
    assert!(invalid.records[2].value.value.is_none());
    assert_eq!(invalid.records[3].value.value, Some("1".into()));

    let valid = readings(&format!("C201030180{profile}"));
    assert_eq!(valid.decode_state, "complete");
    assert_eq!(
        valid.records[1].value.value.as_ref().unwrap()["values"],
        json!(["-32768", "-32770", "-32773"])
    );

    let bad_difference = readings("C201030100CD01831305410101FF03");
    assert_eq!(
        bad_difference.records[1].value.value.as_ref().unwrap()["values"],
        json!(["0", null, null])
    );
}

#[test]
fn real_profiles_preserve_fractional_values_and_following_records() {
    let output = readings("84046D002041118D04831F0605010000C03F010301");
    assert_eq!(output.decode_state, "complete");
    assert_eq!(
        output.records[1].value.value.as_ref().unwrap()["values"],
        json!(["1.5"])
    );
    assert_eq!(output.records[2].value.value, Some("1".into()));

    let increments = readings("05020000C03F0D821F0A45010000003F0000C07F");
    assert_eq!(
        increments.records[1].value.value.as_ref().unwrap()["values"],
        json!(["0.2", null])
    );

    let reserved = readings("0D831F060D010000C03F010301");
    assert_eq!(reserved.decode_state, "partial");
    assert_eq!(reserved.records[1].value.value, Some("1".into()));
    assert_eq!(reserved.diagnostics[0].offset_start, Some(0));
}

#[test]
fn tr06_a6_authenticates_a_mac_only_final_fragment() {
    let [first, last] = TR06_READING.map(|s| hex::decode(s).unwrap());
    let frames = [first.as_slice(), last.as_slice()];
    let output = decode_fragments(&frames, &keyed()).unwrap();
    assert_eq!(output.decode_state, "complete");
    assert_eq!(output.security.decryption_state, "decrypted_authenticated");
    assert_eq!(output.records[0].value.value, Some("23456.789".into()));
    assert_eq!(
        decode_bytes(&last, &keyed())
            .unwrap()
            .security
            .decryption_state,
        "awaiting_fragments"
    );
    assert!(decode_fragments(&frames, &DecodeOptions::default())
        .unwrap()
        .records
        .is_empty());
    let wrong = DecodeOptions {
        key: Some([0; 16]),
        ..Default::default()
    };
    assert_eq!(
        decode_fragments(&frames, &wrong).unwrap_err().code(),
        "security.authentication_failed"
    );
    for flags in [0, 0x04, 0x40] {
        let empty = wireless_frame("089344785634120A07", &[0x90, 2, 2, flags]);
        assert_eq!(
            decode_fragments(&[&first, &empty], &keyed())
                .unwrap_err()
                .code(),
            "fragment.invalid"
        );
    }
}

#[test]
fn tr06_a7_decodes_the_sitp_clock_command() {
    let [first, last] = TR06_CLOCK.map(|s| hex::decode(s).unwrap());
    let options = DecodeOptions::default();
    let output = decode_fragments(&[&first, &last], &options).unwrap();
    assert_eq!(output.decode_state, "complete");
    let sitp = output.application.unwrap().decoded.unwrap();
    assert_eq!(sitp["application_mac_verified"], false);
    assert_eq!(sitp["clock"]["operation"], "Add");
    assert_eq!(
        sitp["clock"]["value"]["Time"],
        json!([{"Single":55},{"Single":0},{"Single":0}])
    );

    let mut transport = first[17..].to_vec();
    transport.extend(&last[14..]);
    let frame = wireless_frame("539344785634120A07", &transport);
    let key = [0x08, 0x15, 0x47, 0x11].repeat(4).try_into().unwrap();
    let authenticated =
        m_bus_parser::output::decode_bytes_with_application_key(&frame, &options, &key).unwrap();
    assert_eq!(
        authenticated.application.unwrap().decoded.unwrap()["application_mac_verified"],
        true
    );

    for (offset, replacement, error) in [
        (47, 0xfe, "application.sitp_protocol_unsupported"),
        (53, 1, "application.payload_invalid"),
    ] {
        let mut changed = first.clone();
        changed[offset] = replacement;
        let result = decode_fragments(&[&changed, &last], &options).unwrap();
        assert_eq!(result.decode_state, "partial");
        assert!(
            result.diagnostics.iter().any(|d| d.code == error),
            "{:?}",
            result.diagnostics
        );
        assert!(result.application.unwrap().decoded.unwrap()["apdu_hex"].is_string());
    }
}

#[test]
fn tr08_a8_decodes_the_encrypted_mbus_clock_command() {
    let transport = hex::decode(TR08_CLOCK_TRANSPORT).unwrap();
    let frame = wireless_frame("53A73D785634120107", &transport);
    let output = decode_bytes(&frame, &keyed()).unwrap();
    assert_eq!(output.decode_state, "complete");
    let payload = output.application.unwrap().decoded.unwrap();
    assert_eq!(payload["Clock"]["operation"], "Add");
    assert_eq!(payload["Clock"]["value"]["Time"][0]["Single"], 50);
}

#[test]
fn annex_m_clock_apdus_accept_only_the_exact_time_field_or_verified_padding() {
    use m_bus_parser::user_data::application_message::{parse_sitp_clock, ClockOperation};
    for (control, operation) in [
        (1, ClockOperation::Add),
        (2, ClockOperation::Subtract),
        (3, ClockOperation::Correct),
    ] {
        assert_eq!(parse_sitp_clock(&[control, 55, 0, 0]).unwrap().0, operation);
    }
    assert_eq!(
        parse_sitp_clock(&[0, 0x36, 0x35, 0xc0, 0x53, 0x39, 0])
            .unwrap()
            .0,
        ClockOperation::Set
    );
    for short in [&[1, 55, 0][..], &[1, 55, 0, 0, 0], &[0xfe, 55, 0, 0]] {
        assert!(parse_sitp_clock(short).is_err());
    }
}

#[test]
fn tr07_history_and_tr12_grid_readings_match_the_published_values() {
    let profile = m_bus_parser::decode_hex(TR07_PROFILE, &keyed()).unwrap();
    assert_eq!(profile.security.decryption_state, "decrypted_authenticated");
    let history = profile
        .records
        .iter()
        .find(|r| r.value.kind == "compact_profile")
        .unwrap()
        .value
        .value
        .as_ref()
        .unwrap();
    assert_eq!(history["spacing"], 6);
    assert_eq!(history["spacing_unit"], "second");
    let expected = [
        1994, 1991, 1986, 1979, 1970, 1962, 1956, 1952, 1950, 1950, 1949, 1946, 1941, 1934, 1925,
        1917, 1911, 1907, 1905, 1905, 1904, 1901, 1896, 1889, 1880, 1872, 1866, 1862, 1860, 1860,
        1859, 1856, 1851, 1844, 1835, 1827, 1821, 1817, 1815, 1815, 1814, 1811, 1806, 1799, 1790,
        1782, 1776, 1772, 1770, 1770, 1769, 1766, 1761, 1754, 1745, 1737, 1731, 1727, 1725, 1725,
        1724,
    ]
    .map(|v| v.to_string());
    assert_eq!(
        history["values"],
        serde_json::to_value(expected.as_slice()).unwrap()
    );
    let grid = m_bus_parser::decode_hex(TR12_GRID, &keyed()).unwrap();
    assert_eq!(grid.security.decryption_state, "decrypted_authenticated");
    assert_eq!(grid.decode_state, "complete");
    let values: Vec<_> = grid
        .records
        .iter()
        .map(|r| r.value.value.as_ref().and_then(|v| v.as_str()).unwrap())
        .collect();
    assert_eq!(
        values,
        [
            "230.2", "230.4", "230.6", "6.12", "6.23", "6.34", "1", "2", "3", "10", "230.2",
            "230.4", "230.6", "6.12", "6.23", "6.34", "1", "2", "3", "70", "0"
        ]
    );
}
