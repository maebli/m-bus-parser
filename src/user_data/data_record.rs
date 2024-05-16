use crate::user_data::data_information::DataInformationField;

use super::{
    data_information::{DataInformation, DataInformationBlock},
    value_information::{ValueInformation, ValueInformationBlock},
    variable_user_data::DataRecordError,
};

#[derive(Debug, PartialEq)]
pub struct DataRecordHeader {
    pub data_information_block: DataInformationBlock,
    pub value_information_block: ValueInformationBlock,
}
#[derive(Debug, PartialEq)]
pub struct RawData {}
#[derive(Debug, PartialEq)]
pub struct Data {}

#[derive(Debug, PartialEq)]
pub struct DataRecord {
    pub raw_data_record: RawDataRecord,
    pub processed_data: ProcessedDataRecord,
}

#[derive(Debug, PartialEq)]
pub struct RawDataRecord {
    pub header: DataRecordHeader,
    pub data: RawData,
}

#[derive(Debug, PartialEq)]
pub struct ProcessedDataRecord {
    pub value_information: ValueInformation,
    pub data_information: DataInformation,
    pub data: Data,
}

impl TryFrom<&[u8]> for RawDataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<RawDataRecord, DataRecordError> {
        let header = DataRecordHeader {
            data_information_block: DataInformationBlock::try_from(data)?,
            value_information_block: ValueInformationBlock::try_from(data)?,
        };
        let data = RawData {};
        Ok(RawDataRecord { header, data })
    }
}
