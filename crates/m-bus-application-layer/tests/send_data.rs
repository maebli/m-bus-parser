use m_bus_application_layer::{
    data_information::{DataFieldCoding, DataType, SpecialFunctions},
    parse_application_layer, DataRecordError, UserDataBlock,
};

#[test]
fn send_data_decodes_identification_and_counter_writes_without_a_transport_header() {
    let data = [
        0x51, 0x0C, 0x79, 0x78, 0x56, 0x34, 0x12, 0x0C, 0x06, 0x07, 0x01, 0x00, 0x00,
    ];
    let block = parse_application_layer(&data).unwrap();
    let UserDataBlock::VariableDataStructureWithoutTplHeader {
        extended_link_layer,
        variable_data_block,
    } = &block
    else {
        panic!("expected records without a transport header");
    };
    assert!(extended_link_layer.is_none());
    assert_eq!(variable_data_block.as_ptr(), data[1..].as_ptr());
    assert_eq!(block.variable_data_len(), 12);

    let records = block
        .data_records()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].value(), Some(&DataType::Number(12_345_678.0)));
    assert_eq!(records[1].value(), Some(&DataType::Number(107.0)));
}

#[test]
fn send_data_exposes_readout_selection_records_without_values() {
    let data = [0x51, 0x08, 0x13, 0x08, 0x5A, 0x7F];
    let block = parse_application_layer(&data).unwrap();
    let records = block
        .data_records()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(records.len(), 3);
    for record in &records[..2] {
        assert_eq!(record.value(), None);
        assert_eq!(
            record.data_information().unwrap().data_field_coding,
            DataFieldCoding::SelectionForReadout
        );
    }
    assert_eq!(
        records[2].data_information().unwrap().data_field_coding,
        DataFieldCoding::SpecialFunctions(SpecialFunctions::GlobalReadoutRequest)
    );
}

#[test]
fn send_data_allows_an_empty_record_list() {
    let block = parse_application_layer(&[0x51]).unwrap();
    assert_eq!(block.variable_data_len(), 0);
    assert_eq!(block.data_records().unwrap().next(), None);
}

#[test]
fn send_data_reports_truncated_records_when_iterated() {
    let block = parse_application_layer(&[0x51, 0x01, 0x7A]).unwrap();
    let mut records = block.data_records().unwrap();
    assert_eq!(records.next(), Some(Err(DataRecordError::InsufficientData)));
    assert_eq!(records.next(), None);
}
