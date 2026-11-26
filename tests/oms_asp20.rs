#![cfg(all(feature = "std", feature = "oms-gen5"))]

use m_bus_parser::output::{decode_bytes, decode_bytes_with_application_key, DecodeOptions};

const COMMAND: &str = "F20ABB316C0584CEAF2741D21ADF3A65E5";
const RESPONSE: &str = "23F955992C57563C726C59695E439CEF2F";
const KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

fn frame(command: bool, protected: &str) -> Vec<u8> {
    let (header, control) = if command {
        ("539344121111110A31C37856341293440A0701000000", "20")
    } else {
        ("449344785634120A07C401000000", "A0")
    };
    let mut frame = hex::decode(format!(
        "00{header}1B0000{control}0036200178563412{protected}"
    ))
    .unwrap();
    frame[0] = (frame.len() - 1) as u8;
    frame
}

#[test]
fn application_key_authenticates_both_asp20_directions() {
    for (command, ciphertext) in [(true, COMMAND), (false, RESPONSE)] {
        let bytes = frame(command, ciphertext);
        let options = DecodeOptions::default();
        let mut decoded = decode_bytes(&bytes, &options).unwrap();
        let finding = |decoded: &m_bus_parser::output::DecodedOutput| {
            decoded
                .assess_oms_5_0_1()
                .findings
                .into_iter()
                .find(|f| f.rule == "ASP20 application authentication")
                .unwrap()
                .status
                .as_str()
        };
        assert_eq!(finding(&decoded), "could_not_check");
        let strict = decode_bytes_with_application_key(&bytes, &options, &KEY).unwrap();
        assert_eq!(finding(&strict), "passed");
        decoded.authenticate_application(&KEY).unwrap();
        let fields = decoded
            .application
            .as_ref()
            .unwrap()
            .decoded
            .as_ref()
            .unwrap();
        assert_eq!(fields["apdu_hex"], "01132A");
        assert_eq!(fields["key_counter"], 0x12345678);
        assert_eq!(fields["records"][0]["value"]["value"], "0.042");
        assert!(decoded.authenticate_application(&[0; 16]).is_err());
        assert_eq!(finding(&decoded), "failed");
        let fields = decoded
            .application
            .as_ref()
            .unwrap()
            .decoded
            .as_ref()
            .unwrap();
        assert!(fields.get("apdu_hex").is_none() && fields.get("records").is_none());
        let mut changed = bytes;
        *changed.last_mut().unwrap() ^= 1;
        assert!(decode_bytes_with_application_key(&changed, &options, &KEY).is_err());
    }
}

#[test]
fn asp20_uses_transport_meter_and_direction() {
    let mut bytes = frame(true, COMMAND);
    bytes[11] ^= 1;
    assert!(decode_bytes_with_application_key(&bytes, &Default::default(), &KEY).is_err());
    assert!(
        decode_bytes_with_application_key(&frame(false, COMMAND), &Default::default(), &KEY)
            .is_err()
    );
}
