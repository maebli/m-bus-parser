
#[derive(Debug, Clone)]
struct DataRecordHeader {
    data_information: DataInformationBlock,
    value_information: ValueInformationBlock,
}

#[derive(Debug, Clone)]
struct DataInformationBlock {
    data_information: u8,
    data_information_extension: Vec<u8>, 
}

#[derive(Debug, Clone)]
struct ValueInformationBlock {
    value_information: u8,
    value_information_extension: Vec<u8>, 
}

#[derive(Debug, Clone)]
struct DataRecord {
    header: DataRecordHeader,
    data: Vec<u8>, 
}

#[derive(Debug, Clone, Copy)]
enum FunctionField {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
struct DataInformationFieldExtension {
    extension_bit: bool,
    lsb_of_storage_number: bool,
    function_field: FunctionField,
    data_field_coding: DataFieldCoding,
}

impl DataInformationFieldExtension {
    fn new(byte: u8) -> Self {
        let extension_bit = byte & 0b1000_0000 != 0;
        let lsb_of_storage_number = byte & 0b0100_0000 != 0;
        let function_field = match (byte & 0b0011_0000) >> 4 {
            0b00 => FunctionField::InstantaneousValue,
            0b01 => FunctionField::MaximumValue,
            0b10 => FunctionField::MinimumValue,
            _ => FunctionField::ValueDuringErrorState,
        };
        let data_field_coding = match byte & 0b0000_1111 {
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

        DataInformationFieldExtension {
            extension_bit,
            lsb_of_storage_number,
            function_field,
            data_field_coding,
        }
    }
}

#[derive(Debug, Clone)]
struct DataInformationBlock {
    dif: DataInformationFieldExtension,
    dife: Vec<DataInformationFieldExtension>,
}

impl DataInformationBlock {
    fn new(dif_byte: u8, dife_bytes: Vec<u8>) -> Self {
        let dif = DataInformationFieldExtension::new(dif_byte);
        let dife = dife_bytes.into_iter().map(DataInformationFieldExtension::new).collect();

        DataInformationBlock { dif, dife }
    }
}
