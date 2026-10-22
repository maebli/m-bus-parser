use m_bus_application_layer::{
    compact::{format_signature, CompactFrame, FormatFrame},
    data_information::DataType,
    parse_data_records, ApplicationLayerError,
};

fn format_payload(descriptors: &[u8]) -> Vec<u8> {
    let mut bytes = vec![u8::try_from(descriptors.len() + 2).unwrap()];
    bytes.extend_from_slice(&format_signature(descriptors).to_le_bytes());
    bytes.extend_from_slice(descriptors);
    bytes
}

fn compact_payload(descriptors: &[u8], full: &[u8], values: &[u8]) -> Vec<u8> {
    let mut bytes = format_signature(descriptors).to_le_bytes().to_vec();
    bytes.extend_from_slice(&format_signature(full).to_le_bytes());
    bytes.extend_from_slice(values);
    bytes
}

#[test]
fn crc_matches_the_en13757_check_value() {
    assert_eq!(format_signature(b"123456789"), 0xC2B7);
}

#[test]
fn annex_g_example_recovers_all_three_measurements() {
    let descriptors = [0x02, 0x02, 0x02, 0x15, 0x02, 0x2A];
    let full = [
        0x02, 0x02, 0xD2, 0x04, 0x02, 0x15, 0x2E, 0x16, 0x02, 0x2A, 0x34, 0x23,
    ];
    let format_bytes = format_payload(&descriptors);
    let compact_bytes = compact_payload(&descriptors, &full, &[0xD2, 0x04, 0x2E, 0x16, 0x34, 0x23]);
    let format = FormatFrame::parse(&format_bytes).unwrap();
    let compact = CompactFrame::parse(&compact_bytes).unwrap();
    let mut output = [0; 32];
    let length = compact.expand(&format, &mut output).unwrap();
    assert_eq!(&output[..length], &full);
    let records = parse_data_records(&output[..length])
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 3);
    for (record, value) in records.iter().zip([1234.0, 5678.0, 9012.0]) {
        assert_eq!(record.value(), Some(&DataType::Number(value)));
    }
}

#[test]
fn variable_length_values_and_padding_are_reconstructed() {
    let descriptors = [0x0D, 0x78, 0x01, 0x13];
    let full = [0x0D, 0x78, 3, b'C', b'B', b'A', 0x01, 0x13, 9];
    let mut format_bytes = format_payload(&descriptors);
    format_bytes.extend_from_slice(&[0x2F; 4]);
    let compact_bytes = compact_payload(&descriptors, &full, &[3, b'C', b'B', b'A', 9, 0x2F, 0x2F]);
    let format = FormatFrame::parse(&format_bytes).unwrap();
    let compact = CompactFrame::parse(&compact_bytes).unwrap();
    let mut output = [0; 32];
    let length = compact.expand(&format, &mut output).unwrap();
    assert_eq!(&output[..length], &full);
    format_bytes[0] = 0;
    format_bytes.truncate(7);
    assert_eq!(
        FormatFrame::parse(&format_bytes).unwrap().descriptors,
        descriptors
    );
}

#[test]
fn invalid_signatures_crc_lengths_and_output_capacity_are_rejected() {
    let descriptors = [0x02, 0x13];
    let full = [0x02, 0x13, 1, 0];
    let format_bytes = format_payload(&descriptors);
    let compact_bytes = compact_payload(&descriptors, &full, &[1, 0]);
    let format = FormatFrame::parse(&format_bytes).unwrap();
    let compact = CompactFrame::parse(&compact_bytes).unwrap();
    assert_eq!(
        compact.expand(&format, &mut [0; 3]),
        Err(ApplicationLayerError::OutputBufferTooSmall)
    );
    for offset in [0, 2, 4] {
        let mut invalid = compact_bytes.clone();
        invalid[offset] ^= 1;
        assert!(CompactFrame::parse(&invalid)
            .unwrap()
            .expand(&format, &mut [0; 8])
            .is_err());
    }
    let mut invalid_format = format_bytes.clone();
    invalid_format[1] ^= 1;
    assert!(matches!(
        FormatFrame::parse(&invalid_format),
        Err(ApplicationLayerError::ChecksumMismatch { .. })
    ));
    for length in 0..format_bytes.len() {
        assert!(FormatFrame::parse(&format_bytes[..length]).is_err());
    }
    for length in 0..compact_bytes.len() {
        if let Ok(truncated) = CompactFrame::parse(&compact_bytes[..length]) {
            assert!(truncated.expand(&format, &mut [0; 8]).is_err());
        }
    }
    let invalid_descriptor = format_payload(&[0x81]);
    assert!(FormatFrame::parse(&invalid_descriptor).is_err());
}

#[test]
fn manufacturer_specific_tail_is_preserved() {
    let descriptors = [0x01, 0x13, 0x0F];
    let full = [0x01, 0x13, 9, 0x0F, 0xDE, 0xAD];
    let format_bytes = format_payload(&descriptors);
    let compact_bytes = compact_payload(&descriptors, &full, &[9, 0xDE, 0xAD]);
    let format = FormatFrame::parse(&format_bytes).unwrap();
    let compact = CompactFrame::parse(&compact_bytes).unwrap();
    let mut output = [0; 6];
    assert_eq!(compact.expand(&format, &mut output).unwrap(), 6);
    assert_eq!(output, full);
}
