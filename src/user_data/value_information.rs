use arrayvec::ArrayVec;

const MAX_VIFE_RECORDS: usize = 10;

impl TryFrom<&[u8]> for ValueInformationBlock {
    type Error = ValueInformationError;

    fn try_from(data: &[u8]) -> Result<Self, ValueInformationError> {
        let mut vife = ArrayVec::<ValueInformationFieldExtension, MAX_VIFE_RECORDS>::new();
        let vif = ValueInformationField::from(data[0]);

        if vif.has_extension() {
            let mut offset = 1;
            while offset < data.len() {
                let vife_data = data[offset];
                let current_vife = ValueInformationFieldExtension {
                    data: vife_data,
                    coding: match (offset, vife_data) {
                        (0, 0xFB) => ValueInformationFieldExtensionCoding::MainVIFCodeExtension,
                        (0, 0xFC) => {
                            ValueInformationFieldExtensionCoding::AlternateVIFCodeExtension
                        }
                        (0, 0xEF) => {
                            ValueInformationFieldExtensionCoding::ReservedAlternateVIFCodeExtension
                        }
                        _ => ValueInformationFieldExtensionCoding::ComninableOrthogonalVIFECodeExtension,
                    },
                };
                vife.push(current_vife);
                offset += 1;
                if !vife.last().unwrap().has_extension() {
                    break;
                }
                if vife.len() > MAX_VIFE_RECORDS {
                    return Err(ValueInformationError::InvalidValueInformation);
                }
            }
        }
        Ok(ValueInformationBlock {
            value_information: vif,
            value_information_extension: if vife.is_empty() { None } else { Some(vife) },
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct ValueInformationBlock {
    pub value_information: ValueInformationField,
    pub value_information_extension:
        Option<ArrayVec<ValueInformationFieldExtension, MAX_VIFE_RECORDS>>,
}

#[derive(Debug, PartialEq)]
pub struct ValueInformationField {
    pub data: u8,
}

#[derive(Debug, PartialEq)]
pub struct ValueInformationFieldExtension {
    pub data: u8,
    pub coding: ValueInformationFieldExtensionCoding,
}

impl From<&ValueInformationField> for ValueInformationCoding {
    fn from(value_information: &ValueInformationField) -> Self {
        match value_information.data {
            0x00..=0x7B | 0x80..=0xFA => ValueInformationCoding::Primary,
            0x7C | 0xFC => ValueInformationCoding::PlainText,
            0xFD => ValueInformationCoding::LineaVIFExtension,
            0xFB => ValueInformationCoding::Any,
            0x7E => ValueInformationCoding::ManufacturerSpecific,
            0xFE => ValueInformationCoding::ManufacturerSpecific,
            0x7F => ValueInformationCoding::ManufacturerSpecific,
            0xFF => ValueInformationCoding::ManufacturerSpecific,
            _ => unreachable!("Invalid value information: {:X}", value_information.data),
        }
    }
}

impl ValueInformationField {
    fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

impl ValueInformationFieldExtension {
    fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationCoding {
    Primary,
    PlainText,
    LineaVIFExtension,
    Any,
    ManufacturerSpecific,
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationFieldExtensionCoding {
    MainVIFCodeExtension,
    AlternateVIFCodeExtension,
    ReservedAlternateVIFCodeExtension,
    ComninableOrthogonalVIFECodeExtension,
}

impl ValueInformationBlock {
    fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.value_information_extension {
            size += vife.len();
        }
        size
    }
}

impl TryFrom<ValueInformationBlock> for Unit {
    type Error = ValueInformationError;

    fn try_from(
        value_information_block: ValueInformationBlock,
    ) -> Result<Self, ValueInformationError> {
        match ValueInformationCoding::from(&value_information_block.value_information) {
            ValueInformationCoding::Primary => match value_information_block.value_information.data
            {
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
                0x6C..=0x6D => Ok(Unit::TimePoint),
                0x74..=0x77 => Ok(Unit::ActualityDuration),
                0x78 => Ok(Unit::FabricationNumber),
                x => todo!("Implement the rest of the units: {:?}", x),
            },
            ValueInformationCoding::PlainText => Ok(Unit::PlainText),
            _ => todo!(
                "Implement the rest of the units: {:?}",
                value_information_block
            ),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationError {
    InvalidValueInformation,
}

impl From<u8> for ValueInformationField {
    fn from(data: u8) -> Self {
        ValueInformationField { data }
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
    ActualityDuration,
    TimePoint,
    FabricationNumber,
    MegaWatt,
    PlainText,
}

mod tests {

    #[test]
    fn test_single_byte_primary_value_information_parsing() {
        use crate::user_data::value_information::Unit;
        use crate::user_data::value_information::{ValueInformationBlock, ValueInformationField};

        /* VIB = 0x13 => m3^3*1e-3 */
        let data = [0x13];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x13),
                value_information_extension: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(result).unwrap(), Unit::CubicMeter);

        /* VIB = 0x14 => m3^-3*1e-2 */
        let data = [0x14];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x14),
                value_information_extension: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(result).unwrap(), Unit::CubicMeter);

        /* VIB = 0x15 => m3^3*1e-2 */
        let data = [0x15];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x15),
                value_information_extension: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(Unit::try_from(result).unwrap(), Unit::CubicMeter);

        /* VIB = 0x16 => m3^-3*1e-1 */
        let data = [0x16];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x16),
                value_information_extension: None
            }
        );
        assert_eq!(result.get_size(), 1);
    }

    #[test]
    fn test_multibyte_primary_value_information() {
        use crate::user_data::value_information::ValueInformationBlock;
        /* 1 VIF, 1 - 10 orthogonal VIFE */

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x12 => Combinable Orthogonal VIFE meaning "averaged" */
        /* VIB = 0x96, 0x12 */
        let data = [0x96, 0x12];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x92 = 0x12 | 0x80  => Combinable Orthogonal VIFE meaning "averaged" with extension */
        /* VIFE 0x20 => Combinable Orthogonal VIFE meaning "per second" */
        /* VIB = 0x96, 0x92,0x20 */

        let data = [0x96, 0x92, 0x20];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 3);

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x92 = 0x12 | 0x80  => Combinable Orthogonal VIFE meaning "averaged" with extension */
        /* VIFE 0xA0= 0x20 | 0x80 => Combinable Orthogonal VIFE meaning "per second" */
        /* VIFE 0x2D => Combinable Orthogonal VIFE meaning "per m3". This cancels out the ViF m3, which is useless
        but till a valid VIB */
        /* VIB = 0x96, 0x92,0xA0, 0x2D */
        let data = [0x96, 0x92, 0xA0, 0x2D];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 4);
    }

