use arrayvec::ArrayVec;

use super::value_information::{ValueInformation, ValueInformationFieldExtension};

const MAX_DIFE_RECORDS: usize = 10;
#[derive(Debug, PartialEq)]
pub struct DataInformationBlock {
    pub _data_information_field: DataInformation,
    pub _data_information_field_extension:
        Option<ArrayVec<ValueInformationFieldExtension, MAX_DIFE_RECORDS>>,
}
#[derive(Debug, PartialEq)]
pub struct DataInformationField {
    pub data: u8,
}

impl From<u8> for DataInformationField {
    fn from(data: u8) -> Self {
        DataInformationField { data }
    }
}

impl From<u8> for DataInformationFieldExtension {
    fn from(data: u8) -> Self {
        DataInformationFieldExtension { data }
    }
}

pub struct DataInformationFieldExtension {
    pub data: u8,
}

impl TryFrom<&[u8]> for DataInformationBlock {
    type Error = DataInformationError;

    fn try_from(data: &[u8]) -> Result<Self, DataInformationError> {
        let dife = ArrayVec::<ValueInformationFieldExtension, MAX_DIFE_RECORDS>::new();
        let dif = DataInformationField::from(data[0]);

        if dif.has_extension() {
            let mut offset = 1;
            while offset < data.len() {
                let next_byte = *data.get(offset).ok_or(DataInformationError::DataTooShort)?;
                let dife = DataInformationFieldExtension::from(next_byte);
                if dife.has_extension() {
                    offset += 1;
                } else {
                    break;
                }
            }
        };
        Ok(DataInformationBlock {
            _data_information_field: DataInformation::try_from(data)?,
            _data_information_field_extension: if dife.is_empty() { None } else { Some(dife) },
        })
    }
}

impl DataInformationField {
    fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

impl DataInformationFieldExtension {
    fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataInformation {
    pub storage_number: u64,
    pub function_field: FunctionField,
    pub data_field_coding: DataFieldCoding,
    pub data_information_extension: Option<DataInformationExtensionField>,
    pub size: usize,
}

const MAXIMUM_DATA_INFORMATION_SIZE: usize = 11;

#[derive(Debug, Clone, PartialEq)]
pub struct DataInformationExtensionField {}

#[derive(Debug, PartialEq)]
pub enum DataInformationError {
    NoData,
    DataTooLong,
    DataTooShort,
}

impl TryFrom<&[u8]> for DataInformation {
    type Error = DataInformationError;

