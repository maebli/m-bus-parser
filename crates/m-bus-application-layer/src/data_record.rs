use super::{
    data_information::{Data, DataFieldCoding, DataInformation, DataInformationBlock, DataType},
    value_information::{ValueInformation, ValueInformationBlock, ValueLabel},
    variable_user_data::DataRecordError,
    LongTplHeader,
};
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RawDataRecordHeader<'a> {
    pub data_information_block: DataInformationBlock<'a>,
    pub value_information_block: Option<ValueInformationBlock>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ProcessedDataRecordHeader {
    pub data_information: Option<DataInformation>,
    pub value_information: Option<ValueInformation>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataRecord<'a> {
    pub data_record_header: DataRecordHeader<'a>,
    pub data: Data<'a>,
    /// Raw bytes encompassing this data record
    pub raw_bytes: &'a [u8],
}

impl DataRecord<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        self.raw_bytes.len()
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn data_record_header_hex(&self) -> String {
        let start = 0;
        let end = self.data_record_header.get_size();
        self.raw_bytes
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn data_hex(&self) -> String {
        let start = self.data_record_header.get_size();
        let end = self.get_size();
        self.raw_bytes
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl<'a> DataRecord<'a> {
    fn parse(
        data: &'a [u8],
        fixed_data_header: Option<&'a LongTplHeader>,
    ) -> Result<Self, DataRecordError> {
        let data_record_header = DataRecordHeader::try_from(data)?;
        let mut header_size = data_record_header.get_size();
        if data_record_header
            .raw_data_record_header
            .data_information_block
            .data_information_field
            .data
            == 0x0F
        {
            header_size = 0;
        }
        if data.len() < header_size {
            return Err(DataRecordError::InsufficientData);
        }
        let offset = header_size;
        let mut data_out = Data {
            value: Some(DataType::ManufacturerSpecific(
                data.get(offset..)
                    .ok_or(DataRecordError::InsufficientData)?,
            )),
            size: data.len() - offset,
        };
        if data_record_header
            .raw_data_record_header
            .value_information_block
            .is_some()
        {
            if let Some(data_info) = &data_record_header
                .processed_data_record_header
                .data_information
            {
                data_out = data_info.data_field_coding.parse(
                    data.get(offset..)
                        .ok_or(DataRecordError::InsufficientData)?,
                    fixed_data_header,
                )?;
            }
        }

        let mut record_size = data_record_header.get_size() + data_out.get_size();
        if record_size > data.len() {
            record_size = data.len();
        }
        let raw_bytes = data
            .get(..record_size)
            .ok_or(DataRecordError::InsufficientData)?;

        Ok(DataRecord {
            data_record_header,
            data: data_out,
            raw_bytes,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataRecordHeader<'a> {
    pub raw_data_record_header: RawDataRecordHeader<'a>,
    pub processed_data_record_header: ProcessedDataRecordHeader,
}

impl DataRecordHeader<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        let s = self
            .raw_data_record_header
            .data_information_block
            .get_size();
        if let Some(x) = &self.raw_data_record_header.value_information_block {
            s + x.get_size()
        } else {
            s
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for RawDataRecordHeader<'a> {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<RawDataRecordHeader<'_>, DataRecordError> {
        let difb = DataInformationBlock::try_from(data)?;
        let offset = difb.get_size();

        let mut vifb = None;

        if difb.data_information_field.data != 0x0F {
            vifb = Some(ValueInformationBlock::try_from(
                data.get(offset..)
                    .ok_or(DataRecordError::InsufficientData)?,
            )?);
        }

        Ok(RawDataRecordHeader {
            data_information_block: difb,
            value_information_block: vifb,
        })
    }
}

impl TryFrom<&RawDataRecordHeader<'_>> for ProcessedDataRecordHeader {
    type Error = DataRecordError;
    fn try_from(raw_data_record_header: &RawDataRecordHeader) -> Result<Self, DataRecordError> {
        let mut value_information = None;
        let mut data_information = None;

        if let Some(x) = &raw_data_record_header.value_information_block {
            let v = ValueInformation::try_from(x)?;

            let mut d = DataInformation::try_from(&raw_data_record_header.data_information_block)?;

            // unfortunately, the data field coding is not always set in the data information block
            // so we must do some additional checks to determine the correct data field coding

            if v.labels.contains(&ValueLabel::Date) {
                d.data_field_coding = DataFieldCoding::DateTypeG;
            } else if v.labels.contains(&ValueLabel::DateTime) {
                d.data_field_coding = DataFieldCoding::DateTimeTypeF;
            } else if v.labels.contains(&ValueLabel::Time) {
                d.data_field_coding = DataFieldCoding::DateTimeTypeJ;
            } else if v.labels.contains(&ValueLabel::DateTimeWithSeconds) {
                d.data_field_coding = DataFieldCoding::DateTimeTypeI;
            }

            value_information = Some(v);
            data_information = Some(d);
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

impl<'a> TryFrom<(&'a [u8], &'a LongTplHeader)> for DataRecord<'a> {
    type Error = DataRecordError;
    fn try_from(
        (data, fixed_data_header): (&'a [u8], &'a LongTplHeader),
    ) -> Result<Self, Self::Error> {
        Self::parse(data, Some(fixed_data_header))
    }
}

impl<'a> TryFrom<&'a [u8]> for DataRecord<'a> {
    type Error = DataRecordError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::parse(data, None)
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
    #[test]
    #[cfg(feature = "std")]
    fn test_manufacturer_specific_block() {
        let data = [0x0F, 0x01, 0x02, 0x03, 0x04];
        let result = DataRecord::try_from(data.as_slice());
        println!("{:?}", result);
    }
}
