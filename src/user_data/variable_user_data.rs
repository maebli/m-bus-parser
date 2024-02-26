
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
    SizeOfStorageBlock,
    StorageIntervalSecondsToDays(u8), 
    StorageIntervalMonths, 
    StorageIntervalYears, 
    DurationSinceLastReadout(u8),
    StartOfTariff,
    DurationOfTariff(u8),
    PeriodOfTariff(u8),
    PeriodOfTarrifMonths,
    PeriodOfTTariffYears,
    Dimensionless,
    Volts(u8), 
    Ampere(u8), 
    ResetCounter,
    CumulationCounter,
    ControlSignal,
    DayOfWeek,
    WeekNumber,
    TimePointOfDay,
    StateOfParameterActivation,
    SpecialSupervision,
    DurationSinceLastCumulation(u8),
    OperatingTimeBattery(u8),
    DateAndTimeOfBatteryChange,
    EnergyMWh(u8), 
    EnergyGJ(u8), 
    VolumeM3(u8), 
    MassTons(u8), 
    VolumeFeet3Tenth, 
    VolumeAmericanGallonTenth,
    VolumeAmericanGallon,
    VolumeFlowAmericanGallonPerMinuteThousandth, 
    VolumeFlowAmericanGallonPerMinute,
    VolumeFlowAmericanGallonPerHour,
    PowerMW(u8), 
    PowerGJH(u8), 
    FlowTemperature(u8), 
    ReturnTemperature(u8),
    TemperatureDifference(u8),
    ExternalTemperature(u8),
    ColdWarmTemperatureLimitFarenheit(u8),
    ColdWarmTemperatureLimitCelsius(u8),
    CumulativeCountMaxPower(u8),
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
                    0x1A => VIFExtension::DigitalOutput,
                    0x1B => VIFExtension::DigitalInput,
                    0x1C => VIFExtension::BaudRate,
                    0x1D => VIFExtension::ResponseDelayTime,
                    0x1E => VIFExtension::Retry,
                    0x20 => VIFExtension::FirstStorage,
                    0x21 => VIFExtension::LastStorage,
                    0x22 => VIFExtension::SizeOfStorageBlock,
                    0x23..=0x26 => VIFExtension::StorageIntervalSecondsToDays(0b11&data[1]),
                    0x28 => VIFExtension::StorageIntervalMonths,
                    0x29 => VIFExtension::StorageIntervalYears,
                    0x2C..=0x2F => VIFExtension::DurationSinceLastReadout(0b11&data[1]),
                    0x30 => VIFExtension::StartOfTariff,
                    0x31..=0x33 => VIFExtension::DurationOfTariff(0b11&data[1]),
                    0x34..=0x37 => VIFExtension::PeriodOfTariff(0b11&data[1]),
                    0x38 => VIFExtension::PeriodOfTarrifMonths,
                    0x39 => VIFExtension::PeriodOfTTariffYears,
                    0x3A => VIFExtension::Dimensionless,
                    0x40..=0x47 => VIFExtension::Volts(0b1111&data[1]),
                    0x48..=0x4F => VIFExtension::Ampere(0b1111&data[1]),
                    0x60 => VIFExtension::ResetCounter,
                    0x61 => VIFExtension::CumulationCounter,
                    0x62 => VIFExtension::ControlSignal,
                    0x63 => VIFExtension::DayOfWeek,
                    0x64 => VIFExtension::WeekNumber,
                    0x65 => VIFExtension::TimePointOfDay,
                    0x66 => VIFExtension::StateOfParameterActivation,
                    0x67 => VIFExtension::SpecialSupervision,
                    0x68..=0x6B => VIFExtension::DurationSinceLastCumulation(0b11&data[1]),
                    0x6C..=0x6F => VIFExtension::OperatingTimeBattery(0b11&data[1]),
                    0x70 => VIFExtension::DateAndTimeOfBatteryChange,
                    _ => VIFExtension::Reserved,
                }

            ),
            0xFB => ValueInformation::Extended(
                match data[1] {
                    0x00|0x01 => VIFExtension::EnergyMWh(0b1&data[1]),
                    0x08|0x09 => VIFExtension::EnergyGJ(0b1&data[1]),
                    0x10|0x11 => VIFExtension::VolumeM3(0b1&data[1]),
                    0x18|0x19 => VIFExtension::MassTons(0b1&data[1]),
                    0x21 => VIFExtension::VolumeFeet3Tenth,
                    0x22 => VIFExtension::VolumeAmericanGallon,
                    0x23 => VIFExtension::VolumeFlowAmericanGallonPerMinuteThousandth,
                    0x24 => VIFExtension::VolumeFlowAmericanGallonPerMinute,
                    0x25 => VIFExtension::VolumeFlowAmericanGallonPerHour,
                    0x28|0x29 => VIFExtension::PowerMW(0b1&data[1]),
                    0x30|0x31 => VIFExtension::PowerGJH(0b1&data[1]),
                    0x50..=0x53 => VIFExtension::FlowTemperature(0b11&data[1]),
                    0x54..=0x57 => VIFExtension::ReturnTemperature(0b11&data[1]),
                    0x60..=0x63 => VIFExtension::TemperatureDifference(0b11&data[1]),
                    0x64..=0x67 => VIFExtension::ExternalTemperature(0b11&data[1]),
                    0x70..=0x73 => VIFExtension::ColdWarmTemperatureLimitFarenheit(0b11&data[1]),
                    0x74..=0x77 => VIFExtension::ColdWarmTemperatureLimitCelsius(0b11&data[1]),
                    0x78..=0x7F => VIFExtension::CumulativeCountMaxPower(0b111&data[1]),
                    _ => VIFExtension::Reserved,
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
pub struct DataRecord {
    function: FunctionField,
    storage_number: u32,
    unit: Unit,
    quantity: String,
    value: f64,
}

#[derive(Debug, PartialEq)]
pub enum VariableUserDataError{
}


pub fn parse_variable_data(data: &[u8]) -> Result<Vec<DataRecord>,VariableUserDataError> {
    let mut records = Vec::new();
    let mut data = data;
    let vif = ValueInformation::new(data);
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
