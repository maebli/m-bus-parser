#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_oms_rules::lpwan::{has_mac_crc, LpwanHeader};
use m_bus_parser::output::{render_oms_capture, OmsFormat};
use m_bus_parser::{decode_hex, DecodeOptions};

const MSNR: &str = "005B44A73D78563412150375900F002C25B30A000021924D4F2FB66E017A75002007109058475F4BC91DF878B80A1B0F98B629024AAC727942BFC549233C0140829B932BE5B9B7";
const MACK: &str = "098378CFC7";
const MERR: &str = "422202A73D7856341215039F07FC0F";
const MCMD: &str = "2D6801370140A853A89304A73D78563412150351E4A0D6";
const MCMD_REQ: &str = "2D68013801D4EF39BC311D7BA73D785634121503089375170000DACEF83C";
const MRSP: &str = "61226A01380122D32DB30481AC1B08A73D78563412150308900F002C25B30A000021924D4F2FB66E017A75002007109058475F4BC91DF878B80A1B0F98B629024AAC727942BFC549233C0140829B936DBE2FD6";

#[test]
fn six_published_annex_q_examples_decode_with_and_without_upper_layers() {
    let options = DecodeOptions::default();
    for (input, access) in [
        (MSNR, Some(0x75)),
        (MACK, None),
        (MERR, None),
        (MCMD, None),
        (MCMD_REQ, Some(8)),
        (MRSP, Some(8)),
    ] {
        let frame = decode_hex(input, &options).unwrap();
        assert_eq!(frame.frame.kind, "lpwan");
        let header = frame.lpwan.as_ref().unwrap();
        assert_eq!(
            header.llc.as_ref().and_then(|llc| llc.access_number),
            access
        );
        let report = frame.assess_oms_5_0_1();
        if input == MCMD_REQ {
            assert_eq!(frame.transport.application_ci, Some(0x93));
            assert_eq!(frame.transport.header_kind.as_deref(), Some("short_tpl"));
            assert!(!report
                .findings
                .iter()
                .any(|f| f.rule == "Transport header for link function"
                    && f.status.as_str() == "failed"));
        }
        for check in report
            .findings
            .iter()
            .filter(|f| f.rule.starts_with("LPWAN"))
        {
            assert_ne!(check.status.as_str(), "failed", "{input}: {}", check.rule);
        }
    }
    let with_key = DecodeOptions {
        key: Some(core::array::from_fn(|i| i as u8)),
        ..Default::default()
    };
    for input in [MSNR, MRSP] {
        let frame = decode_hex(input, &with_key).unwrap();
        assert_eq!(frame.security.decryption_state, "decrypted_authenticated");
        assert_eq!(frame.records.len(), 3);
    }
    let frame = decode_hex(MCMD, &options).unwrap();
    let body = frame.lpwan.unwrap().body.unwrap();
    assert_eq!(body.derivation_counter, Some(1));
    assert_eq!(body.message_counter, Some(311));
    assert_eq!(body.encrypted_blocks, [0x93]);
}

fn packet(body: &[u8]) -> Vec<u8> {
    let mut crc = 0u32;
    for byte in body {
        crc ^= u32::from(*byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0xf4ac_fb13
            } else {
                crc << 1
            };
        }
    }
    [body, &crc.to_be_bytes()].concat()
}

#[test]
fn mac_fields_and_llc_flags_reject_invalid_values() {
    for (data, rule) in [
        (&[0x49, 2][..], "LPWAN MAC acknowledge access"),
        (&[0x20, 0x41, 1], "LPWAN MAC derivation counter protection"),
        (&[0x28, 1, 1], "LPWAN MAC block frame type"),
        (&[0x20, 1, 14], "LPWAN MAC block identifiers"),
        (
            &[0x0d, 0x44, 0xa7, 0x3d, 0x78, 0x56, 0x34, 0x12, 0x15, 3],
            "LPWAN LLC addressing",
        ),
        (&[0, 0x20], "LPWAN LLC control bits"),
        (&[0, 0x10], "LPWAN upper layer presence"),
        (&[0, 0x00, 0x78], "LPWAN upper layer presence"),
        (&[1, 0], "LPWAN MAC response body"),
    ] {
        let bytes = packet(data);
        let frame = m_bus_parser::decode_bytes(&bytes, &DecodeOptions::default()).unwrap();
        assert!(
            frame
                .assess_oms_5_0_1()
                .findings
                .iter()
                .any(|f| f.rule == rule && f.status.as_str() == "failed"),
            "{rule}"
        );
    }
}

#[test]
fn malformed_mac_fields_do_not_read_past_the_frame() {
    let mut frame = hex::decode(MSNR).unwrap();
    frame[1] ^= 1;
    assert!(!has_mac_crc(&frame));
    for data in [&[0x20, 1], &[0, 2], &[0x40, 0x80], &[0x20, 0x2f]] {
        assert!(LpwanHeader::parse(&packet(data)).is_err());
    }
}

#[test]
fn independent_mac_frames_produce_ordered_reports() {
    let frames = m_bus_parser::output::decode_capture_hex(
        &[MSNR, MACK, MCMD_REQ, MRSP].join("\n"),
        &Default::default(),
    )
    .unwrap();
    let m_bus_parser::output::FragmentOutput::Multiple(frames) = frames else {
        panic!("expected four frames")
    };
    assert_eq!(frames.len(), 4);
    assert!(render_oms_capture(&frames, OmsFormat::Json, Some(3), true)
        .unwrap()
        .contains("LPWAN MAC response body"));
}
