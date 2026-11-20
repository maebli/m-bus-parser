use m_bus_application_layer::{parse_application_layer, parse_data_records};

#[test]
fn reserved_data_information_byte_is_skipped_before_the_next_record() {
    let data = [0x3F, 0x01, 0x13, 0x2A];
    let mut records = parse_data_records(&data);
    assert_eq!(records.next().unwrap().unwrap().raw_bytes(), &data[1..]);
    assert!(records.next().is_none());
}

#[test]
fn ell_ii_payload_starts_after_the_session_number_and_crc() {
    let telegram = [0x8D, 0x00, 0x27, 1, 2, 3, 4, 0x34, 0x12, 0x01, 0x13, 0x2A];
    let block = parse_application_layer(&telegram).unwrap();
    let (header, payload) = block.transport_payload().unwrap();
    assert_eq!(header.access_number, 0x27);
    assert_eq!(payload, &[0x01, 0x13, 0x2A]);
}

#[test]
fn short_transport_aliases_preserve_the_header_and_payload() {
    for ci in 0xA0..=0xAF {
        let mut telegram = vec![ci];
        if ci == 0xA0 {
            telegram.push(0x00);
        }
        telegram.extend_from_slice(&[0x27, 0, 0, 0, 0x01, 0x13, 0x2A]);
        let block = parse_application_layer(&telegram).unwrap();
        let (header, payload) = block.transport_payload().unwrap();
        assert_eq!(header.control_information, ci);
        assert_eq!(header.access_number, 0x27);
        assert_eq!(payload, &[0x01, 0x13, 0x2A]);
    }
}
