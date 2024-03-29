pub struct DataRecordHeader {
    pub _data_information_block: DataInformationBlock,
    pub _value_information_block: ValueInformationBlock,
}

pub struct DataRecord {
    pub _header: DataRecordHeader,
    pub _data: Data,
}
