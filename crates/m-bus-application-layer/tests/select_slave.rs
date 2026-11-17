use m_bus_application_layer::{
    parse_application_layer, ApplicationLayerError, SecondaryAddressMask, UserDataBlock,
};

#[test]
fn select_slave_preserves_the_libmbus_address_layout() {
    let block =
        parse_application_layer(&[0x52, 0x01, 0x10, 0x49, 0x14, 0x10, 0x57, 0x01, 0x06]).unwrap();
    let UserDataBlock::SelectSlave { secondary_address } = &block else {
        panic!("expected a slave selection");
    };
    assert_eq!(
        *secondary_address,
        SecondaryAddressMask {
            identification_number_bcd: [0x01, 0x10, 0x49, 0x14],
            manufacturer_bytes: [0x10, 0x57],
            version: 1,
            device_type: 6,
        }
    );
    assert_eq!(secondary_address.to_string(), "1449100110570106");
    assert!(block.data_records().is_none());
    assert_eq!(block.variable_data_len(), 0);
    assert_eq!(block.is_encrypted(), None);
}

#[test]
fn select_slave_preserves_digit_and_field_wildcards() {
    for (payload, expected) in [
        (
            [0x52, 0x78, 0xF6, 0xF4, 0x12, 0xFF, 0xFF, 0x01, 0xFF],
            "12F4F678FFFF01FF",
        ),
        (
            [0x52, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            "FFFFFFFFFFFFFFFF",
        ),
        (
            [0x52, 0x01, 0x00, 0x00, 0x00, 0x2D, 0x2C, 0x00, 0x07],
            "000000012D2C0007",
        ),
    ] {
        let UserDataBlock::SelectSlave { secondary_address } =
            parse_application_layer(&payload).unwrap()
        else {
            panic!("expected a slave selection");
        };
        assert_eq!(secondary_address.to_string(), expected);
    }
}

#[test]
fn select_slave_requires_exactly_eight_address_bytes() {
    let payload = [0x52, 0x01, 0x10, 0x49, 0x14, 0x10, 0x57, 0x01, 0x06, 0xFF];
    for length in (1..9).chain([10]) {
        assert_eq!(
            parse_application_layer(&payload[..length]),
            Err(ApplicationLayerError::InvalidDataLength {
                expected: 9,
                actual: length,
            })
        );
    }
}
