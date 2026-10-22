use crate::{
    data_information::{DataFieldCoding, SpecialFunctions},
    data_record::{ProcessedDataRecordHeader, RawDataRecordHeader},
    ApplicationLayerError, DataRecordError,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatFrame<'a> {
    pub length: u8,
    pub signature: u16,
    pub descriptors: &'a [u8],
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactFrame<'a> {
    pub signature: u16,
    pub full_frame_crc: u16,
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct FormatRecords<'a> {
    remaining: &'a [u8],
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct FormatRecord<'a> {
    pub raw_bytes: &'a [u8],
    pub header: ProcessedDataRecordHeader<'a>,
}

impl<'a> Iterator for FormatRecords<'a> {
    type Item = Result<FormatRecord<'a>, DataRecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let result = (|| {
            let raw = RawDataRecordHeader::try_from(self.remaining)?;
            let header = ProcessedDataRecordHeader::try_from(&raw)?;
            let size = raw.get_size();
            let bytes = self
                .remaining
                .get(..size)
                .ok_or(DataRecordError::InsufficientData)?;
            self.remaining = self
                .remaining
                .get(size..)
                .ok_or(DataRecordError::InsufficientData)?;
            Ok(FormatRecord {
                raw_bytes: bytes,
                header,
            })
        })();
        if result.is_err() {
            self.remaining = &[];
        }
        Some(result)
    }
}

impl<'a> FormatFrame<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ApplicationLayerError> {
        let [length, low, high, rest @ ..] = data else {
            return Err(ApplicationLayerError::InsufficientData);
        };
        let descriptors = if *length == 0 {
            rest
        } else {
            let count = usize::from(*length).checked_sub(2).ok_or(
                ApplicationLayerError::InvalidPayload {
                    field: "format length",
                },
            )?;
            let descriptors = rest
                .get(..count)
                .ok_or(ApplicationLayerError::InsufficientData)?;
            let padding = rest
                .get(count..)
                .ok_or(ApplicationLayerError::InsufficientData)?;
            if padding.iter().any(|byte| *byte != 0x2F) {
                return Err(ApplicationLayerError::InvalidPayload {
                    field: "format padding",
                });
            }
            descriptors
        };
        let frame = Self {
            length: *length,
            signature: u16::from_le_bytes([*low, *high]),
            descriptors,
        };
        let actual = format_signature(descriptors);
        if frame.signature != actual {
            return Err(ApplicationLayerError::ChecksumMismatch {
                expected: frame.signature,
                actual,
            });
        }
        for record in frame.records() {
            record.map_err(|_| ApplicationLayerError::InvalidPayload {
                field: "format descriptor",
            })?;
        }
        Ok(frame)
    }

    pub fn records(&self) -> FormatRecords<'a> {
        FormatRecords {
            remaining: self.descriptors,
        }
    }
}

impl<'a> CompactFrame<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ApplicationLayerError> {
        let [signature_low, signature_high, crc_low, crc_high, rest @ ..] = data else {
            return Err(ApplicationLayerError::InsufficientData);
        };
        Ok(Self {
            signature: u16::from_le_bytes([*signature_low, *signature_high]),
            full_frame_crc: u16::from_le_bytes([*crc_low, *crc_high]),
            data: rest,
        })
    }

    pub fn expand(
        &self,
        format: &FormatFrame<'_>,
        output: &mut [u8],
    ) -> Result<usize, ApplicationLayerError> {
        if self.signature != format.signature
            || format.signature != format_signature(format.descriptors)
        {
            return Err(ApplicationLayerError::FormatSignatureMismatch);
        }
        let mut remaining = self.data;
        let mut written: usize = 0;
        for record in format.records() {
            let record = record.map_err(|_| ApplicationLayerError::InvalidPayload {
                field: "format descriptor",
            })?;
            let coding = record
                .header
                .data_information
                .as_ref()
                .map(|information| information.data_field_coding)
                .unwrap_or(DataFieldCoding::SpecialFunctions(
                    SpecialFunctions::ManufacturerSpecific,
                ));
            let size = coding
                .data_size(remaining)
                .ok_or(ApplicationLayerError::InsufficientData)?;
            let value = remaining
                .get(..size)
                .ok_or(ApplicationLayerError::InsufficientData)?;
            for bytes in [record.raw_bytes, value] {
                let end = written
                    .checked_add(bytes.len())
                    .ok_or(ApplicationLayerError::OutputBufferTooSmall)?;
                output
                    .get_mut(written..end)
                    .ok_or(ApplicationLayerError::OutputBufferTooSmall)?
                    .copy_from_slice(bytes);
                written = end;
            }
            remaining = remaining
                .get(size..)
                .ok_or(ApplicationLayerError::InsufficientData)?;
        }
        if remaining.iter().any(|byte| *byte != 0x2F) {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "compact trailing data",
            });
        }
        let actual = format_signature(
            output
                .get(..written)
                .ok_or(ApplicationLayerError::OutputBufferTooSmall)?,
        );
        if self.full_frame_crc != actual {
            return Err(ApplicationLayerError::ChecksumMismatch {
                expected: self.full_frame_crc,
                actual,
            });
        }
        Ok(written)
    }
}

pub fn format_signature(bytes: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x3D65
            } else {
                crc << 1
            };
        }
    }
    !crc
}
