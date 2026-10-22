use m_bus_application_layer::{
    application_message::ClockOperation,
    compact::format_signature,
    data_information::{DataType, SingleEveryOrInvalid},
    parse_application_layer, ApplicationLayerError, ApplicationPayload, ApplicationProtocol,
    UserDataBlock,
};

const LONG_HEADER: [u8; 12] = [0x78, 0x56, 0x34, 0x12, 0x2D, 0x2C, 1, 7, 3, 0, 0, 0];

fn telegram(ci: u8, header_size: usize, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![ci];
    match header_size {
        4 => bytes.extend_from_slice(&LONG_HEADER[8..]),
        12 => bytes.extend_from_slice(&LONG_HEADER),
        _ => {}
    }
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn control_commands_decode_baud_rates_and_standard_synchronization() {
    for (ci, protocol) in [
        (0x5C, ApplicationProtocol::Synchronize),
        (0x54, ApplicationProtocol::RequestSelectedApplication),
        (0xB8, ApplicationProtocol::SetBaudRate(300)),
        (0xB9, ApplicationProtocol::SetBaudRate(600)),
        (0xBA, ApplicationProtocol::SetBaudRate(1200)),
        (0xBB, ApplicationProtocol::SetBaudRate(2400)),
        (0xBC, ApplicationProtocol::SetBaudRate(4800)),
        (0xBD, ApplicationProtocol::SetBaudRate(9600)),
        (0xBE, ApplicationProtocol::SetBaudRate(19200)),
        (0xBF, ApplicationProtocol::SetBaudRate(38400)),
    ] {
        let bytes = [ci];
        let block = parse_application_layer(&bytes).unwrap();
        assert!(block.data_records().is_none());
        let UserDataBlock::ApplicationMessage(message) = block else {
            panic!("expected command")
        };
        assert_eq!(message.protocol, protocol);
        match protocol {
            ApplicationProtocol::SetBaudRate(baud) => assert_eq!(
                message.decode().unwrap(),
                ApplicationPayload::BaudRate(baud)
            ),
            _ => assert_eq!(message.decode().unwrap(), ApplicationPayload::Command),
        }
        assert!(matches!(
            parse_application_layer(&[ci, 0]),
            Err(ApplicationLayerError::InvalidDataLength { .. })
        ));
    }
}

#[test]
fn command_transport_headers_expose_written_records() {
    for (ci, size) in [(0x5A, 4), (0x5B, 12)] {
        let bytes = telegram(ci, size, &[0x01, 0x7A, 8]);
        let block = parse_application_layer(&bytes).unwrap();
        let records = block
            .data_records()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value(), Some(&DataType::Number(8.0)));
        for length in 1..=size {
            assert!(parse_application_layer(&bytes[..length]).is_err());
        }
    }
}

#[test]
fn reports_keep_error_codes_and_manufacturer_alarm_details() {
    for (ci, size) in [(0x70, 0), (0x6E, 4), (0x6F, 12)] {
        for payload in [&[][..], &[8][..], &[0x80, 0x12, 0x34][..]] {
            let bytes = telegram(ci, size, payload);
            let UserDataBlock::ApplicationMessage(message) =
                parse_application_layer(&bytes).unwrap()
            else {
                panic!("expected error report")
            };
            assert_eq!(
                message.decode().unwrap(),
                ApplicationPayload::Error {
                    code: payload.first().copied().unwrap_or(0),
                    details: payload.get(1..).unwrap_or_default(),
                }
            );
        }
    }
    for (ci, size) in [(0x71, 0), (0x74, 4), (0x75, 12)] {
        let bytes = telegram(ci, size, &[0x81, 0x23]);
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected alarm")
        };
        assert_eq!(
            message.decode().unwrap(),
            ApplicationPayload::Alarm {
                status: Some(0x81),
                details: &[0x23]
            }
        );
    }
}

#[test]
fn external_protocols_keep_their_identity_header_and_borrowed_payload() {
    for (ci, size, protocol) in [
        (0xB1, 0, ApplicationProtocol::ReadRam),
        (0xB2, 0, ApplicationProtocol::WriteRam),
        (0xB3, 0, ApplicationProtocol::StartCalibration),
        (0xB4, 0, ApplicationProtocol::ReadEeprom),
        (0xB6, 0, ApplicationProtocol::StartSoftwareTest),
        (0x60, 12, ApplicationProtocol::DlmsCosem),
        (0x61, 4, ApplicationProtocol::DlmsCosem),
        (0x7C, 12, ApplicationProtocol::DlmsCosem),
        (0x7D, 4, ApplicationProtocol::DlmsCosem),
        (0x64, 12, ApplicationProtocol::Sml),
        (0x65, 4, ApplicationProtocol::Sml),
        (0x7E, 12, ApplicationProtocol::Sml),
        (0x7F, 4, ApplicationProtocol::Sml),
        (0x81, 0, ApplicationProtocol::NetworkLayer),
        (0x82, 12, ApplicationProtocol::NetworkManagement),
        (0x83, 0, ApplicationProtocol::NetworkManagement),
        (0x89, 0, ApplicationProtocol::NetworkManagement),
    ] {
        let bytes = telegram(ci, size, &[0xDE, 0xAD, 0xBE, 0xEF]);
        let block = parse_application_layer(&bytes).unwrap();
        assert!(block.data_records().is_none());
        let UserDataBlock::ApplicationMessage(message) = block else {
            panic!("expected protocol message")
        };
        assert_eq!(message.protocol, protocol);
        assert_eq!(message.transport_header.size(), size);
        assert_eq!(message.payload.as_ptr(), bytes[1 + size..].as_ptr());
        assert_eq!(
            message.decode().unwrap(),
            ApplicationPayload::External(&[0xDE, 0xAD, 0xBE, 0xEF])
        );
        for length in 1..=size {
            assert!(parse_application_layer(&bytes[..length]).is_err());
        }
    }
    for procedure in 0..8 {
        let bytes = [0x90 + procedure, 0x12, 0x34];
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected legacy hash procedure")
        };
        assert_eq!(
            message.protocol,
            ApplicationProtocol::LegacyHashProcedure(procedure)
        );
        assert_eq!(message.payload, &[0x12, 0x34]);
    }
}

