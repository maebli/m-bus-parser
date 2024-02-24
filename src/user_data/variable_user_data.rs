
#[derive(Debug)]
struct DataRecordHeader {
    _data_information: DataInformationBlock,
    value_information: ValueInformationBlock,
}

#[derive(Debug, Clone)]
struct DataInformationBlock {
    data_information: u8,
    data_information_extension: Vec<u8>, 
}

#[derive(Debug, PartialEq)]
pub enum VIFExtension {
    CreditOfCurrencyUnits(u8),
    DebitOfCurrencyUnits(u8), 
    AccessNumber, 
    Medium, 
    Manufacturer, 
    ParameterSetIdentification, 
    ModelVersion, 
    HardwareVersion,
    FirmwareVersion,
    SoftwareVersion,
    CustomerLocation,
    Customer, 
    AccessCodeUser, 
    AccessCodeOperator, 
    AccessCodeSystemOperator, 
    AccessCodeDeveloper, 
    Password, 
    ErrorFlags,
    ErrorMask,
    Reserved,
    DigitalOutput,
    DigitalInput,
    BaudRate,
    ResponseDelayTime,
    Retry,
    FirstStorage,
    LastStorage,
    SizeOfStorage,
    StorageIntervalSecondsToDays(u8), 
    StorageIntervalMonths, 
    StorageIntervalYears, 
    DurationSinceLastReadout(u8),
    StartOfTariff,
    Volts(u8), 
    Ampere(u8), 
    EnergyMWh(u8), 
    EnergyGJ(u8), 
    VolumeM3(u8), 
    MassTons(u8), 
    VolumeFeet3, 
    VolumeAmericanGallon, 
    VolumeFlowAmericanGallonMin, 
    PowerMW(u8), 
    PowerGJH(u8), 
    FlowTemperature(u8), 
    ReturnTemperature(u8), 
}

#[derive(Debug, PartialEq)]
enum ValueInformation {
    Primary,
    PlainText,
    Extended(VIFExtension ),
    Any,
    ManufacturerSpecific,
}

impl ValueInformation {
    fn new(data:&[u8]) -> Self {
        match data[0] {
            0x00..=0x7B => ValueInformation::Primary,
            0x7C => ValueInformation::PlainText,
            0xFD => ValueInformation::Extended (
                match data[1] {
                    0x00..=0x03 => VIFExtension::CreditOfCurrencyUnits(0b11&data[1]),
                    0x04..=0x07 => VIFExtension::DebitOfCurrencyUnits(0b11&data[1]),
                    0x08 => VIFExtension::AccessNumber,
                    0x09 => VIFExtension::Medium,
                    0x0A => VIFExtension::Manufacturer,
                    0x0B => VIFExtension::ParameterSetIdentification,
                    0x0C => VIFExtension::ModelVersion,
                    0x0D => VIFExtension::HardwareVersion,
                    0x0E => VIFExtension::FirmwareVersion,
                    0x0F => VIFExtension::SoftwareVersion,
                    0x10 => VIFExtension::CustomerLocation,
                    0x11 => VIFExtension::Customer,
                    0x12 => VIFExtension::AccessCodeUser,
                    0x13 => VIFExtension::AccessCodeOperator,
                    0x14 => VIFExtension::AccessCodeSystemOperator,
                    0x15 => VIFExtension::AccessCodeDeveloper,
                    0x16 => VIFExtension::Password,
                    0x17 => VIFExtension::ErrorFlags,
                    0x18 => VIFExtension::ErrorMask,
                    0x19|0x25|0x28|0x32|0x33 => VIFExtension::Reserved,
                    0x20 => VIFExtension::DigitalOutput,
                    0x21 => VIFExtension::DigitalInput,
                    0x22 => VIFExtension::BaudRate,
                    0x23 => VIFExtension::ResponseDelayTime,
                    0x24 => VIFExtension::Retry,
                    0x26 => VIFExtension::LastStorage,
                    0x27 => VIFExtension::SizeOfStorage,
                    0x29 => VIFExtension::StorageIntervalSecondsToDays(0b11&data[1]),
                    0x30 => VIFExtension::StorageIntervalMonths,
                    0x31 => VIFExtension::StorageIntervalYears,
                    _ => {!unimplemented!("VIFExtension not implemented")};
                }

            ),
            0x7D | 0xFE => ValueInformation::Any,
            0x7E | 0xFF => ValueInformation::ManufacturerSpecific,
            _ => unreachable!(), 
        }
    }
}

#[derive(Debug,)]
struct ValueInformationBlock {
    value_information: ValueInformation,
    value_information_extension: Option<Vec<u8>>, 
}

#[derive(Debug, Clone, Copy,PartialEq)]
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

#[derive(Debug, Clone, Copy,PartialEq)]
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

impl DataInformationBlock {
}

#[derive(Debug, Clone,PartialEq)]
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
        let vif = ValueInformation::new(data);
    }
    Ok(records)
}


mod tests {

    use super::*;
    #[test]
    fn test_parse_vafriable_data() {
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