    fn try_from(data: &[u8]) -> Result<Self, DataInformationError> {
        let first_byte = *data.first().ok_or(DataInformationError::DataTooLong)?;

        let mut storage_number = ((first_byte & 0b0100_0000) >> 6) as u64;

        let mut extension_bit = data[0] & 0x80 != 0;
        let mut extension_index = 1;
        let mut _tariff = 0;
        let mut _sub_unit = 0;

        while extension_bit {
            if extension_index > MAXIMUM_DATA_INFORMATION_SIZE {
                return Err(DataInformationError::DataTooLong);
            }

            let next_byte = *data
                .get(extension_index)
                .ok_or(DataInformationError::DataTooShort)?;
            storage_number += ((next_byte & 0x0f) as u64) << ((extension_index * 4) + 1);
            _sub_unit += (((next_byte & 0x40) >> 6) as u32) << extension_index;
            _tariff += (((next_byte & 0x30) >> 4) as u64) << (extension_index * 2);
            extension_bit = next_byte & 0x80 != 0;
            extension_index += 1;
        }

        let function_field = match (data[0] & 0b0011_0000) >> 4 {
            0b00 => FunctionField::InstantaneousValue,
            0b01 => FunctionField::MaximumValue,
            0b10 => FunctionField::MinimumValue,
            _ => FunctionField::ValueDuringErrorState,
        };
        let data_field_coding = match data[0] & 0b0000_1111 {
            0b0000 => DataFieldCoding::NoData,
            0b0001 => DataFieldCoding::Integer8Bit,
            0b0010 => DataFieldCoding::Integer16Bit,
            0b0011 => DataFieldCoding::Integer24Bit,
            0b0100 => DataFieldCoding::Integer32Bit,
            0b0101 => DataFieldCoding::Real32Bit,
            0b0110 => DataFieldCoding::Integer48Bit,
            0b0111 => DataFieldCoding::Integer64Bit,
            0b1000 => DataFieldCoding::SelectionForReadout,
            0b1001 => DataFieldCoding::BCD2Digit,
            0b1010 => DataFieldCoding::BCD4Digit,
            0b1011 => DataFieldCoding::BCD6Digit,
            0b1100 => DataFieldCoding::BCD8Digit,
            0b1101 => DataFieldCoding::VariableLength,
            0b1110 => DataFieldCoding::BCDDigit12,
            0b1111 => DataFieldCoding::SpecialFunctions(match data[1] {
                0x0F => SpecialFunctions::ManufacturerSpecific,
                0x1F => SpecialFunctions::MoreRecordsFollow,
                0x2F => SpecialFunctions::IdleFiller,
                0x7F => SpecialFunctions::GlobalReadoutRequest,
                _ => SpecialFunctions::Reserved,
            }),
            _ => unreachable!(), // This case should never occur due to the 4-bit width
        };

        Ok(DataInformation {
            storage_number,
            function_field,
            data_field_coding,
            data_information_extension: if extension_bit {
                Some(DataInformationExtensionField {})
            } else {
                None
            },
            size: extension_index,
        })
    }
}

impl DataInformation {
    pub fn get_size(&self) -> usize {
        self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionField {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FunctionField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionField::InstantaneousValue => write!(f, "Instantaneous Value"),
            FunctionField::MaximumValue => write!(f, "Maximum Value"),
            FunctionField::MinimumValue => write!(f, "Minimum Value"),
            FunctionField::ValueDuringErrorState => write!(f, "Value During Error State"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialFunctions {
    ManufacturerSpecific,
    MoreRecordsFollow,
    IdleFiller,
    Reserved,
    GlobalReadoutRequest,
}

pub struct Value {
    pub data: f64,
    pub byte_size: usize,
}

impl DataFieldCoding {
    pub fn extract_from_bytes(&self, data: &[u8]) -> Value {
        match *self {
            DataFieldCoding::Real32Bit => Value {
                data: f32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64,
                byte_size: 4,
            },
            DataFieldCoding::Integer8Bit => Value {
                data: data[0] as f64,
                byte_size: 1,
            },
            DataFieldCoding::Integer16Bit => Value {
                data: ((data[1] as u16) << 8 | data[0] as u16) as f64,
                byte_size: 2,
            },
            DataFieldCoding::Integer24Bit => Value {
                data: ((data[2] as u32) << 16 | (data[1] as u32) << 8 | data[0] as u32) as f64,
                byte_size: 3,
            },
            DataFieldCoding::Integer32Bit => Value {
                data: ((data[3] as u32) << 24
                    | (data[2] as u32) << 16
                    | (data[1] as u32) << 8
                    | data[0] as u32) as f64,
                byte_size: 4,
            },
            DataFieldCoding::Integer48Bit => Value {
                data: ((data[5] as u64) << 40
                    | (data[4] as u64) << 32
                    | (data[3] as u64) << 24
                    | (data[2] as u64) << 16
                    | (data[1] as u64) << 8
                    | data[0] as u64) as f64,
                byte_size: 6,
            },
            DataFieldCoding::Integer64Bit => Value {
                data: ((data[7] as u64) << 56
                    | (data[6] as u64) << 48
                    | (data[5] as u64) << 40
                    | (data[4] as u64) << 32
                    | (data[3] as u64) << 24
                    | (data[2] as u64) << 16
                    | (data[1] as u64) << 8
                    | data[0] as u64) as f64,
                byte_size: 8,
            },
            DataFieldCoding::BCD2Digit => Value {
                data: ((data[0] >> 4) as f64 * 10.0) + (data[0] & 0x0F) as f64,
                byte_size: 1,
            },
            DataFieldCoding::BCD4Digit => Value {
                data: ((data[1] >> 4) as f64 * 1000.0)
                    + ((data[1] & 0x0F) as f64 * 100.0)
                    + ((data[0] >> 4) as f64 * 10.0)
                    + (data[0] & 0x0F) as f64,
                byte_size: 2,
            },
            DataFieldCoding::BCD6Digit => Value {
                data: ((data[2] >> 4) as f64 * 100000.0)
                    + ((data[2] & 0x0F) as f64 * 10000.0)
                    + ((data[1] >> 4) as f64 * 1000.0)
                    + ((data[1] & 0x0F) as f64 * 100.0)
                    + ((data[0] >> 4) as f64 * 10.0)
                    + (data[0] & 0x0F) as f64,
                byte_size: 3,
            },
            DataFieldCoding::BCD8Digit => Value {
                data: ((data[3] >> 4) as f64 * 10000000.0)
                    + ((data[3] & 0x0F) as f64 * 1000000.0)
                    + ((data[2] >> 4) as f64 * 100000.0)
                    + ((data[2] & 0x0F) as f64 * 10000.0)
                    + ((data[1] >> 4) as f64 * 1000.0)
                    + ((data[1] & 0x0F) as f64 * 100.0)
                    + ((data[0] >> 4) as f64 * 10.0)
                    + (data[0] & 0x0F) as f64,
                byte_size: 4,
            },
            DataFieldCoding::BCDDigit12 => Value {
                data: ((data[5] >> 4) as f64 * 100000000000.0)
                    + ((data[5] & 0x0F) as f64 * 10000000000.0)
                    + ((data[4] >> 4) as f64 * 1000000000.0)
                    + ((data[4] & 0x0F) as f64 * 100000000.0)
                    + ((data[3] >> 4) as f64 * 10000000.0)
                    + ((data[3] & 0x0F) as f64 * 1000000.0)
                    + ((data[2] >> 4) as f64 * 100000.0)
                    + ((data[2] & 0x0F) as f64 * 10000.0)
                    + ((data[1] >> 4) as f64 * 1000.0)
                    + ((data[1] & 0x0F) as f64 * 100.0)
                    + ((data[0] >> 4) as f64 * 10.0)
                    + (data[0] & 0x0F) as f64,
                byte_size: 6,
            },
            DataFieldCoding::NoData => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::SelectionForReadout => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::SpecialFunctions(_) => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::VariableLength => Value {
                data: 0.0,
                byte_size: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataFieldCoding {
    NoData,
    Integer8Bit,
    Integer16Bit,
    Integer24Bit,
    Integer32Bit,
    Real32Bit,
    Integer48Bit,
    Integer64Bit,
    SelectionForReadout,
    BCD2Digit,
    BCD4Digit,
    BCD6Digit,
    BCD8Digit,
    VariableLength,
    BCDDigit12,
    SpecialFunctions(SpecialFunctions),
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_data_information() {
        let data = [0x13 as u8];
        let result = DataInformation::try_from(data.as_slice());
        assert_eq!(
            result,
            Ok(DataInformation {
                storage_number: 0,
                function_field: FunctionField::MaximumValue,
                data_field_coding: DataFieldCoding::Integer24Bit,
                data_information_extension: None,
                size: 1,
            })
        );
    }

    #[test]
    fn test_invalid_data_information() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = DataInformation::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_longest_data_information_not_too_long() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = DataInformation::try_from(data.as_slice());
        assert_ne!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_short_data_information() {
        let data = [0xFF];
        let result = DataInformation::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooShort));
    }
}
