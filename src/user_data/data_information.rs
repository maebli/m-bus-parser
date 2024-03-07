
#[derive(Debug, Clone, PartialEq)]
pub struct DataInformation {
    pub storage_number: u64,
    pub function_field: FunctionField,
    pub data_field_coding: DataFieldCoding,
    pub data_information_extension: Option<DataInformationExtension>,
    pub size: u32,
}

const MAXIMUM_DATA_INFORMATION_SIZE: usize = 11;

#[derive(Debug, Clone,PartialEq)]
struct DataInformationExtension{

}

#[derive(Debug, PartialEq)]
pub enum DataInformationError{
    NoData,
    DataTooLong
}

impl DataInformation {
    pub fn new(data:&[u8]) -> Result<Self,DataInformationError> {

        let first_byte = *data.get(0).ok_or(DataInformationError::DataTooLong)?;

        let mut storage_number = ((first_byte & 0b0100_0000) >> 6) as u64;

        let mut extension_bit = data[0] & 0x80 != 0;
        let mut extension_index = 1;
        let mut tariff = 0;
        let mut sub_unit =  0;

        while extension_bit {
            let next_byte = *data.get(extension_index).ok_or(DataInformationError::DataTooLong)?;
            storage_number += (((next_byte & 0x0f) as u64) << ((extension_index * 4) + 1)) as u64;
            sub_unit += (((next_byte & 0x40) >> 6) as u32) << extension_index;
            tariff += (((next_byte & 0x30) >> 4) as u64) << (extension_index * 2);
            extension_bit = next_byte & 0x80 != 0;
            extension_index += 1;

            if extension_index > MAXIMUM_DATA_INFORMATION_SIZE {
                return Err(DataInformationError::DataTooLong);
            }

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
            0b1111 => DataFieldCoding::SpecialFunctions,
            _ => unreachable!(), // This case should never occur due to the 4-bit width
        };

        Ok(DataInformation {
            storage_number,
            function_field,
            data_field_coding,
            data_information_extension: if extension_bit {
                Some(DataInformationExtension{})
            } else {
                None
            },
            size: extension_index as u32,
        })
    }
}


#[derive(Debug, Clone, Copy,PartialEq)]
pub enum FunctionField {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[derive(Debug, Clone, Copy,PartialEq)]
enum DataFieldCoding {
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
    SpecialFunctions,
}

#[derive(Debug, Clone, Copy,PartialEq)]
pub enum Unit {
    Hms = 0x00,
    DMY = 0x01,
    Wh = 0x02,
    Wh1e1 = 0x03, 
    Wh1e2 = 0x04, 
    KWh = 0x05,
    KWh1e1 = 0x06,
    KWh1e2 = 0x07,
    MWh = 0x08,
    MWh1e1 = 0x09,
    MWh1e2 = 0x0A,
    KJ = 0x0B,
    KJ1e1 = 0x0C, 
    KJ1e2 = 0x0D, 
    MJ = 0x0E,
    MJ1e1 = 0x0F, 
    MJ1e2 = 0x10, 
    GJ = 0x11,
    GJ1e1 = 0x12, 
    GJ1e2 = 0x13, 
    W = 0x14,
    W1e1 = 0x15, 
    W1e2 = 0x16, 
    KW = 0x17,
    KW1e1 = 0x18,
    KW1e2 = 0x19,
    MW = 0x1A,
    MW1e1 = 0x1B,
    MW1e2 = 0x1C,
    KJH = 0x1D,
    KJH1e1 = 0x1E,
    KJH1e2 = 0x1F,
    MJH = 0x20,
    MJH1e1 = 0x21,
    MJH1e2 = 0x22,
    GJH = 0x23,
    GJH1e1 = 0x24,
    GJH1e2 = 0x25,
    Ml = 0x26,
    Ml1e1 = 0x27, 
    Ml1e2 = 0x28, 
    L = 0x29,
    L1e1 = 0x2A, 
    L1e2 = 0x2B, 
    M3 = 0x2C,
    M31e1 = 0x2D,
    M31e2 = 0x2E,
    MlH = 0x2F,
    MlH1e1 = 0x30,
    MlH1e2 = 0x31,
    LH = 0x32,
    LH1e1 = 0x33, 
    LH1e2 = 0x34, 
    M3H = 0x35,
    M3H1e1 = 0x36,
    M3H1e2 = 0x37,
    Celsius1e3 = 0x38,
    UnitsForHCA = 0x39,
    Reserved3A = 0x3A,
    Reserved3B = 0x3B,
    Reserved3C = 0x3C,
    Reserved3D = 0x3D,
    SameButHistoric = 0x3E,
    WithoutUnits = 0x3F,
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_data_information() {
        let data = vec![0x13, 0x15, 0x31, 0x00];
        let result = DataInformation::new(&data);
        assert_eq!(result, Ok(DataInformation{
            storage_number: 0,
            function_field: FunctionField::MaximumValue,
            data_field_coding: DataFieldCoding::Integer24Bit,
            data_information_extension: None,
            size: 1,
        }));
    }

    #[test]
    fn test_invalid_data_information(){
        let data = vec![0xFF, 0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF];
        let result = DataInformation::new(&data);
        assert_eq!(result, Err(DataInformationError::DataTooLong));
    }
}

