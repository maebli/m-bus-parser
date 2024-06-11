use super::{
    data_information::{Data, DataInformation, DataInformationBlock},
    value_information::{ValueInformation, ValueInformationBlock},
    variable_user_data::DataRecordError,
};
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq)]
pub struct RawDataRecordHeader {
    pub data_information_block: DataInformationBlock,
    pub value_information_block: ValueInformationBlock,
}
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq)]
pub struct ProcessedDataRecordHeader {
    pub data_information: DataInformation,
    pub value_information: ValueInformation,
}
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq)]
pub struct DataRecord {
    pub data_record_header: DataRecordHeader,
    pub data: Data,
}

impl DataRecord {
    pub fn get_size(&self) -> usize {
        self.data_record_header.get_size() + self.data.get_size()
    }
}
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq)]
pub struct DataRecordHeader {
    pub raw_data_record_header: RawDataRecordHeader,
    pub processed_data_record_header: ProcessedDataRecordHeader,
}

impl DataRecordHeader {
    pub fn get_size(&self) -> usize {
        self.raw_data_record_header
            .data_information_block
            .get_size()
            + self
                .raw_data_record_header
                .value_information_block
                .get_size()
    }
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
        let offset = data_record_header
            .raw_data_record_header
            .data_information_block
            .get_size()
            + data_record_header
                .raw_data_record_header
                .value_information_block
                .get_size();
        let data = data_record_header
            .processed_data_record_header
            .data_information
            .data_field_coding
            .parse(&data[offset..])?;
        Ok(DataRecord {
            data_record_header,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_raw_data_record() {
        let data = &[0x03, 0x13, 0x15, 0x31, 0x00];
        let _result = DataRecordHeader::try_from(data.as_slice());
    }
}
