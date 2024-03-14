#[derive(Debug, Clone, PartialEq)]
pub struct DataInformation {
    pub storage_number: u64,
    pub function_field: FunctionField,
    pub data_field_coding: DataFieldCoding,
    pub data_information_extension: Option<DataInformationExtension>,
    pub size: usize,
}

const MAXIMUM_DATA_INFORMATION_SIZE: usize = 11;

#[derive(Debug, Clone, PartialEq)]
pub struct DataInformationExtension {}

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
                Some(DataInformationExtension {})
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialFunctions {
    ManufacturerSpecific,
    MoreRecordsFollow,
    IdleFiller,
    Reserved,
    GlobalReadoutRequest,
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
