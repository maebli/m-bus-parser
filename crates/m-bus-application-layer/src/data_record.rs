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
    pub value_information_block: Option<ValueInformationBlock<'a>>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ProcessedDataRecordHeader<'a> {
    pub data_information: Option<DataInformation>,
    pub value_information: Option<ValueInformation<'a>>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataRecord<'a> {
    pub data_record_header: DataRecordHeader<'a>,
    pub data: Data<'a>,
    /// Raw bytes encompassing this data record
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "m_bus_core::serde_hex::serialize")
    )]
    pub raw_bytes: &'a [u8],
}

impl<'a> DataRecord<'a> {
    /// Returns the parsed value carried by this record.
    #[must_use]
    pub fn value(&self) -> Option<&DataType<'_>> {
        self.data.value.as_ref()
    }

    /// Returns the processed data information (DIF and DIFE fields).
    #[must_use]
    pub fn data_information(&self) -> Option<&DataInformation> {
        self.data_record_header
            .processed_data_record_header
            .data_information
            .as_ref()
    }

    /// Returns the processed value information (VIF and VIFE fields).
    #[must_use]
    pub fn value_information(&self) -> Option<&ValueInformation<'a>> {
        self.data_record_header
            .processed_data_record_header
            .value_information
            .as_ref()
    }

    /// Returns all raw bytes consumed by this record.
    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        self.raw_bytes
    }

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
        let header_size = data_record_header.get_size();
        if data.len() < header_size {
            return Err(DataRecordError::InsufficientData);
        }
        let offset = header_size;
        let data_out = if let Some(data_info) = &data_record_header
            .processed_data_record_header
            .data_information
        {
            data_info.data_field_coding.parse(
                data.get(offset..)
                    .ok_or(DataRecordError::InsufficientData)?,
                fixed_data_header,
            )?
        } else {
            Data {
                value: Some(DataType::ManufacturerSpecific(
                    data.get(offset..)
                        .ok_or(DataRecordError::InsufficientData)?,
                )),
                size: data.len() - offset,
            }
        };

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
    pub processed_data_record_header: ProcessedDataRecordHeader<'a>,
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

        if !difb.data_information_field.is_special_function() {
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

impl<'a> TryFrom<&RawDataRecordHeader<'a>> for ProcessedDataRecordHeader<'a> {
    type Error = DataRecordError;
    fn try_from(raw_data_record_header: &RawDataRecordHeader<'a>) -> Result<Self, DataRecordError> {
        let mut value_information = None;
        let mut data_information = None;

        if let Some(x) = &raw_data_record_header.value_information_block {
            let v = ValueInformation::try_from(x)?;

            let mut d = DataInformation::try_from(&raw_data_record_header.data_information_block)?;

            // unfortunately, the data field coding is not always set in the data information block
            // so we must do some additional checks to determine the correct data field coding

            // Decode the lazy labels once, retaining the existing precedence
            // when more than one date/time label is present.
            let date_time_labels = v.labels().fold(0u8, |flags, label| {
                flags
                    | match label {
                        ValueLabel::Date => 1,
                        ValueLabel::DateTime => 2,
                        ValueLabel::Time => 4,
                        ValueLabel::DateTimeWithSeconds => 8,
                        _ => 0,
                    }
            });
            if date_time_labels & 1 != 0 {
                d.data_field_coding = DataFieldCoding::DateTypeG;
            } else if date_time_labels & 2 != 0 {
                // VIF 0x6D with a 6-byte data field is a type I date and time
                // (EN 13757-3), only the 4-byte variant is type F.
                d.data_field_coding = if d.data_field_coding == DataFieldCoding::Integer48Bit {
                    DataFieldCoding::DateTimeTypeI
                } else {
                    DataFieldCoding::DateTimeTypeF
                };
            } else if date_time_labels & 4 != 0 {
                d.data_field_coding = DataFieldCoding::DateTimeTypeJ;
            } else if date_time_labels & 8 != 0 {
                d.data_field_coding = DataFieldCoding::DateTimeTypeI;
            }

            value_information = Some(v);
            data_information = Some(d);
        } else if raw_data_record_header
            .data_information_block
            .data_information_field
            .is_special_function()
        {
            data_information = Some(DataInformation::try_from(
                &raw_data_record_header.data_information_block,
            )?);
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
    fn date_time_overrides_match_all_single_extension_vifs() {
        // Compare the single-pass classification with the original label
        // precedence for all VIF/VIFE bytes and the relevant DIF widths.
        for dif in [0x02, 0x04, 0x06, 0x09] {
            for vif in 0..=u8::MAX {
                // 0x7D is reserved and the existing VIF decoder rejects it
                // with an unreachable!(), rather than a parse error.
                if vif == 0x7d {
                    continue;
                }
                for vife in 0..=0x7f {
                    let bytes = [dif, vif, vife];
                    let Ok(raw) = RawDataRecordHeader::try_from(bytes.as_slice()) else {
                        continue;
                    };
                    let Ok(actual) = ProcessedDataRecordHeader::try_from(&raw) else {
                        continue;
                    };
                    let Some(value) = actual.value_information.as_ref() else {
                        continue;
                    };
                    let original = DataInformation::try_from(&raw.data_information_block)
                        .unwrap()
                        .data_field_coding;
                    let expected = if value.has_label(ValueLabel::Date) {
                        DataFieldCoding::DateTypeG
                    } else if value.has_label(ValueLabel::DateTime) {
                        if original == DataFieldCoding::Integer48Bit {
                            DataFieldCoding::DateTimeTypeI
                        } else {
                            DataFieldCoding::DateTimeTypeF
                        }
                    } else if value.has_label(ValueLabel::Time) {
                        DataFieldCoding::DateTimeTypeJ
                    } else if value.has_label(ValueLabel::DateTimeWithSeconds) {
                        DataFieldCoding::DateTimeTypeI
                    } else {
                        original
                    };
                    assert_eq!(
                        actual.data_information.unwrap().data_field_coding,
                        expected,
                        "DIF={dif:02x} VIF={vif:02x} VIFE={vife:02x}",
                    );
                }
            }
        }
    }

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
