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

impl TryFrom<&RawDataRecord> for ProcessedDataRecord {
    type Error = DataRecordError;
    fn try_from(raw_data_record: &RawDataRecord) -> Result<ProcessedDataRecord, DataRecordError> {
        let value_information =
            ValueInformation::try_from(&raw_data_record.header.value_information_block)?;
        let data_information =
            DataInformation::try_from(&raw_data_record.header.data_information_block)?;
        let data = Data {};
        Ok(ProcessedDataRecord {
            value_information,
            data_information,
            data,
        })
    }
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let raw_data_record = RawDataRecord::try_from(data)?;
        let processed_data_record = ProcessedDataRecord::try_from(&raw_data_record)?;
        Ok(DataRecord {
            raw_data_record,
            processed_data: processed_data_record,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_data::data_information::DataInformationField;
    use crate::user_data::value_information::ValueInformationField;

    #[test]
    fn test_parse_raw_data_record() {
        let data = &[0x03, 0x13, 0x15, 0x31, 0x00];
        let raw_data_record = RawDataRecord::try_from(data.as_slice());
        let processed_data_record = ProcessedDataRecord::try_from(&raw_data_record.unwrap());
    }
}
