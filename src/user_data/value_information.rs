
#[derive(Debug, PartialEq)]
pub enum ValueInformation {
    Primary,
    PlainText,
    Extended(VIFExtension ),
    Any,
    ManufacturerSpecific,
}

impl ValueInformation {
    pub fn new(data:&[u8]) -> Self {
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

#[derive(Debug,)]
struct ValueInformationBlock {
    value_information: ValueInformation,
    value_information_extension: Option<Vec<u8>>, 
}


mod tests{

    use super::*;

    #[test]
    fn test_value_information_new() {
        let data = vec![0x13];
        let result = ValueInformation::new(&data);
        assert_eq!(result, ValueInformation::Primary);
    }

}