#[test]
fn transport_only_messages_are_not_mistaken_for_data_records() {
    for (ci, size, protocol) in [
        (0x80, 12, ApplicationProtocol::TransportOnly),
        (0x84, 12, ApplicationProtocol::RequestCompactFrame),
        (0x85, 12, ApplicationProtocol::RequestFormatFrame),
        (0x8A, 4, ApplicationProtocol::TransportOnly),
        (0x8B, 12, ApplicationProtocol::TransportOnly),
    ] {
        let bytes = telegram(ci, size, &[]);
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected transport-only message")
        };
        assert_eq!(message.protocol, protocol);
        assert_eq!(message.decode().unwrap(), ApplicationPayload::Command);
        let extra = telegram(ci, size, &[0]);
        assert!(parse_application_layer(&extra).is_err());
    }
}

#[test]
fn clock_commands_decode_time_and_validate_control_and_verification() {
    let mut payload = [0u8; 14];
    payload[0] = 1;
    payload[1..4].copy_from_slice(&[50, 1, 0]);
    payload[10..].fill(0x2F);
    for (control, operation) in [(1, ClockOperation::Add), (2, ClockOperation::Subtract)] {
        payload[0] = control;
        let bytes = telegram(0x6D, 12, &payload);
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected clock adjustment")
        };
        assert_eq!(
            message.decode().unwrap(),
            ApplicationPayload::Clock {
                operation,
                value: DataType::Time(
                    SingleEveryOrInvalid::Single(50),
                    SingleEveryOrInvalid::Single(1),
                    SingleEveryOrInvalid::Single(0)
                ),
            }
        );
    }
    payload[0] = 0;
    payload[1..7].copy_from_slice(&[0x36, 0x35, 0xC0, 0x53, 0x39, 0x00]);
    let bytes = telegram(0x6C, 12, &payload);
    let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
    else {
        panic!("expected clock setting")
    };
    assert!(matches!(
        message.decode().unwrap(),
        ApplicationPayload::Clock {
            operation: ClockOperation::Set,
            value: DataType::DateTimeWithSeconds(..)
        }
    ));
    for index in [0, 7, 10] {
        let mut invalid = payload;
        invalid[index] ^= 1;
        assert!(parse_application_layer(&telegram(0x6C, 12, &invalid)).is_err());
    }
}

#[test]
fn encrypted_protocol_payload_is_not_interpreted_before_decryption() {
    let mut bytes = telegram(0x6D, 12, &[0; 16]);
    bytes[11] = 0x10;
    bytes[12] = 0x05;
    let block = parse_application_layer(&bytes).unwrap();
    assert_eq!(block.is_encrypted(), Some(true));
    assert_eq!(block.variable_data_len(), 16);
    let UserDataBlock::ApplicationMessage(message) = block else {
        panic!("expected encrypted clock")
    };
    assert_eq!(
        message.decode(),
        Err(ApplicationLayerError::EncryptedPayload)
    );
    assert!(message.decode_payload(&[0; 16]).is_err());
    let plaintext = [
        0x2F, 0x2F, 1, 50, 1, 0, 0, 0, 0, 0, 0, 0, 0x2F, 0x2F, 0x2F, 0x2F,
    ];
    assert!(matches!(
        message.decode_payload(&plaintext).unwrap(),
        ApplicationPayload::Clock {
            operation: ClockOperation::Add,
            ..
        }
    ));
}

#[test]
fn compact_and_format_wrappers_use_the_expected_transport_headers() {
    let descriptors = [2, 2, 2, 0x15, 2, 0x2A];
    let mut format = vec![8];
    format.extend_from_slice(&format_signature(&descriptors).to_le_bytes());
    format.extend_from_slice(&descriptors);
    for (ci, size) in [(0x69, 0), (0x6A, 4), (0x6B, 12)] {
        let bytes = telegram(ci, size, &format);
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected format frame")
        };
        assert!(matches!(
            message.decode().unwrap(),
            ApplicationPayload::Format(_)
        ));
        assert_eq!(message.transport_header.size(), size);
    }
    for (ci, size) in [(0x79, 0), (0x7B, 4)] {
        let bytes = telegram(ci, size, &[1, 2, 3, 4, 5, 6]);
        let UserDataBlock::ApplicationMessage(message) = parse_application_layer(&bytes).unwrap()
        else {
            panic!("expected compact frame")
        };
        assert!(matches!(
            message.decode().unwrap(),
            ApplicationPayload::Compact(_)
        ));
        assert_eq!(message.transport_header.size(), size);
    }
}
