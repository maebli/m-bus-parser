use super::{
    data_information::{Data, DataFieldCoding, DataInformation, DataInformationBlock},
    value_information::{ValueInformation, ValueInformationBlock, ValueLabel},
    variable_user_data::DataRecordError,
};
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub struct RawDataRecordHeader<'a> {
    pub data_information_block: DataInformationBlock<'a>,
    pub value_information_block: ValueInformationBlock,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ProcessedDataRecordHeader {
    pub data_information: DataInformation,
    pub value_information: ValueInformation,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub struct DataRecord<'a> {
    pub data_record_header: DataRecordHeader<'a>,
    pub data: Data<'a>,
}

impl DataRecord<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        self.data_record_header.get_size() + self.data.get_size()
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub struct DataRecordHeader<'a> {
    pub raw_data_record_header: RawDataRecordHeader<'a>,
    pub processed_data_record_header: ProcessedDataRecordHeader,
}

impl DataRecordHeader<'_> {
    #[must_use]
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

impl<'a> TryFrom<&'a [u8]> for RawDataRecordHeader<'a> {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<RawDataRecordHeader, DataRecordError> {
        let difb = DataInformationBlock::try_from(data)?;
        let offset = difb.get_size();
        let vifb = ValueInformationBlock::try_from(
            data.get(offset..)
                .ok_or(DataRecordError::InsufficientData)?,
        )?;
        Ok(RawDataRecordHeader {
            data_information_block: difb,
            value_information_block: vifb,
        })
    }
}

impl<'a> TryFrom<&RawDataRecordHeader<'a>> for ProcessedDataRecordHeader {
    type Error = DataRecordError;
    fn try_from(raw_data_record_header: &RawDataRecordHeader) -> Result<Self, DataRecordError> {
        let value_information =
            ValueInformation::try_from(&raw_data_record_header.value_information_block)?;
        let mut data_information =
            DataInformation::try_from(&raw_data_record_header.data_information_block)?;

        // unfortunately, the data field coding is not always set in the data information block
        // so we must do some additional checks to determine the correct data field coding

        if value_information.labels.contains(&ValueLabel::Date) {
            data_information.data_field_coding = DataFieldCoding::DateTypeG;
        } else if value_information.labels.contains(&ValueLabel::DateTime) {
            data_information.data_field_coding = DataFieldCoding::DateTimeTypeF;
        } else if value_information.labels.contains(&ValueLabel::Time) {
            data_information.data_field_coding = DataFieldCoding::DateTimeTypeJ;
        } else if value_information
            .labels
            .contains(&ValueLabel::DateTimeWithSeconds)
        {
            data_information.data_field_coding = DataFieldCoding::DateTimeTypeI;
        }
        Ok(Self {
            data_information,
            value_information,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for DataRecordHeader<'a> {
    type Error = DataRecordError;
    fn try_from(data: &'a [u8]) -> Result<Self, DataRecordError> {
        let raw_data_record_header = RawDataRecordHeader::try_from(data)?;
        let processed_data_record_header =
            ProcessedDataRecordHeader::try_from(&raw_data_record_header)?;
        Ok(Self {
            raw_data_record_header,
            processed_data_record_header,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for DataRecord<'a> {
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
            .parse(
                data.get(offset..)
                    .ok_or(DataRecordError::InsufficientData)?,
            )?;
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
