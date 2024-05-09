use super::{
    data_information::DataInformationBlock, value_information::ValueInformationBlock,
    variable_user_data::DataRecordError,
};

#[derive(Debug, PartialEq)]
pub struct DataRecordHeader {
    pub data_information_block: DataInformationBlock,
    pub value_information_block: ValueInformationBlock,
}
#[derive(Debug, PartialEq)]
pub struct Data {}

#[derive(Debug, PartialEq)]
pub struct DataRecord {
    pub header: DataRecordHeader,
    pub data: Data,
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let header = DataRecordHeader {
            data_information_block: DataInformationBlock::try_from(data)?,
            value_information_block: ValueInformationBlock::try_from(data)?,
        };
        let data = Data {};
        Ok(DataRecord { header, data })
    }
}
