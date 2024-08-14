#[cfg(feature = "std")]
use std::fmt;

use super::data_information::DataInformationError;
use arrayvec::ArrayVec;

const MAX_VIFE_RECORDS: usize = 10;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Unit {
    pub name: UnitName,
    pub exponent: i32,
}
macro_rules! unit {
    ($name:ident) => {
        Unit {
            name: UnitName::$name,
            exponent: 1,
        }
    };
    ($name:ident ^ $exponent:literal) => {
        Unit {
            name: UnitName::$name,
            exponent: $exponent,
        }
    };
}

impl TryFrom<&[u8]> for ValueInformationBlock {
    type Error = DataInformationError;

    fn try_from(data: &[u8]) -> Result<Self, DataInformationError> {
        let mut vife = ArrayVec::<ValueInformationFieldExtension, MAX_VIFE_RECORDS>::new();
        let vif =
            ValueInformationField::from(*data.first().ok_or(DataInformationError::DataTooShort)?);
        let mut plaintext_vife: Option<ArrayVec<char, 9>> = None;

        #[cfg(not(feature = "plaintext-before-extension"))]
        let standard_plaintex_vib = true;
        #[cfg(feature = "plaintext-before-extension")]
        let standard_plaintex_vib = false;

        if !standard_plaintex_vib && vif.value_information_contains_ascii() {
            plaintext_vife = Some(extract_plaintext_vife(
                data.get(1..).ok_or(DataInformationError::DataTooShort)?,
            )?);
        }

        if vif.has_extension() {
            let mut offset = 1;
            while offset < data.len() {
                let vife_data = *data.get(offset).ok_or(DataInformationError::DataTooShort)?;
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
                let has_extension = current_vife.has_extension();
                vife.push(current_vife);
                offset += 1;
                if !has_extension {
                    break;
                }
                if vife.len() > MAX_VIFE_RECORDS {
                    return Err(DataInformationError::InvalidValueInformation);
                }
            }
            if standard_plaintex_vib && vif.value_information_contains_ascii() {
                plaintext_vife = Some(extract_plaintext_vife(
                    data.get(offset..)
                        .ok_or(DataInformationError::DataTooShort)?,
                )?);
            }
        }

        Ok(Self {
            value_information: vif,
            value_information_extension: if vife.is_empty() { None } else { Some(vife) },
            plaintext_vife,
        })
    }
}

