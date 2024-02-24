
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

enum ValueInformation {
    Primary,
    PlainText,
    Extended,
    Any,
    ManufacturerSpecific,
}
#[derive(Debug, Clone)]
struct ValueInformationBlock {
    value_information: ValueInformation,
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
pub enum Unit {
    Hms = 0x00,
    DMY = 0x01,
    Wh = 0x02,
    Wh1e1 = 0x03, // Wh * 10
    Wh1e2 = 0x04, // Wh * 100
    KWh = 0x05,
    KWh1e1 = 0x06, // kWh * 10
    KWh1e2 = 0x07, // kWh * 100
    MWh = 0x08,
    MWh1e1 = 0x09, // MWh * 10
    MWh1e2 = 0x0A, // MWh * 100
    KJ = 0x0B,
    KJ1e1 = 0x0C, // kJ * 10
    KJ1e2 = 0x0D, // kJ * 100
    MJ = 0x0E,
    MJ1e1 = 0x0F, // MJ * 10
    MJ1e2 = 0x10, // MJ * 100
    GJ = 0x11,
    GJ1e1 = 0x12, // GJ * 10
    GJ1e2 = 0x13, // GJ * 100
    W = 0x14,
    W1e1 = 0x15, // W * 10
    W1e2 = 0x16, // W * 100
    KW = 0x17,
    KW1e1 = 0x18, // kW * 10
    KW1e2 = 0x19, // kW * 100
    MW = 0x1A,
    MW1e1 = 0x1B, // MW * 10
    MW1e2 = 0x1C, // MW * 100
    KJH = 0x1D,
    KJH1e1 = 0x1E, // kJ/h * 10
    KJH1e2 = 0x1F, // kJ/h * 100
    MJH = 0x20,
    MJH1e1 = 0x21, // MJ/h * 10
    MJH1e2 = 0x22, // MJ/h * 100
    GJH = 0x23,
    GJH1e1 = 0x24, // GJ/h * 10
    GJH1e2 = 0x25, // GJ/h * 100
    Ml = 0x26,
    Ml1e1 = 0x27, // ml * 10
    Ml1e2 = 0x28, // ml * 100
    L = 0x29,
    L1e1 = 0x2A, // l * 10
    L1e2 = 0x2B, // l * 100
    M3 = 0x2C,
    M31e1 = 0x2D, // m^3 * 10
    M31e2 = 0x2E, // m^3 * 100
    MlH = 0x2F,
    MlH1e1 = 0x30, // ml/h * 10
    MlH1e2 = 0x31, // ml/h * 100
    LH = 0x32,
    LH1e1 = 0x33, // l/h * 10
    LH1e2 = 0x34, // l/h * 100
    M3H = 0x35,
    M3H1e1 = 0x36, // m^3/h * 10
    M3H1e2 = 0x37, // m^3/h * 100
    Celsius1e3 = 0x38, // °C * 10^-3
    UnitsForHCA = 0x39,
    Reserved3A = 0x3A,
    Reserved3B = 0x3B,
    Reserved3C = 0x3C,
    Reserved3D = 0x3D,
    SameButHistoric = 0x3E,
    WithoutUnits = 0x3F,
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

        DataInformationBlock { data_information: dif, data_information_extension: dife } 
    }
}

#[derive(Debug, Clone)]
struct DataRecord {
    function: FunctionField,
    storage_number: u32,
    unit: Unit,
    quantity: String,
    value: f64,
}

#[derive(Debug, PartialEq)]
pub enum VariableUserDataError{
}


fn parse_variable_data(data: &[u8]) -> Result<Vec<DataRecord>,VariableUserDataError> {
    let mut records = Vec::new();
    let mut data = data;
    while !data.is_empty() {
        let (header, data) = parse_data_record_header(data);
        let (data, record) = parse_data_record(header, data);
        records.push(record);
    }
    Ok(records)
}


mod tests {

    use super::*;
    #[test]
    fn test_parse_variable_data() {
        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        let data = vec![
            0x03, 0x13, 0x15, 0x31, 0x00
        ];
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));
    }

    fn test_parse_variable_data2(){
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let data = vec![
            0xDA, 0x02, 0x3B, 0x13, 0x01
        ];
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));
    }

    fn test_parse_variable_data3(){
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = vec![
            0x8B, 0x60, 0x04, 0x37, 0x18, 0x02
        ];
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));

    }
}