    #[test]
    fn test_plain_text_vif_norm_conform() {
        use crate::user_data::value_information::{Unit, ValueInformationBlock};
        // This is the ascii conform method of encoding the VIF
        // VIF  VIFE  LEN(3) 'R'   'H'  '%'
        // 0xFC, 0x74, 0x03, 0x48, 0x52, 0x25,
        // %RH
        // Combinable (orthogonal) VIFE-Code extension table
        // VIFE = 0x74 => E111 0nnn Multiplicative correction factor for value (not unit): 10nnn–6 => 10^-2
        //
        // according to the Norm the LEN and ASCII is not part tof the VIB
        let data = [0xFC, 0x74];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);
        assert_eq!(result.value_information.data, 0xFC);
        assert_eq!(Unit::try_from(result).unwrap(), Unit::PlainText);

        // This is how the VIF is encoded in the test vectors
        // VIF  LEN(3) 'R'   'H'  '%'    VIFE
        // 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74,
        // %RH
        // VIFE = 0x74 => E111 0nnn Multiplicative correction factor for value (not unit): 10nnn–6 => 10^-2
        // when not following the norm the LEN and ASCII is part of the VIB
        // It is however none norm conform, see the next example which follows
        // the MBUS Norm which explicitly states that the VIIFE should be after the VIF
        // not aftter the ASCII plain text and its size
    }
}