fn extract_plaintext_vife(data: &[u8]) -> Result<ArrayVec<char, 9>, DataInformationError> {
    let ascii_length = *data.first().ok_or(DataInformationError::DataTooShort)? as usize;
    let mut ascii = ArrayVec::<char, 9>::new();
    for item in data
        .get(1..=ascii_length)
        .ok_or(DataInformationError::DataTooShort)?
    {
        ascii.push(*item as char);
    }
    Ok(ascii)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ValueInformationBlock {
    pub value_information: ValueInformationField,
    pub value_information_extension:
        Option<ArrayVec<ValueInformationFieldExtension, MAX_VIFE_RECORDS>>,
    pub plaintext_vife: Option<ArrayVec<char, 9>>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ValueInformationField {
    pub data: u8,
}

impl ValueInformationField {
    const fn value_information_contains_ascii(&self) -> bool {
        self.data == 0x7C || self.data == 0xFC
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ValueInformationFieldExtension {
    pub data: u8,
    pub coding: ValueInformationFieldExtensionCoding,
}

impl From<&ValueInformationField> for ValueInformationCoding {
    fn from(value_information: &ValueInformationField) -> Self {
        match value_information.data {
            0x00..=0x7B | 0x80..=0xFA => Self::Primary,
            0x7C | 0xFC => Self::PlainText,
            0xFD => Self::MainVIFExtension,
            0xFB => Self::AlternateVIFExtension,
            0x7E => Self::ManufacturerSpecific,
            0xFE => Self::ManufacturerSpecific,
            0x7F => Self::ManufacturerSpecific,
            0xFF => Self::ManufacturerSpecific,
            _ => unreachable!("Invalid value information: {:X}", value_information.data),
        }
    }
}

impl ValueInformationField {
    const fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

impl ValueInformationFieldExtension {
    const fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationCoding {
    Primary,
    PlainText,
    MainVIFExtension,
    AlternateVIFExtension,
    ManufacturerSpecific,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum ValueInformationFieldExtensionCoding {
    MainVIFCodeExtension,
    AlternateVIFCodeExtension,
    ReservedAlternateVIFCodeExtension,
    ComninableOrthogonalVIFECodeExtension,
}

impl ValueInformationBlock {
    #[must_use]
    pub const fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.value_information_extension {
            size += vife.len();
        }
        if let Some(plaintext_vife) = &self.plaintext_vife {
            // 1 byte for the length of the ASCII string
            size += plaintext_vife.len() + 1;
        }
        size
    }
}

impl TryFrom<&ValueInformationBlock> for ValueInformation {
    type Error = DataInformationError;

    fn try_from(
        value_information_block: &ValueInformationBlock,
    ) -> Result<Self, DataInformationError> {
        let mut units = ArrayVec::<Unit, 10>::new();
        let mut labels = ArrayVec::<ValueLabel, 10>::new();
        let mut decimal_scale_exponent: isize = 0;
        let mut decimal_offset_exponent = 0;
        match ValueInformationCoding::from(&value_information_block.value_information) {
            ValueInformationCoding::Primary => {
                match value_information_block.value_information.data & 0x7F {
                    0x00..=0x07 => {
                        units.push(unit!(Watt));
                        units.push(unit!(Hour));
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x08..=0x0F => {
                        units.push(unit!(Joul));
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize;
                    }
                    0x10..=0x17 => {
                        units.push(unit!(Meter ^ 3));
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 6;
                    }
                    0x18..=0x1F => {
                        units.push(unit!(Kilogram));
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x20 | 0x24 => {
                        units.push(unit!(Second));
                    }
                    0x21 | 0x25 => units.push(unit!(Meter)),
                    0x22 | 0x26 => units.push(unit!(Hour)),
                    0x23 | 0x27 => units.push(unit!(Day)),
                    0x28..=0x2F => {
                        units.push(unit!(Watt));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x30..=0x37 => {
                        units.push(unit!(Joul));
                        units.push(unit!(Hour ^ -1));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize;
                    }
                    0x38..=0x3F => {
                        units.push(unit!(Meter ^ 3));
                        units.push(unit!(Hour ^ -1));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 6;
                    }
                    0x40..=0x47 => {
                        units.push(unit!(Meter ^ 3));
                        units.push(unit!(Minute ^ -1));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 7;
                    }
                    0x48..=0x4F => {
                        units.push(unit!(Meter ^ 3));
                        units.push(unit!(Second ^ -1));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 9;
                    }
                    0x50..=0x57 => {
                        units.push(unit!(Kilogram ^ 3));
                        units.push(unit!(Hour ^ -1));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x58..=0x5F | 0x64..=0x67 => {
                        units.push(unit!(Celsius));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x60..=0x63 => {
                        units.push(unit!(Kelvin));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x68..=0x6B => {
                        units.push(unit!(Bar));
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x6C => labels.push(ValueLabel::Date),
                    0x6D => labels.push(ValueLabel::DateTime),
                    0x6E => labels.push(ValueLabel::DimensionlessHCA),
                    0x70..=0x73 => labels.push(ValueLabel::AveragingDuration),
                    0x74..=0x77 => labels.push(ValueLabel::ActualityDuration),
                    0x78 => labels.push(ValueLabel::FabricationNumber),
                    0x79 => labels.push(ValueLabel::EnhancedIdentification),
                    0x7A => labels.push(ValueLabel::Address),
                    0x7B => {}

                    x => todo!("Implement the rest of the units: {:X?}", x),
                };
                /* consume orthogonal vife */
                consume_orthhogonal_vife(
                    value_information_block,
                    &mut labels,
                    &mut units,
                    &mut decimal_scale_exponent,
                    &mut decimal_offset_exponent,
                );
            }
            ValueInformationCoding::MainVIFExtension => {
                let vife = value_information_block
                    .value_information_extension
                    .as_ref()
                    .ok_or(Self::Error::InvalidValueInformation)?;

                let first_vife_data = vife.first().ok_or(DataInformationError::DataTooShort)?.data;
                let second_vife_data = vife.get(1).ok_or(DataInformationError::DataTooShort)?.data;
                match first_vife_data & 0x7F {
                    0x00..=0x03 => {
                        units.push(unit!(LocalMoneyCurrency));
                        labels.push(ValueLabel::Credit);
                        decimal_scale_exponent = (first_vife_data & 0b11) as isize - 3;
                    }
                    0x04..=0x07 => {
                        units.push(unit!(LocalMoneyCurrency));
                        labels.push(ValueLabel::Debit);
                        decimal_scale_exponent = (first_vife_data & 0b11) as isize - 3;
                    }
                    0x08 => labels.push(ValueLabel::UniqueMessageIdentificationOrAccessNumber),
                    0x09 => labels.push(ValueLabel::DeviceType),
                    0x0A => labels.push(ValueLabel::Manufacturer),
                    0x0B => labels.push(ValueLabel::ParameterSetIdentification),
                    0x0C => labels.push(ValueLabel::ModelOrVersion),
                    0x0D => labels.push(ValueLabel::HardwareVersion),
                    0x0E => labels.push(ValueLabel::MetrologyFirmwareVersion),
                    0x0F => labels.push(ValueLabel::OtherSoftwareVersion),
                    0x10 => labels.push(ValueLabel::CustomerLocation),
                    0x11 => labels.push(ValueLabel::Customer),
                    0x12 => labels.push(ValueLabel::AccessCodeUser),
                    0x13 => labels.push(ValueLabel::AccessCodeOperator),
                    0x14 => labels.push(ValueLabel::AccessCodeSystemOperator),
                    0x15 => labels.push(ValueLabel::AccessCodeDeveloper),
                    0x16 => labels.push(ValueLabel::Password),
                    0x17 => labels.push(ValueLabel::ErrorFlags),
                    0x18 => labels.push(ValueLabel::ErrorMask),
                    0x19 => labels.push(ValueLabel::SecurityKey),
                    0x1A => {
                        labels.push(ValueLabel::DigitalOutput);
                        labels.push(ValueLabel::Binary);
                    }
                    0x1B => {
                        labels.push(ValueLabel::DigitalInput);
                        labels.push(ValueLabel::Binary);
                    }
                    0x1C => {
                        units.push(unit!(Symbol));
                        units.push(unit!(Second ^ -1));
                        labels.push(ValueLabel::BaudRate);
                    }
                    0x1D => {
                        units.push(unit!(BitTime));
                        labels.push(ValueLabel::ResponseDelayTime);
                    }
                    0x1E => labels.push(ValueLabel::Retry),
                    0x1F => labels.push(ValueLabel::RemoteControl),
                    0x20 => labels.push(ValueLabel::FirstStorageForCycleStorage),
                    0x21 => labels.push(ValueLabel::LastStorageForCycleStorage),
                    0x22 => labels.push(ValueLabel::SizeOfStorageBlock),
                    0x23 => labels.push(ValueLabel::DescripitonOfTariffAndSubunit),
                    0x24 => {
                        units.push(unit!(Second));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x25 => {
                        units.push(unit!(Minute));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x26 => {
                        units.push(unit!(Hour));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x27 => {
                        units.push(unit!(Day));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x28 => {
                        units.push(unit!(Month));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x29 => {
                        units.push(unit!(Year));
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x30 => labels.push(ValueLabel::DimensionlessHCA),
                    0x31 => labels.push(ValueLabel::DataContainerForWmbusProtocol),
                    0x32 => {
                        units.push(unit!(Second));
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x33 => {
                        units.push(unit!(Meter));
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x34 => {
                        units.push(unit!(Hour));
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x35 => {
                        units.push(unit!(Day));
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x50..=0x5F => {
                        units.push(unit!(Volt));
                        decimal_scale_exponent = (first_vife_data & 0b1111) as isize - 9;
                    }
                    0x60 => labels.push(ValueLabel::ResetCounter),
                    0x61 => labels.push(ValueLabel::CumulationCounter),
                    0x62 => labels.push(ValueLabel::ControlSignal),
                    0x63 => labels.push(ValueLabel::DayOfWeek),
                    0x64 => labels.push(ValueLabel::WeekNumber),
                    0x65 => labels.push(ValueLabel::TimePointOfChangeOfTariff),
                    0x66 => labels.push(ValueLabel::StateOfParameterActivation),
                    0x67 => labels.push(ValueLabel::SpecialSupplierInformation),
                    0x68 => {
                        units.push(unit!(Hour));
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x69 => {
                        units.push(unit!(Day));
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6A => {
                        units.push(unit!(Month));
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6B => {
                        units.push(unit!(Year));
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6C => {
                        units.push(unit!(Hour));
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6D => {
                        units.push(unit!(Day));
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6E => {
                        units.push(unit!(Month));
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6F => {
                        units.push(unit!(Hour));
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x70 => {
                        units.push(unit!(Second));
                        labels.push(ValueLabel::DateAndTimeOfBatteryChange);
                    }
                    0x71 => {
                        units.push(unit!(DecibelMilliWatt));
                        labels.push(ValueLabel::RFPowerLevel);
                    }
                    0x72 => labels.push(ValueLabel::DaylightSavingBeginningEndingDeviation),
                    0x73 => labels.push(ValueLabel::ListeningWindowManagementData),
                    0x74 => labels.push(ValueLabel::RemainingBatteryLifeTime),
                    0x75 => labels.push(ValueLabel::NumberOfTimesTheMeterWasStopped),
                    0x76 => labels.push(ValueLabel::DataContainerForManufacturerSpecificProtocol),
                    0x7D => match second_vife_data & 0x7F {
                        0x00 => labels.push(ValueLabel::CurrentlySelectedApplication),
                        0x02 => {
                            units.push(unit!(Month));
                            labels.push(ValueLabel::RemainingBatteryLifeTime);
                        }
                        0x03 => {
                            units.push(unit!(Year));
                            labels.push(ValueLabel::RemainingBatteryLifeTime);
                        }
                        _ => labels.push(ValueLabel::Reserved),
                    },
                    _ => labels.push(ValueLabel::Reserved),
                }
            }
            ValueInformationCoding::AlternateVIFExtension => {
                use UnitName::*;
                use ValueLabel::*;
                let mk_unit = |name, exponent| Unit { name, exponent };
                macro_rules! populate {
                    (@trd) => {};
                    (@trd , $label:expr) => {{ labels.push($label); }};
                    (@snd dec: $decimal:literal $($rem:tt)*) => {{
                        decimal_scale_exponent = $decimal;
                        populate!(@trd $($rem)*);
                    }};
                    ($name:ident / h, $exponent:expr, $($rem:tt)*) => {{
                        units.push(mk_unit($name, $exponent));
                        units.push(mk_unit(Hour, -1));
                        populate!(@snd $($rem)*)
                    }};
                    ($name:ident * h, $exponent:expr, $($rem:tt)*) => {{
                        units.push(mk_unit($name, $exponent));
                        units.push(mk_unit(Hour, 1));
                        populate!(@snd $($rem)*)
                    }};
                    ($name:ident, $exponent:expr, $($rem:tt)*) => {{
                        units.push(mk_unit($name, $exponent));
                        populate!(@snd $($rem)*)
                    }};
                }
                let vife = value_information_block
                    .value_information_extension
                    .as_ref()
                    .ok_or(Self::Error::InvalidValueInformation)?;
                let first_vife_data = vife.first().ok_or(DataInformationError::DataTooShort)?.data;
                match first_vife_data & 0x7F {
                    0b0 => populate!(Watt / h, 3, dec: 5, Energy),
                    0b000_0001 => populate!(Watt / h, 3, dec: 6, Energy),
                    0b000_0010 => populate!(ReactiveWatt * h, 1, dec: 3, Energy),
                    0b000_0011 => populate!(ReactiveWatt * h, 1, dec: 4, Energy),
                    0b000_1000 => populate!(Joul, 1, dec: 8, Energy),
                    0b000_1001 => populate!(Joul, 1, dec: 9, Energy),
                    0b000_1100 => populate!(Joul, 1, dec: 5, Energy),
                    0b000_1101 => populate!(Joul, 1, dec: 6, Energy),
                    0b000_1110 => populate!(Joul, 1, dec: 7, Energy),
                    0b000_1111 => populate!(Joul, 1, dec: 8, Energy),
                    0b001_0000 => populate!(Meter, 3, dec: 2),
                    0b001_0001 => populate!(Meter, 3, dec: 3),
                    0b001_0100 => populate!(ReactiveWatt, 1, dec: -3),
                    0b001_0101 => populate!(ReactiveWatt, 1, dec: -2),
                    0b001_0110 => populate!(ReactiveWatt, 1, dec: -1),
                    0b001_0111 => populate!(ReactiveWatt, 1, dec: 0),
                    0b001_1000 => populate!(Tonne, 1, dec: 2),
                    0b001_1001 => populate!(Tonne, 1, dec: 3),
                    0b001_1010 => populate!(Percent, 1, dec: -1, RelativeHumidity),
                    0b010_0000 => populate!(Feet, 3, dec: 0),
                    0b010_0001 => populate!(Feet, 3, dec: 1),
                    0b010_1000 => populate!(Watt, 1, dec: 5),
                    0b010_1001 => populate!(Watt, 1, dec: 6),
                    0b010_1010 => populate!(Degree, 1, dec: -1, PhaseUtoU),
                    0b010_1011 => populate!(Degree, 1, dec: -1, PhaseUtoI),
                    0b010_1100 => populate!(Hertz, 1, dec: -3),
                    0b010_1101 => populate!(Hertz, 1, dec: -2),
                    0b010_1110 => populate!(Hertz, 1, dec: -1),
                    0b010_1111 => populate!(Hertz, 1, dec: 0),
                    0b011_0000 => populate!(Joul / h, 1, dec: -8),
                    0b011_0001 => populate!(Joul / h, 1, dec: -7),
                    0b011_0100 => populate!(ApparentWatt / h, 1, dec: 0),
                    0b011_0101 => populate!(ApparentWatt / h, 1, dec: 1),
                    0b011_0110 => populate!(ApparentWatt / h, 1, dec: 2),
                    0b011_0111 => populate!(ApparentWatt / h, 1, dec: 3),
                    0b111_0100 => populate!(Celsius, 1, dec: -3, ColdWarmTemperatureLimit),
                    0b111_0101 => populate!(Celsius, 1, dec: -2, ColdWarmTemperatureLimit),
                    0b111_0110 => populate!(Celsius, 1, dec: -1, ColdWarmTemperatureLimit),
                    0b111_0111 => populate!(Celsius, 1, dec: 0, ColdWarmTemperatureLimit),
                    0b111_1000 => populate!(Watt, 1, dec: -3, CumaltiveMaximumOfActivePower),
                    0b111_1001 => populate!(Watt, 1, dec: -2, CumaltiveMaximumOfActivePower),
                    0b111_1010 => populate!(Watt, 1, dec: -1, CumaltiveMaximumOfActivePower),
                    0b111_1011 => populate!(Watt, 1, dec: 0, CumaltiveMaximumOfActivePower),
                    0b111_1100 => populate!(Watt, 1, dec: 1, CumaltiveMaximumOfActivePower),
                    0b111_1101 => populate!(Watt, 1, dec: 2, CumaltiveMaximumOfActivePower),
                    0b111_1110 => populate!(Watt, 1, dec: 3, CumaltiveMaximumOfActivePower),
                    0b111_1111 => populate!(Watt, 1, dec: 4, CumaltiveMaximumOfActivePower),
                    0b110_1000 => populate!(HCAUnit, 1,dec: 0, ResultingRatingFactor),
                    0b110_1001 => populate!(HCAUnit, 1,dec: 0, ThermalOutputRatingFactor),
                    0b110_1010 => populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingFactorOverall),
                    0b110_1011 => populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingRoomSide),
                    0b110_1100 => {
                        populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingFactorHeatingSide)
                    }
                    0b110_1101 => populate!(HCAUnit, 1,dec: 0, LowTemperatureRatingFactor),
                    0b110_1110 => populate!(HCAUnit, 1,dec: 0, DisplayOutputScalingFactor),

                    _ => todo!("Implement the rest of the units: {:X?}", first_vife_data),
                };
            }
            // we need to check if the next byte is equivalent to the length of the rest of the
            // the data. In this case it is very likely that, this is how the payload is built up.
            ValueInformationCoding::PlainText => labels.push(ValueLabel::PlainText),
            ValueInformationCoding::ManufacturerSpecific => {
                labels.push(ValueLabel::ManufacturerSpecific)
            }
        }

        Ok(Self {
            decimal_offset_exponent,
            labels,
            decimal_scale_exponent,
            units,
        })
    }
}

fn consume_orthhogonal_vife(
    value_information_block: &ValueInformationBlock,
    labels: &mut ArrayVec<ValueLabel, 10>,
    units: &mut ArrayVec<Unit, 10>,
    decimal_scale_exponent: &mut isize,
    decimal_offset_exponent: &mut isize,
) {
    if let Some(vife) = &value_information_block.value_information_extension {
        let mut is_extension_of_combinable_orthogonal_vife = false;
        for v in vife {
            if v.data == 0xFC {
                is_extension_of_combinable_orthogonal_vife = true;
                continue;
            }
            if is_extension_of_combinable_orthogonal_vife {
                is_extension_of_combinable_orthogonal_vife = false;
                match v.data & 0x7F {
                    0x00 => labels.push(ValueLabel::Reserved),
                    0x01 => labels.push(ValueLabel::AtPhaseL1),
                    0x02 => labels.push(ValueLabel::AtPhaseL2),
                    0x03 => labels.push(ValueLabel::AtPhaseL3),
                    0x04 => labels.push(ValueLabel::AtNeutral),
                    0x05 => labels.push(ValueLabel::BetweenPhasesL1L2),
                    0x06 => labels.push(ValueLabel::BetweenPhasesL2L3),
                    0x07 => labels.push(ValueLabel::BetweenPhasesL3L1),
                    0x08 => labels.push(ValueLabel::AtQuadrant1),
                    0x09 => labels.push(ValueLabel::AtQuadrant2),
                    0x0A => labels.push(ValueLabel::AtQuadrant3),
                    0x0B => labels.push(ValueLabel::AtQuadrant4),
                    0x0C => labels.push(ValueLabel::DeltaBetweenImportAndExport),
                    0x0F => labels.push(
                        ValueLabel::AccumulationOfAbsoluteValueBothPositiveAndNegativeContribution,
                    ),
                    0x11 => labels.push(ValueLabel::DataPresentedWithTypeC),
                    0x12 => labels.push(ValueLabel::DataPresentedWithTypeD),
                    0x13 => labels.push(ValueLabel::DirectionFromCommunicationPartnerToMeter),
                    0x14 => labels.push(ValueLabel::DirectionFromMeterToCommunicationPartner),
                    _ => labels.push(ValueLabel::Reserved),
                }
            } else {
                match v.data & 0x7F {
                    0x00..=0x0F => labels.push(ValueLabel::ReservedForObjectActions),
                    0x10..=0x11 => labels.push(ValueLabel::Reserved),
                    0x12 => labels.push(ValueLabel::Averaged),
                    0x13 => labels.push(ValueLabel::InverseCompactProfile),
                    0x14 => labels.push(ValueLabel::RelativeDeviation),
                    0x15..=0x1C => labels.push(ValueLabel::RecoordErrorCodes),
                    0x1D => labels.push(ValueLabel::StandardConformDataContent),
                    0x1E => labels.push(ValueLabel::CompactProfileWithRegisterNumbers),
                    0x1F => labels.push(ValueLabel::CompactProfile),
                    0x20 => units.push(unit!(Second ^ -1)),
                    0x21 => units.push(unit!(Minute ^ -1)),
                    0x22 => units.push(unit!(Hour ^ -1)),
                    0x23 => units.push(unit!(Day ^ -1)),
                    0x24 => units.push(unit!(Week ^ -1)),
                    0x25 => units.push(unit!(Month ^ -1)),
                    0x26 => units.push(unit!(Year ^ -1)),
                    0x27 => units.push(unit!(Revolution ^ -1)),
                    0x28 => {
                        units.push(unit!(Increment));
                        units.push(unit!(InputPulseOnChannel0 ^ -1));
                    }
                    0x29 => {
                        units.push(unit!(Increment));
                        units.push(unit!(OutputPulseOnChannel0 ^ -1));
                    }
                    0x2A => {
                        units.push(unit!(Increment));
                        units.push(unit!(InputPulseOnChannel1 ^ -1));
                    }
                    0x2B => {
                        units.push(unit!(Increment));
                        units.push(unit!(OutputPulseOnChannel1 ^ -1));
                    }
                    0x2C => units.push(unit!(Liter)),
                    0x2D => units.push(unit!(Meter ^ -3)),
                    0x2E => units.push(unit!(Kilogram ^ -1)),
                    0x2F => units.push(unit!(Kelvin ^ -1)),
                    0x30 => {
                        units.push(unit!(Watt ^ -1));
                        units.push(unit!(Hour ^ -1));
                        *decimal_scale_exponent -= 3;
                    }
                    0x31 => {
                        units.push(unit!(Joul ^ -1));
                        *decimal_scale_exponent += -9;
                    }
                    0x32 => {
                        units.push(unit!(Watt ^ -1));
                        *decimal_scale_exponent += -3;
                    }
                    0x33 => {
                        units.push(unit!(Kelvin ^ -1));
                        units.push(unit!(Liter ^ -1));
                    }
                    0x34 => units.push(unit!(Volt ^ -1)),
                    0x35 => units.push(unit!(Ampere ^ -1)),
                    0x36 => units.push(unit!(Second ^ 1)),
                    0x37 => {
                        units.push(unit!(Second ^ 1));
                        units.push(unit!(Volt ^ -1));
                    }
                    0x38 => {
                        units.push(unit!(Second ^ 1));
                        units.push(unit!(Ampere ^ -1));
                    }
                    0x39 => labels.push(ValueLabel::StartDateOf),
                    0x3A => labels.push(ValueLabel::VifContinsUncorrectedUnitOrValue),
                    0x3B => labels.push(ValueLabel::AccumulationOnlyIfValueIsPositive),
                    0x3C => labels.push(ValueLabel::AccumulationOnlyIfValueIsNegative),
                    0x3D => labels.push(ValueLabel::NoneMetricUnits),
                    0x3E => labels.push(ValueLabel::ValueAtBaseConditions),
                    0x3F => labels.push(ValueLabel::ObisDecleration),
                    0x40 => labels.push(ValueLabel::UpperLimitValue),
                    0x48 => labels.push(ValueLabel::LowerLimitValue),
                    0x41 => labels.push(ValueLabel::NumberOfExceedsOfUpperLimitValue),
                    0x49 => labels.push(ValueLabel::NumberOfExceedsOfLowerLimitValue),
                    0x42 => labels.push(ValueLabel::DateOfBeginFirstLowerLimitExceed),
                    0x43 => labels.push(ValueLabel::DateOfBeginFirstUpperLimitExceed),
                    0x46 => labels.push(ValueLabel::DateOfBeginLastLowerLimitExceed),
                    0x47 => labels.push(ValueLabel::DateOfBeginLastUpperLimitExceed),
                    0x4A => labels.push(ValueLabel::DateOfEndLastLowerLimitExceed),
                    0x4B => labels.push(ValueLabel::DateOfEndLastUpperLimitExceed),
                    0x4E => labels.push(ValueLabel::DateOfEndFirstLowerLimitExceed),
                    0x4F => labels.push(ValueLabel::DateOfEndFirstUpperLimitExceed),
                    0x50 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(unit!(Second));
                    }
                    0x51 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(unit!(Minute));
                    }
                    0x52 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(unit!(Hour));
                    }
                    0x53 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(unit!(Day));
                    }
                    0x54 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(unit!(Second));
                    }
                    0x55 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(unit!(Minute));
                    }
                    0x56 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(unit!(Hour));
                    }
                    0x57 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(unit!(Day));
                    }
                    0x58 => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(unit!(Second));
                    }
                    0x59 => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(unit!(Minute));
                    }
                    0x5A => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(unit!(Hour));
                    }
                    0x5B => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(unit!(Day));
                    }
                    0x5C => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(unit!(Second));
                    }
                    0x5D => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(unit!(Minute));
                    }
                    0x5E => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(unit!(Hour));
                    }
                    0x5F => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(unit!(Day));
                    }
                    0x60 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(unit!(Second));
                    }
                    0x61 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(unit!(Minute));
                    }
                    0x62 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(unit!(Hour));
                    }
                    0x63 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(unit!(Day));
                    }
                    0x64 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(unit!(Second));
                    }
                    0x65 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(unit!(Minute));
                    }
                    0x66 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(unit!(Day));
                    }
                    0x68 => labels.push(ValueLabel::ValueDuringLowerValueExeed),
                    0x6C => labels.push(ValueLabel::ValueDuringUpperValueExceed),
                    0x69 => labels.push(ValueLabel::LeakageValues),
                    0x6D => labels.push(ValueLabel::OverflowValues),
                    0x6A => labels.push(ValueLabel::DateOfBeginFirst),
                    0x6B => labels.push(ValueLabel::DateOfBeginLast),
                    0x6E => labels.push(ValueLabel::DateOfEndLast),
                    0x6F => labels.push(ValueLabel::DateOfEndFirst),
                    0x70..=0x77 => {
                        *decimal_scale_exponent += (v.data & 0b111) as isize - 6;
                    }
                    0x78..=0x7B => {
                        *decimal_offset_exponent += (v.data & 0b11) as isize - 3;
                    }
                    0x7D => labels.push(ValueLabel::MultiplicativeCorrectionFactor103),
                    0x7E => labels.push(ValueLabel::FutureValue),
                    0x7F => {
                        labels.push(ValueLabel::NextVIFEAndDataOfThisBlockAreManufacturerSpecific)
                    }
                    _ => labels.push(ValueLabel::Reserved),
                };
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ValueInformationError {
    InvalidValueInformation,
}

impl From<u8> for ValueInformationField {
    fn from(data: u8) -> Self {
        Self { data }
    }
}
/// This is the most important type of the this file and represents
/// the whole information inside the value information block
/// value(x) = (multiplier * value + offset) * units
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ValueInformation {
    pub decimal_offset_exponent: isize,
    pub labels: ArrayVec<ValueLabel, 10>,
    pub decimal_scale_exponent: isize,
    pub units: ArrayVec<Unit, 10>,
}

#[cfg(feature = "std")]
impl fmt::Display for ValueInformation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.decimal_offset_exponent != 0 {
            write!(f, "+{})", self.decimal_offset_exponent)?;
        } else {
            write!(f, ")")?;
        }
        if self.decimal_scale_exponent != 0 {
            write!(f, "e{}", self.decimal_scale_exponent)?;
        }
        if !self.units.is_empty() {
            write!(f, "[")?;
            for unit in &self.units {
                write!(f, "{}", unit)?;
            }
            write!(f, "]")?;
        }
        if !self.labels.is_empty() {
            write!(f, "(")?;
            for (i, label) in self.labels.iter().enumerate() {
                write!(f, "{:?}", label)?;
                if i != self.labels.len() - 1 {
                    write!(f, ", ")?;
                }
            }

            return write!(f, ")");
        }
        Ok(())
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum ValueLabel {
    Instantaneous,
    ReservedForObjectActions,
    Reserved,
    Averaged,
    Integral,
    Parameter,
    InverseCompactProfile,
    RelativeDeviation,
    RecoordErrorCodes,
    StandardConformDataContent,
    CompactProfileWithRegisterNumbers,
    CompactProfile,
    ActualityDuration,
    AveragingDuration,
    Date,
    Time,
    DateTime,
    DateTimeWithSeconds,
    FabricationNumber,
    EnhancedIdentification,
    Address,
    PlainText,
    RevolutionOrMeasurement,
    IncrementPerInputPulseOnChannelP,
    IncrementPerOutputPulseOnChannelP,
    HourMinuteSecond,
    DayMonthYear,
    StartDateOf,
    VifContinsUncorrectedUnitOrValue,
    AccumulationOnlyIfValueIsPositive,
    AccumulationOnlyIfValueIsNegative,
    NoneMetricUnits,
    ValueAtBaseConditions,
    ObisDecleration,
    UpperLimitValue,
    LowerLimitValue,
    NumberOfExceedsOfUpperLimitValue,
    NumberOfExceedsOfLowerLimitValue,
    DateOfBeginFirstLowerLimitExceed,
    DateOfBeginFirstUpperLimitExceed,
    DateOfBeginLastLowerLimitExceed,
    DateOfBeginLastUpperLimitExceed,
    DateOfEndLastLowerLimitExceed,
    DateOfEndLastUpperLimitExceed,
    DateOfEndFirstLowerLimitExceed,
    DateOfEndFirstUpperLimitExceed,
    DurationOfFirstLowerLimitExceed,
    DurationOfFirstUpperLimitExceed,
    DurationOfLastLowerLimitExceed,
    DurationOfLastUpperLimitExceed,
    DurationOfFirst,
    DurationOfLast,
    ValueDuringLowerValueExeed,
    ValueDuringUpperValueExceed,
    LeakageValues,
    OverflowValues,
    DateOfBeginLast,
    DateOfBeginFirst,
    DateOfEndLast,
    DateOfEndFirst,
    ExtensionOfCombinableOrthogonalVIFE,
    MultiplicativeCorrectionFactor103,
    FutureValue,
    NextVIFEAndDataOfThisBlockAreManufacturerSpecific,
    Credit,
    Debit,
    UniqueMessageIdentificationOrAccessNumber,
    DeviceType,
    Manufacturer,
    ParameterSetIdentification,
    ModelOrVersion,
    HardwareVersion,
    MetrologyFirmwareVersion,
    OtherSoftwareVersion,
    CustomerLocation,
    Customer,
    AccessCodeUser,
    AccessCodeOperator,
    AccessCodeSystemOperator,
    AccessCodeDeveloper,
    Password,
    ErrorFlags,
    ErrorMask,
    SecurityKey,
    DigitalInput,
    DigitalOutput,
    Binary,
    BaudRate,
    ResponseDelayTime,
    Retry,
    RemoteControl,
    FirstStorageForCycleStorage,
    LastStorageForCycleStorage,
    SizeOfStorageBlock,
    DescripitonOfTariffAndSubunit,
    StorageInterval,
    DimensionlessHCA,
    DataContainerForWmbusProtocol,
    PeriodOfNormalDataTransmition,
    ResetCounter,
    CumulationCounter,
    ControlSignal,
    DayOfWeek,
    WeekNumber,
    TimePointOfChangeOfTariff,
    StateOfParameterActivation,
    SpecialSupplierInformation,
    DurationSinceLastCumulation,
    OperatingTimeBattery,
    DateAndTimeOfBatteryChange,
    RFPowerLevel,
    DaylightSavingBeginningEndingDeviation,
    ListeningWindowManagementData,
    RemainingBatteryLifeTime,
    NumberOfTimesTheMeterWasStopped,
    DataContainerForManufacturerSpecificProtocol,
    CurrentlySelectedApplication,
    Energy,
    AtPhaseL1,
    AtPhaseL2,
    AtPhaseL3,
    AtNeutral,
    BetweenPhasesL1L2,
    BetweenPhasesL2L3,
    BetweenPhasesL3L1,
    AtQuadrant1,
    AtQuadrant2,
    AtQuadrant3,
    AtQuadrant4,
    DeltaBetweenImportAndExport,
    AccumulationOfAbsoluteValueBothPositiveAndNegativeContribution,
    DataPresentedWithTypeC,
    DataPresentedWithTypeD,
    DirectionFromCommunicationPartnerToMeter,
    DirectionFromMeterToCommunicationPartner,
    RelativeHumidity,
    PhaseUtoU,
    PhaseUtoI,
    ColdWarmTemperatureLimit,
    CumaltiveMaximumOfActivePower,
    ResultingRatingFactor,
    ThermalOutputRatingFactor,
    ThermalCouplingRatingFactorOverall,
    ThermalCouplingRatingRoomSide,
    ThermalCouplingRatingFactorHeatingSide,
    LowTemperatureRatingFactor,
    DisplayOutputScalingFactor,
    ManufacturerSpecific,
}

#[cfg(feature = "std")]
impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let superscripts = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
        let invalid_superscript = '⁻';
        match self.exponent {
            1 => write!(f, "{}", self.name),
            0..=9 => write!(
                f,
                "{}{}",
                self.name,
                superscripts
                    .get(self.exponent as usize)
                    .unwrap_or(&invalid_superscript)
            ),
            10..=19 => write!(
                f,
                "{}{}{}",
                self.name,
                superscripts.get(1).unwrap_or(&invalid_superscript),
                superscripts
                    .get(self.exponent as usize - 10)
                    .unwrap_or(&invalid_superscript)
            ),
            x if (-9..0).contains(&x) => {
                write!(
                    f,
                    "{}⁻{}",
                    self.name,
                    superscripts
                        .get((-x) as usize)
                        .unwrap_or(&invalid_superscript)
                )
            }
            x if (-19..0).contains(&x) => write!(
                f,
                "{}⁻{}{}",
                self.name,
                superscripts.get(1).unwrap_or(&invalid_superscript),
                superscripts
                    .get((-x) as usize - 10)
                    .unwrap_or(&invalid_superscript)
            ),
            x => write!(f, "{}^{}", self.name, x),
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitName {
    Watt,
    ReactiveWatt,
    ApparentWatt,
    Joul,
    Kilogram,
    Tonne,
    Meter,
    Feet,
    Celsius,
    Kelvin,
    Bar,
    HCA,
    Reserved,
    WithoutUnits,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
    Revolution,
    Increment,
    InputPulseOnChannel0,
    OutputPulseOnChannel0,
    InputPulseOnChannel1,
    OutputPulseOnChannel1,
    Liter,
    Volt,
    Ampere,
    LocalMoneyCurrency,
    Symbol,
    BitTime,
    DecibelMilliWatt,
    Percent,
    Degree,
    Hertz,
    HCAUnit,
}

#[cfg(feature = "std")]
impl fmt::Display for UnitName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnitName::Watt => write!(f, "W"),
            UnitName::ReactiveWatt => write!(f, "W (reactive)"),
            UnitName::ApparentWatt => write!(f, "W (apparent)"),
            UnitName::Joul => write!(f, "J"),
            UnitName::Kilogram => write!(f, "Kg"),
            UnitName::Tonne => write!(f, "t"),
            UnitName::Meter => write!(f, "m"),
            UnitName::Feet => write!(f, "ft"),
            UnitName::Celsius => write!(f, "°C"),
            UnitName::Kelvin => write!(f, "°K"),
            UnitName::Bar => write!(f, "Bar"),
            UnitName::HCA => write!(f, "HCA"),
            UnitName::Reserved => write!(f, "Reserved"),
            UnitName::WithoutUnits => write!(f, "-"),
            UnitName::Second => write!(f, "s"),
            UnitName::Minute => write!(f, "min"),
            UnitName::Hour => write!(f, "h"),
            UnitName::Day => write!(f, "day"),
            UnitName::Week => write!(f, "week"),
            UnitName::Month => write!(f, "month"),
            UnitName::Year => write!(f, "year"),
            UnitName::Revolution => write!(f, "revolution"),
            UnitName::Increment => write!(f, "increment"),
            UnitName::InputPulseOnChannel0 => write!(f, "InputPulseOnChannel0"),
            UnitName::OutputPulseOnChannel0 => write!(f, "OutputPulseOnChannel0"),
            UnitName::InputPulseOnChannel1 => write!(f, "InputPulseOnChannel1"),
            UnitName::OutputPulseOnChannel1 => write!(f, "OutputPulseOnChannel1"),
            UnitName::Liter => write!(f, "l"),
            UnitName::Volt => write!(f, "A"),
            UnitName::Ampere => write!(f, "A"),
            UnitName::LocalMoneyCurrency => write!(f, "$ (local)"),
            UnitName::Symbol => write!(f, "Symbol"),
            UnitName::BitTime => write!(f, "BitTime"),
            UnitName::DecibelMilliWatt => write!(f, "dBmW"),
            UnitName::Percent => write!(f, "%"),
            UnitName::Degree => write!(f, "°"),
            UnitName::Hertz => write!(f, "Hz"),
            UnitName::HCAUnit => write!(f, "HCAUnit"),
        }
    }
}

mod tests {

    #[test]
    fn test_single_byte_primary_value_information_parsing() {
        use crate::user_data::value_information::UnitName;
        use crate::user_data::value_information::{
            Unit, ValueInformation, ValueInformationBlock, ValueInformationField, ValueLabel,
        };
        use arrayvec::ArrayVec;

        /* VIB = 0x13 => m3^3*1e-3 */
        let data = [0x13];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x13),
                value_information_extension: None,
                plaintext_vife: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                decimal_offset_exponent: 0,
                decimal_scale_exponent: -3,
                units: {
                    let mut x = ArrayVec::<Unit, 10>::new();
                    x.push(unit!(Meter ^ 3));
                    x
                },
                labels: ArrayVec::<ValueLabel, 10>::new()
            }
        );

        /* VIB = 0x14 => m3^-3*1e-2 */
        let data = [0x14];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x14),
                value_information_extension: None,
                plaintext_vife: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                decimal_offset_exponent: 0,
                decimal_scale_exponent: -2,
                units: {
                    let mut x = ArrayVec::<Unit, 10>::new();
                    x.push(unit!(Meter ^ 3));
                    x
                },
                labels: ArrayVec::<ValueLabel, 10>::new()
            }
        );

        /* VIB = 0x15 => m3^3*1e-2 */
        let data = [0x15];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x15),
                value_information_extension: None,
                plaintext_vife: None
            }
        );
        assert_eq!(result.get_size(), 1);
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                decimal_offset_exponent: 0,
                decimal_scale_exponent: -1,
                units: {
                    let mut x = ArrayVec::<Unit, 10>::new();
                    x.push(unit!(Meter ^ 3));
                    x
                },
                labels: ArrayVec::<ValueLabel, 10>::new()
            }
        );

        /* VIB = 0x16 => m3^-3*1e-1 */
        let data = [0x16];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(
            result,
            ValueInformationBlock {
                value_information: ValueInformationField::from(0x16),
                value_information_extension: None,
                plaintext_vife: None
            },
        );
        assert_eq!(result.get_size(), 1);
    }

    #[test]
    fn test_multibyte_primary_value_information() {
        use crate::user_data::value_information::UnitName;
        use crate::user_data::value_information::{
            Unit, ValueInformation, ValueInformationBlock, ValueInformationField, ValueLabel,
        };
        use arrayvec::ArrayVec;
        /* 1 VIF, 1 - 10 orthogonal VIFE */

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x12 => Combinable Orthogonal VIFE meaning "averaged" */
        /* VIB = 0x96, 0x12 */
        let data = [0x96, 0x12];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);
        assert_eq!(result.value_information, ValueInformationField::from(0x96));
        assert_eq!(ValueInformation::try_from(&result).unwrap().labels, {
            let mut x = ArrayVec::<ValueLabel, 10>::new();
            x.push(ValueLabel::Averaged);
            x
        });

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x92 = 0x12 | 0x80  => Combinable Orthogonal VIFE meaning "averaged" with extension */
        /* VIFE 0x20 => Combinable Orthogonal VIFE meaning "per second" */
        /* VIB = 0x96, 0x92,0x20 */

        let data = [0x96, 0x92, 0x20];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 3);
        assert_eq!(result.value_information, ValueInformationField::from(0x96));
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                labels: {
                    let mut x = ArrayVec::<ValueLabel, 10>::new();
                    x.push(ValueLabel::Averaged);
                    x
                },
                decimal_offset_exponent: 0,
                decimal_scale_exponent: 0,
                units: {
                    let mut x = ArrayVec::<Unit, 10>::new();
                    x.push(unit!(Meter ^ 3));
                    x.push(unit!(Second ^ -1));
                    x
                }
            }
        );

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x92 = 0x12 | 0x80  => Combinable Orthogonal VIFE meaning "averaged" with extension */
        /* VIFE 0xA0= 0x20 | 0x80 => Combinable Orthogonal VIFE meaning "per second" */
        /* VIFE 0x2D => Combinable Orthogonal VIFE meaning "per m3". This cancels out the VIF m3, which is useless
        but till a valid VIB */
        /* VIB = 0x96, 0x92,0xA0, 0x2D */
        let data = [0x96, 0x92, 0xA0, 0x2D];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 4);
        assert_eq!(result.value_information, ValueInformationField::from(0x96));
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                labels: {
                    let mut x = ArrayVec::<ValueLabel, 10>::new();
                    x.push(ValueLabel::Averaged);
                    x
                },
                decimal_offset_exponent: 0,
                decimal_scale_exponent: 0,
                units: {
                    let mut x = ArrayVec::<Unit, 10>::new();
                    x.push(unit!(Meter ^ 3));
                    x.push(unit!(Second ^ -1));
                    x.push(unit!(Meter ^ -3));
                    x
                }
            }
        );
    }

    #[cfg(not(feature = "plaintext-before-extension"))]
    #[test]
    fn test_plain_text_vif_norm_conform() {
        use arrayvec::ArrayVec;

        use crate::user_data::value_information::{Unit, ValueInformation, ValueLabel};

        use crate::user_data::value_information::ValueInformationBlock;
        // This is the ascii conform method of encoding the VIF
        // VIF  VIFE  LEN(3) 'R'   'H'  '%'
        // 0xFC, 0x74, 0x03, 0x52, 0x48, 0x25,
        // %RH
        // Combinable (orthogonal) VIFE-Code extension table
        // VIFE = 0x74 => E111 0nnn Multiplicative correction factor for value (not unit): 10nnn–6 => 10^-2
        //
        // according to the Norm the LEN and ASCII is not part tof the VIB however this makes parsing
        // cumbersome so we include it in the VIB

        let data = [0xFC, 0x74, 0x03, 0x52, 0x48, 0x25];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 6);
        assert_eq!(result.value_information.data, 0xFC);
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                decimal_offset_exponent: 0,
                decimal_scale_exponent: 0,
                units: { ArrayVec::<Unit, 10>::new() },
                labels: {
                    let mut x = ArrayVec::<ValueLabel, 10>::new();
                    x.push(ValueLabel::PlainText);
                    x
                }
            }
        );

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

    #[test]
    fn test_short_vif_with_vife() {
        use crate::user_data::value_information::ValueInformationBlock;
        let data = [253, 27];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);
    }
}
