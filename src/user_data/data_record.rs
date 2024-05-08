use super::{
    data_information::DataInformationBlock, value_information::ValueInformationBlock,
    variable_user_data::DataRecordError,
};

#[derive(Debug, PartialEq)]
pub struct DataRecordHeader {
    pub _data_information_block: DataInformationBlock,
    pub _value_information_block: ValueInformationBlock,
}
#[derive(Debug, PartialEq)]
pub struct Data {}

#[derive(Debug, PartialEq)]
pub struct DataRecord {
    pub _header: DataRecordHeader,
    pub _data: Data,
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let _header = DataRecordHeader {
            _data_information_block: None,
            _value_information_block: ValueInformationBlock::try_from(data).unwrap(),
        };
        let _data = Data {};
        Ok(DataRecord { _header, _data })
    }
}
