use super::{
    data_information::{
        Data, DataFieldCoding, DataInformation, DataInformationBlock, DataType, SpecialFunctions,
    },
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
    /// Parses the record at the start of `data` and sets `consumed` to how far
    /// a record stream must advance: the record's length or, after an error,
    /// its declared length if the header is readable, otherwise all of `data`.
    /// Reporting this here lets callers return the record without reading it
    /// again, so it can be built directly in their result.
    pub(crate) fn parse(
        data: &'a [u8],
        fixed_data_header: Option<&'a LongTplHeader>,
        consumed: &mut usize,
    ) -> Result<Self, DataRecordError> {
        *consumed = data.len();
        // This runs for every record, so keep large values out of extra stack
        // slots: both headers are borrowed from their results instead of moved
        // out with `?`, and the record is assembled only once everything parsed.
        let raw = RawDataRecordHeader::try_from(data);
        let raw_data_record_header = match &raw {
            Ok(header) => header,
            Err(error) => return Err(*error),
        };
        let processed = ProcessedDataRecordHeader::try_from(raw_data_record_header);
        let processed_data_record_header = match &processed {
            Ok(header) => header,
            Err(error) => return Err(*error),
        };
        let header_size = raw_data_record_header.get_size();
        let rest = data
            .get(header_size..)
            .ok_or(DataRecordError::InsufficientData)?;
        // Without a DIB coding the rest of the record is manufacturer specific;
        // decoding it through the same call avoids merging two `Data` values.
        let data_field_coding = processed_data_record_header
            .data_information
            .as_ref()
            .map_or(
                DataFieldCoding::SpecialFunctions(SpecialFunctions::ManufacturerSpecific),
                |data_info| data_info.data_field_coding,
            );
        let data_out = match data_field_coding.parse(rest, fixed_data_header) {
            Ok(data_out) => data_out,
            Err(error) => {
                // A record whose contents fail to decode must not cost us the
                // records behind it: step over it by its declared length.
                if let Some(size) = data_field_coding
                    .data_size(rest)
                    .and_then(|size| size.checked_add(header_size))
                    .filter(|&size| size != 0 && size <= data.len())
                {
                    *consumed = size;
                }
                return Err(error);
            }
        };
        // A data field longer than the input is truncated to the input.
        let raw_bytes = data
            .get(..header_size + data_out.get_size())
            .unwrap_or(data);
        *consumed = raw_bytes.len();

        Ok(Self::assemble(
            raw_data_record_header,
            processed_data_record_header,
            data_out,
            raw_bytes,
        ))
    }

    // Out of line: cloning the optional header fields needs temporaries that
    // should not widen the frame of `parse`, which is live during the deepest
    // calls of a decode.
    #[inline(never)]
    fn assemble(
        raw_data_record_header: &RawDataRecordHeader<'a>,
        processed_data_record_header: &ProcessedDataRecordHeader<'a>,
        data: Data<'a>,
        raw_bytes: &'a [u8],
    ) -> Self {
        DataRecord {
            data_record_header: DataRecordHeader {
                raw_data_record_header: raw_data_record_header.clone(),
                processed_data_record_header: processed_data_record_header.clone(),
            },
            data,
            raw_bytes,
        }
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
        self.raw_data_record_header.get_size()
    }
}

impl RawDataRecordHeader<'_> {
    pub(crate) fn get_size(&self) -> usize {
        let s = self.data_information_block.get_size();
        if let Some(x) = &self.value_information_block {
            s + x.get_size()
        } else {
            s
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for RawDataRecordHeader<'a> {
    type Error = DataRecordError;
    // Out of line: the temporaries of the block parsers are dead before the
    // deepest calls of a decode, so they should not widen the caller's frame.
    #[inline(never)]
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
        let value_information = match &raw_data_record_header.value_information_block {
            Some(x) => Some(ValueInformation::try_from(x)?),
            None if raw_data_record_header
                .data_information_block
                .data_information_field
                .is_special_function() =>
            {
                None
            }
            None => {
                return Ok(Self {
                    data_information: None,
                    value_information: None,
                })
            }
        };

        // One call site, read through the borrowed result: each `?` move or
        // extra call site would give `DataInformation` another stack slot.
        let data_information =
            DataInformation::try_from(&raw_data_record_header.data_information_block);
        let d = match &data_information {
            Ok(d) => d,
            Err(error) => return Err((*error).into()),
        };
        // unfortunately, the data field coding is not always set in the data information block
        // so we must do some additional checks to determine the correct data field coding
        let data_field_coding = match &value_information {
            Some(v) => date_time_coding(v, d.data_field_coding),
            None => d.data_field_coding,
        };

        Ok(Self {
            data_information: Some(DataInformation {
                data_field_coding,
                ..d.clone()
            }),
            value_information,
        })
    }
}

/// Returns the coding implied by date and time labels, which the DIF alone
/// does not always announce, or `coding` if there are none.
fn date_time_coding(v: &ValueInformation<'_>, coding: DataFieldCoding) -> DataFieldCoding {
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
        DataFieldCoding::DateTypeG
    } else if date_time_labels & 2 != 0 {
        // VIF 0x6D with a 6-byte data field is a type I date and time
        // (EN 13757-3), only the 4-byte variant is type F.
        if coding == DataFieldCoding::Integer48Bit {
            DataFieldCoding::DateTimeTypeI
        } else {
            DataFieldCoding::DateTimeTypeF
        }
    } else if date_time_labels & 4 != 0 {
        DataFieldCoding::DateTimeTypeJ
    } else if date_time_labels & 8 != 0 {
        DataFieldCoding::DateTimeTypeI
    } else {
        coding
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
        Self::parse(data, Some(fixed_data_header), &mut 0)
    }
}

impl<'a> TryFrom<&'a [u8]> for DataRecord<'a> {
    type Error = DataRecordError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::parse(data, None, &mut 0)
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
