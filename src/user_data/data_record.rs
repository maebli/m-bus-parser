use super::{
    data_information::{DataInformation, DataInformationBlock},
    value_information::{ValueInformation, ValueInformationBlock},
    variable_user_data::DataRecordError,
};

#[derive(Debug, PartialEq)]
pub struct RawDataRecordHeader {
    pub data_information_block: DataInformationBlock,
    pub value_information_block: ValueInformationBlock,
}

#[derive(Debug, PartialEq)]
pub struct ProcessedDataRecordHeader {
    pub data_information: DataInformation,
    pub value_information: ValueInformation,
}

#[derive(Debug, PartialEq)]
pub struct DataRecord {
    pub value_information: ValueInformation,
    pub data_information: DataInformation,
    pub data: Data,
}

#[derive(Debug, PartialEq)]
pub struct RawData {}
#[derive(Debug, PartialEq)]
pub struct Data {}

#[derive(Debug, PartialEq)]
pub struct DataRecordHeader {
    pub raw_data_record_header: RawDataRecordHeader,
    pub processed_data_record_header: ProcessedDataRecordHeader,
}

impl TryFrom<&[u8]> for RawDataRecordHeader {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<RawDataRecordHeader, DataRecordError> {
        let difb = DataInformationBlock::try_from(data)?;
        let offset = difb.get_size();
        let vifb = ValueInformationBlock::try_from(&data[offset..])?;
        Ok(RawDataRecordHeader {
            data_information_block: difb,
            value_information_block: vifb,
        })
    }
}

impl TryFrom<&RawDataRecordHeader> for ProcessedDataRecordHeader {
    type Error = DataRecordError;
    fn try_from(
        raw_data_record_header: &RawDataRecordHeader,
    ) -> Result<ProcessedDataRecordHeader, DataRecordError> {
        let value_information =
            ValueInformation::try_from(&raw_data_record_header.value_information_block)?;
        let data_information =
            DataInformation::try_from(&raw_data_record_header.data_information_block)?;
        let data = Data {};
        Ok(ProcessedDataRecordHeader {
            value_information,
            data_information,
        })
    }
}

impl TryFrom<&[u8]> for DataRecordHeader {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecordHeader, DataRecordError> {
        let raw_data_record_header = RawDataRecordHeader::try_from(data)?;
        let processed_data_record_header =
            ProcessedDataRecordHeader::try_from(&raw_data_record_header)?;
        Ok(DataRecordHeader {
            raw_data_record_header,
            processed_data_record_header,
        })
    }
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let data_record_header = DataRecordHeader::try_from(data)?;
        let data = Data {};
        Ok(DataRecord {
            value_information: data_record_header
                .processed_data_record_header
                .value_information,
            data_information: data_record_header
                .processed_data_record_header
                .data_information,
            data,
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
        let result = DataRecordHeader::try_from(data.as_slice());
    }
}
