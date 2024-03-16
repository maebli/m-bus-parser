use arrayvec::ArrayVec;

#[derive(Debug, PartialEq)]
pub enum ValueInformation {
    Primary(u8),
    PlainText,
    Extended(VIFExtension),
    Any,
    ManufacturerSpecific,
}

impl ValueInformation {
    pub fn get_size(&self) -> usize {
        match self {
            ValueInformation::Primary(_) => 1,
            ValueInformation::PlainText => 1,
            ValueInformation::Extended(_) => 2,
            ValueInformation::Any => 1,
            ValueInformation::ManufacturerSpecific => 1,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationError {
    InvalidValueInformation,
}

impl TryFrom<&[u8]> for ValueInformation {
    type Error = ValueInformationError;

    fn try_from(data: &[u8]) -> Result<Self, ValueInformationError> {
        Ok(match data[0] {
            0x00..=0x7B => ValueInformation::Primary(data[0]),
            0x7C => ValueInformation::PlainText,
            0xFD => ValueInformation::Extended(match data[1] {
                0x00..=0x03 => VIFExtension::CreditOfCurrencyUnits(0b11 & data[1]),
                0x04..=0x07 => VIFExtension::DebitOfCurrencyUnits(0b11 & data[1]),
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
                0x23..=0x26 => VIFExtension::StorageIntervalSecondsToDays(0b11 & data[1]),
                0x28 => VIFExtension::StorageIntervalMonths,
                0x29 => VIFExtension::StorageIntervalYears,
                0x2C..=0x2F => VIFExtension::DurationSinceLastReadout(0b11 & data[1]),
                0x30 => VIFExtension::StartOfTariff,
                0x31..=0x33 => VIFExtension::DurationOfTariff(0b11 & data[1]),
                0x34..=0x37 => VIFExtension::PeriodOfTariff(0b11 & data[1]),
                0x38 => VIFExtension::PeriodOfTarrifMonths,
                0x39 => VIFExtension::PeriodOfTTariffYears,
                0x3A => VIFExtension::Dimensionless,
                0x40..=0x47 => VIFExtension::Volts(0b1111 & data[1]),
                0x48..=0x4F => VIFExtension::Ampere(0b1111 & data[1]),
                0x60 => VIFExtension::ResetCounter,
                0x61 => VIFExtension::CumulationCounter,
                0x62 => VIFExtension::ControlSignal,
                0x63 => VIFExtension::DayOfWeek,
                0x64 => VIFExtension::WeekNumber,
                0x65 => VIFExtension::TimePointOfDay,
                0x66 => VIFExtension::StateOfParameterActivation,
                0x67 => VIFExtension::SpecialSupervision,
                0x68..=0x6B => VIFExtension::DurationSinceLastCumulation(0b11 & data[1]),
                0x6C..=0x6F => VIFExtension::OperatingTimeBattery(0b11 & data[1]),
                0x70 => VIFExtension::DateAndTimeOfBatteryChange,
                _ => VIFExtension::Reserved,
            }),
            0xFB => ValueInformation::Extended(match data[1] {
                0x00 | 0x01 => VIFExtension::EnergyMWh(0b1 & data[1]),
                0x08 | 0x09 => VIFExtension::EnergyGJ(0b1 & data[1]),
                0x10 | 0x11 => VIFExtension::VolumeM3(0b1 & data[1]),
                0x18 | 0x19 => VIFExtension::MassTons(0b1 & data[1]),
                0x21 => VIFExtension::VolumeFeet3Tenth,
                0x22 => VIFExtension::VolumeAmericanGallon,
                0x23 => VIFExtension::VolumeFlowAmericanGallonPerMinuteThousandth,
                0x24 => VIFExtension::VolumeFlowAmericanGallonPerMinute,
                0x25 => VIFExtension::VolumeFlowAmericanGallonPerHour,
                0x28 | 0x29 => VIFExtension::PowerMW(0b1 & data[1]),
                0x30 | 0x31 => VIFExtension::PowerGJH(0b1 & data[1]),
                0x50..=0x53 => VIFExtension::FlowTemperature(0b11 & data[1]),
                0x54..=0x57 => VIFExtension::ReturnTemperature(0b11 & data[1]),
                0x60..=0x63 => VIFExtension::TemperatureDifference(0b11 & data[1]),
                0x64..=0x67 => VIFExtension::ExternalTemperature(0b11 & data[1]),
                0x70..=0x73 => VIFExtension::ColdWarmTemperatureLimitFarenheit(0b11 & data[1]),
                0x74..=0x77 => VIFExtension::ColdWarmTemperatureLimitCelsius(0b11 & data[1]),
                0x78..=0x7F => VIFExtension::CumulativeCountMaxPower(0b111 & data[1]),
                _ => VIFExtension::Reserved,
            }),
            0x7D | 0xFE => ValueInformation::Any,
            0x7E | 0xFF => ValueInformation::ManufacturerSpecific,
            _ => unreachable!(),
        })
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    HourMinuteSecond,
    DayMonthYear,
    WattHour,
    KiloWattHour,
    MegaWattHour,
    Joul,
    Kilogram,
    KiloJoul,
    MegaJoul,
    GigaJoul,
    Watt,
    KiloWatt,
    MegaWat,
    KiloJoulHour,
    MegaJoulHour,
    GigaJoulHour,
    MegaLiter,
    Liter,
    CubicMeter,
    MegaLiterHour,
    LiterHour,
    CubicMeterPerHour,
    CubicMeterPerMinute,
    CubicMeterPerSecond,
    KilogramPerHour,
    Celsius,
    Kelvin,
    Bar,
    HCA,
    Reserved,
    WithoutUnits,
    Seconds,
    Minutes,
    Hours,
    Days,
    JoulPerHour,
}

impl TryFrom<&ValueInformation> for Unit {
    type Error = ValueInformationError;

    fn try_from(value_information: &ValueInformation) -> Result<Self, ValueInformationError> {
        match value_information {
            ValueInformation::Primary(x) => match x & 0x7F {
                0x00..=0x07 => Ok(Unit::WattHour),
                0x08..=0x0F => Ok(Unit::Joul),
                0x10..=0x17 => Ok(Unit::CubicMeter),
                0x18..=0x1F => Ok(Unit::Kilogram),
                0x20 | 0x24 => Ok(Unit::Seconds),
                0x21 | 0x25 => Ok(Unit::Minutes),
                0x22 | 0x26 => Ok(Unit::Hours),
                0x23 | 0x27 => Ok(Unit::Days),
                0x28..=0x2F => Ok(Unit::Watt),
                0x30..=0x37 => Ok(Unit::JoulPerHour),
                0x38..=0x3F => Ok(Unit::CubicMeterPerHour),
                0x40..=0x47 => Ok(Unit::CubicMeterPerMinute),
                0x48..=0x4F => Ok(Unit::CubicMeterPerSecond),
                0x50..=0x57 => Ok(Unit::KilogramPerHour),
                0x58..=0x5F | 0x64..=0x67 => Ok(Unit::Celsius),
                0x60..=0x63 => Ok(Unit::Kelvin),
                0x68..=0x6B => Ok(Unit::Bar),

                _ => todo!("Implement the rest of the units: {:?}", x),
            },
            ValueInformation::PlainText => todo!(),
            ValueInformation::Extended(_) => todo!(),
            ValueInformation::Any => todo!(),
            ValueInformation::ManufacturerSpecific => todo!(),
        }
    }
}

const MAX_RECORDS: usize = 10;
#[derive(Debug)]
struct ValueInformationBlock {
    _value_information: ValueInformation,
    _value_information_extension: Option<ArrayVec<u8, MAX_RECORDS>>,
}
mod tests {

    #[test]
    fn test_value_information_new() {
        use crate::user_data::value_information::Unit;
        use crate::user_data::value_information::ValueInformation;

        /* VIF = 0x13 => m3^3*1e-3 */
        let data = [0x13];
        let result = ValueInformation::try_from(data.as_slice()).unwrap();
        assert_eq!(result, ValueInformation::Primary(0x13));
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(&result).unwrap(), Unit::CubicMeter);

        /* VIF = 0x14 => m3^-3*1e-2 */
        let data = [0x14];
        let result = ValueInformation::try_from(data.as_slice()).unwrap();
        assert_eq!(result, ValueInformation::Primary(0x14));
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(&result).unwrap(), Unit::CubicMeter);

        /* VIF = 0x15 => m3^3*1e-2 */

        let data = [0x15];
        let result = ValueInformation::try_from(data.as_slice()).unwrap();
        assert_eq!(result, ValueInformation::Primary(0x15));
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(&result).unwrap(), Unit::CubicMeter);

        /* VIF = 0x16 => m3^-3*1e-1 */
        let data = [0x16];
        let result = ValueInformation::try_from(data.as_slice()).unwrap();
        assert_eq!(result, ValueInformation::Primary(0x16));
        assert_eq!(result.get_size(), 1);
    }
}
