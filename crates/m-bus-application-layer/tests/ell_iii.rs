use m_bus_application_layer::{
    data_information::DataType,
    extended_link_layer::{EllFormat, ExtendedLinkLayer},
    parse_application_layer, ApplicationLayerError, ApplicationPayload, UserDataBlock,
};

const HEADER: [u8; 10] = [0x20, 0x27, 0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 1, 7];

#[test]
fn ell_iii_requires_exactly_ten_header_bytes_without_encryption_fields() {
    let (header, consumed) = ExtendedLinkLayer::parse(&HEADER, EllFormat::FormatIII).unwrap();
    assert_eq!(consumed, 10);
    assert_eq!(header.communication_control, 0x20);
    assert_eq!(header.access_number, 0x27);
    assert!(header.encryption.is_none());
    let receiver = header.receiver_address.unwrap();
    assert_eq!(receiver.manufacturer, 0x2C2D);
    assert_eq!(receiver.address, [0x78, 0x56, 0x34, 0x12, 1, 7]);
    for length in 0..HEADER.len() {
        assert_eq!(
            ExtendedLinkLayer::parse(&HEADER[..length], EllFormat::FormatIII),
            Err(ApplicationLayerError::InsufficientData)
        );
    }
}

#[test]
fn ell_iii_preserves_nested_records_and_application_reports() {
    for application in [
        &[0x78, 0x01, 0x13, 42][..],
        &[0x7A, 0x99, 8, 0, 0, 0x01, 0x13, 42][..],
        &[
            0x72, 1, 0, 0, 0, 0x2D, 0x2C, 1, 7, 0x99, 8, 0, 0, 0x01, 0x13, 42,
        ][..],
        &[0x70, 8][..],
    ] {
        let mut data = vec![0x8E];
        data.extend_from_slice(&HEADER);
        data.extend_from_slice(application);
        let block = parse_application_layer(&data).unwrap();
        let ell = match &block {
            UserDataBlock::VariableDataStructureWithLongTplHeader {
                extended_link_layer,
                ..
            }
            | UserDataBlock::VariableDataStructureWithShortTplHeader {
                extended_link_layer,
                ..
            }
            | UserDataBlock::VariableDataStructureWithoutTplHeader {
                extended_link_layer,
                ..
            } => extended_link_layer,
            UserDataBlock::ApplicationMessage(message) => &message.extended_link_layer,
            _ => panic!("expected nested application block"),
        }
        .as_ref()
        .unwrap();
        assert_eq!(ell.access_number, 0x27);
        assert!(ell.receiver_address.is_some());
        assert!(ell.encryption.is_none());
        if let UserDataBlock::ApplicationMessage(message) = &block {
            assert_eq!(message.control_information, 0x70);
            assert_eq!(
                message.decode().unwrap(),
                ApplicationPayload::Error {
                    code: 8,
                    details: &[]
                }
            );
            assert_eq!(message.payload.as_ptr(), data[12..].as_ptr());
        } else {
            let records = block
                .data_records()
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].value(), Some(&DataType::Number(42.0)));
            assert_eq!(
                records[0].raw_bytes().as_ptr(),
                data[data.len() - 3..].as_ptr()
            );
        }
    }
}

#[test]
fn ell_iii_rejects_missing_or_truncated_nested_headers() {
    let mut data = vec![0x8E];
    data.extend_from_slice(&HEADER);
    assert!(parse_application_layer(&data).is_err());
    for application in [&[0x7A, 1][..], &[0x72, 1][..]] {
        let mut truncated = data.clone();
        truncated.extend_from_slice(application);
        assert!(parse_application_layer(&truncated).is_err());
    }
}
