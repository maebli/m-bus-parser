#[cfg(feature = "std")]
use std::fmt;

use super::data_information::DataInformationError;

#[derive(Clone, Copy, Debug, PartialEq)]
struct VifInfo {
    labels: &'static [ValueLabel],
    units: &'static [Unit],
    scale: isize,
    offset: isize,
}
impl VifInfo {
    const EMPTY: Self = Self {
        labels: &[],
        units: &[],
        scale: 0,
        offset: 0,
    };
}
macro_rules! labels {
    ($($label:expr),+ $(,)?) => { VifInfo { labels: &[$($label),+], ..VifInfo::EMPTY } };
}
macro_rules! units {
    ($($unit:expr),+ $(,)?) => { VifInfo { units: &[$($unit),+], ..VifInfo::EMPTY } };
}

const MAX_VIFE_RECORDS: usize = 10;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

impl<'a> TryFrom<&'a [u8]> for ValueInformationBlock<'a> {
    type Error = DataInformationError;

    fn try_from(data: &'a [u8]) -> Result<Self, DataInformationError> {
        let vif =
            ValueInformationField::from(*data.first().ok_or(DataInformationError::DataTooShort)?);
        let mut offset = 1;
        let mut value_information_extension = None;
        let mut plaintext_vife = None;

        #[cfg(feature = "plaintext-before-extension")]
        if vif.value_information_contains_ascii() {
            let plaintext = PlainTextValueInformationExtension::new(
                data.get(offset..)
                    .ok_or(DataInformationError::DataTooShort)?,
            )?;
            offset += plaintext.ascii_len() + 1;
            plaintext_vife = Some(plaintext);
        }

        if vif.has_extension() {
            // When the plaintext VIF precedes the extensions, the VIFE chain
            // starts after the ASCII length byte and string, not at offset 1.
            let extensions = ValueInformationFieldExtensions::new(
                data.get(offset..)
                    .ok_or(DataInformationError::DataTooShort)?,
            )?;
            #[cfg(not(feature = "plaintext-before-extension"))]
            {
                offset += extensions.len();
            }
            value_information_extension = Some(extensions);
        }

        #[cfg(not(feature = "plaintext-before-extension"))]
        if vif.value_information_contains_ascii() {
            plaintext_vife = Some(PlainTextValueInformationExtension::new(
                data.get(offset..)
                    .ok_or(DataInformationError::DataTooShort)?,
            )?);
        }

        Ok(Self {
            value_information: vif,
            value_information_extension,
            plaintext_vife,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Clone)]
pub struct ValueInformationBlock<'a> {
    pub value_information: ValueInformationField,
    pub value_information_extension: Option<ValueInformationFieldExtensions<'a>>,
    pub plaintext_vife: Option<PlainTextValueInformationExtension<'a>>,
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for ValueInformationBlock<'a> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "ValueInformationBlock{{ value_information: {:?}",
            self.value_information
        );
        if let Some(ext) = &self.value_information_extension {
            defmt::write!(f, ", value_information_extension: [");
            ext.iter().for_each(|x| defmt::write!(f, "{},", x));
            defmt::write!(f, "]");
        }
        if let Some(text) = &self.plaintext_vife {
            defmt::write!(f, ", plaintext_vife: {}", text.as_ascii_str());
        }
        defmt::write!(f, " }}");
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ValueInformationField {
    pub data: u8,
}

impl ValueInformationField {
    const fn value_information_contains_ascii(&self) -> bool {
        self.data == 0x7C || self.data == 0xFC
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ValueInformationFieldExtensions<'a>(&'a [u8]);

#[cfg(feature = "serde")]
impl serde::Serialize for ValueInformationFieldExtensions<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

impl<'a> ValueInformationFieldExtensions<'a> {
    fn new(data: &'a [u8]) -> Result<Self, DataInformationError> {
        let Some(last_index) = data
            .iter()
            .take(MAX_VIFE_RECORDS + 1)
            .position(|byte| byte & 0x80 == 0)
        else {
            return Err(if data.len() > MAX_VIFE_RECORDS {
                DataInformationError::InvalidValueInformation
            } else {
                DataInformationError::DataTooShort
            });
        };

        let length = last_index + 1;
        if length > MAX_VIFE_RECORDS {
            return Err(DataInformationError::InvalidValueInformation);
        }

        Ok(Self(
            data.get(..length)
                .ok_or(DataInformationError::DataTooShort)?,
        ))
    }
}

impl Iterator for ValueInformationFieldExtensions<'_> {
    type Item = ValueInformationFieldExtension;
    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(ValueInformationFieldExtension { data: *head })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl ExactSizeIterator for ValueInformationFieldExtensions<'_> {}
impl DoubleEndedIterator for ValueInformationFieldExtensions<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (end, start) = self.0.split_last()?;
        self.0 = start;
        Some(ValueInformationFieldExtension { data: *end })
    }
}

impl<'a> ValueInformationFieldExtensions<'a> {
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = ValueInformationFieldExtension> + ExactSizeIterator + '_
    {
        self.0
            .iter()
            .copied()
            .map(|data| ValueInformationFieldExtension { data })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PlainTextValueInformationExtension<'a>(&'a [u8]);

#[cfg(feature = "serde")]
impl serde::Serialize for PlainTextValueInformationExtension<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let plaintext = self.as_ascii_str().ok_or_else(|| {
            <S::Error as serde::ser::Error>::custom("invalid plaintext VIFE encoding")
        })?;

        serializer.collect_seq(plaintext.chars())
    }
}

impl<'a> PlainTextValueInformationExtension<'a> {
    fn new(data: &'a [u8]) -> Result<Self, DataInformationError> {
        let ascii_len = usize::from(*data.first().ok_or(DataInformationError::DataTooShort)?);

        if ascii_len > 9 {
            return Err(DataInformationError::InvalidValueInformation);
        }

        let encoded = data
            .get(..ascii_len + 1)
            .ok_or(DataInformationError::DataTooShort)?;

        if !encoded[1..].is_ascii() {
            return Err(DataInformationError::InvalidValueInformation);
        }

        Ok(Self(encoded))
    }

    pub const fn ascii_len(&self) -> usize {
        if let Some(x) = self.0.first() {
            *x as usize
        } else {
            0
        }
    }

    pub fn as_ascii_str(&self) -> Option<&str> {
        core::str::from_utf8(self.0.get(1..)?).ok()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ValueInformationFieldExtension {
    pub data: u8,
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ValueInformationCoding {
    Primary,
    PlainText,
    MainVIFExtension,
    AlternateVIFExtension,
    ManufacturerSpecific,
}

impl<'a> ValueInformationBlock<'a> {
    pub fn new(
        value_information: ValueInformationField,
        value_information_extension: Option<ValueInformationFieldExtensions<'a>>,
        plaintext_vife: Option<PlainTextValueInformationExtension<'a>>,
    ) -> Self {
        Self {
            value_information,
            value_information_extension,
            plaintext_vife,
        }
    }

    #[must_use]
    pub fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.value_information_extension {
            size += vife.iter().count();
        }
        if let Some(plaintext_vife) = &self.plaintext_vife {
            // 1 byte for the length of the ASCII string
            size += plaintext_vife.ascii_len() + 1;
        }
        size
    }
}

fn head_vif_info(
    vif: ValueInformationField,
    first_vife: Option<u8>,
    second_vife_data: Option<u8>,
) -> Result<VifInfo, DataInformationError> {
    Ok(match ValueInformationCoding::from(&vif) {
        ValueInformationCoding::Primary => match vif.data & 0x7F {
            0x00..=0x07 => VifInfo {
                labels: &[ValueLabel::Energy],
                units: &[unit!(Watt), unit!(Hour)],
                scale: (vif.data & 0b111) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x08..=0x0F => VifInfo {
                labels: &[ValueLabel::Energy],
                units: &[unit!(Joul)],
                scale: (vif.data & 0b111) as isize,
                ..VifInfo::EMPTY
            },
            0x10..=0x17 => VifInfo {
                labels: &[ValueLabel::Volume],
                units: &[unit!(Meter ^ 3)],
                scale: (vif.data & 0b111) as isize - 6,
                ..VifInfo::EMPTY
            },
            0x18..=0x1F => VifInfo {
                labels: &[ValueLabel::Mass],
                units: &[unit!(Kilogram)],
                scale: (vif.data & 0b111) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x20..=0x23 => {
                return Ok(VifInfo {
                    labels: &[ValueLabel::OnTime],
                    units: match vif.data & 3 {
                        0 => &[unit!(Second)],
                        1 => &[unit!(Minute)],
                        2 => &[unit!(Hour)],
                        _ => &[unit!(Day)],
                    },
                    ..VifInfo::EMPTY
                });
            }
            0x24..=0x27 => {
                return Ok(VifInfo {
                    labels: &[ValueLabel::OperatingTime],
                    units: match vif.data & 3 {
                        0 => &[unit!(Second)],
                        1 => &[unit!(Minute)],
                        2 => &[unit!(Hour)],
                        _ => &[unit!(Day)],
                    },
                    ..VifInfo::EMPTY
                });
            }
            0x28..=0x2F => VifInfo {
                labels: &[ValueLabel::Power],
                units: &[unit!(Watt)],
                scale: (vif.data & 0b111) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x30..=0x37 => VifInfo {
                labels: &[ValueLabel::Power],
                units: &[unit!(Joul), unit!(Hour ^ -1)],
                scale: (vif.data & 0b111) as isize,
                ..VifInfo::EMPTY
            },
            0x38..=0x3F => VifInfo {
                labels: &[ValueLabel::VolumeFlow],
                units: &[unit!(Meter ^ 3), unit!(Hour ^ -1)],
                scale: (vif.data & 0b111) as isize - 6,
                ..VifInfo::EMPTY
            },
            0x40..=0x47 => VifInfo {
                labels: &[ValueLabel::VolumeFlow],
                units: &[unit!(Meter ^ 3), unit!(Minute ^ -1)],
                scale: (vif.data & 0b111) as isize - 7,
                ..VifInfo::EMPTY
            },
            0x48..=0x4F => VifInfo {
                labels: &[ValueLabel::VolumeFlow],
                units: &[unit!(Meter ^ 3), unit!(Second ^ -1)],
                scale: (vif.data & 0b111) as isize - 9,
                ..VifInfo::EMPTY
            },
            0x50..=0x57 => VifInfo {
                labels: &[ValueLabel::MassFlow],
                units: &[unit!(Kilogram), unit!(Hour ^ -1)],
                scale: (vif.data & 0b111) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x58..=0x5B => VifInfo {
                labels: &[ValueLabel::FlowTemperature],
                units: &[unit!(Celsius)],
                scale: (vif.data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x5C..=0x5F => VifInfo {
                labels: &[ValueLabel::ReturnTemperature],
                units: &[unit!(Celsius)],
                scale: (vif.data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x60..=0x63 => VifInfo {
                labels: &[ValueLabel::TemperatureDifference],
                units: &[unit!(Kelvin)],
                scale: (vif.data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x64..=0x67 => VifInfo {
                labels: &[ValueLabel::ExternalTemperature],
                units: &[unit!(Celsius)],
                scale: (vif.data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x68..=0x6B => VifInfo {
                labels: &[ValueLabel::Pressure],
                units: &[unit!(Bar)],
                scale: (vif.data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x6C => labels!(ValueLabel::Date),
            0x6D => labels!(ValueLabel::DateTime),
            0x6E => labels!(ValueLabel::DimensionlessHCA),
            0x70..=0x73 => labels!(ValueLabel::AveragingDuration),
            0x74..=0x77 => labels!(ValueLabel::ActualityDuration),
            0x78 => labels!(ValueLabel::FabricationNumber),
            0x79 => labels!(ValueLabel::EnhancedIdentification),
            0x7A => labels!(ValueLabel::Address),
            0x7B => VifInfo::EMPTY,

            _ => {
                return Err(DataInformationError::Unimplemented {
                    feature: "Primary value information unit codes (partial)",
                })
            }
        },
        ValueInformationCoding::MainVIFExtension => {
            let Some(first_vife_data) = first_vife else {
                return Ok(VifInfo::EMPTY);
            };
            match first_vife_data & 0x7F {
                0x00..=0x03 => VifInfo {
                    labels: &[ValueLabel::Credit],
                    units: &[unit!(LocalMoneyCurrency)],
                    scale: (first_vife_data & 0b11) as isize - 3,
                    ..VifInfo::EMPTY
                },
                0x04..=0x07 => VifInfo {
                    labels: &[ValueLabel::Debit],
                    units: &[unit!(LocalMoneyCurrency)],
                    scale: (first_vife_data & 0b11) as isize - 3,
                    ..VifInfo::EMPTY
                },
                0x08 => labels!(ValueLabel::UniqueMessageIdentificationOrAccessNumber),
                0x09 => labels!(ValueLabel::DeviceType),
                0x0A => labels!(ValueLabel::Manufacturer),
                0x0B => labels!(ValueLabel::ParameterSetIdentification),
                0x0C => labels!(ValueLabel::ModelOrVersion),
                0x0D => labels!(ValueLabel::HardwareVersion),
                0x0E => labels!(ValueLabel::MetrologyFirmwareVersion),
                0x0F => labels!(ValueLabel::OtherSoftwareVersion),
                0x10 => labels!(ValueLabel::CustomerLocation),
                0x11 => labels!(ValueLabel::Customer),
                0x12 => labels!(ValueLabel::AccessCodeUser),
                0x13 => labels!(ValueLabel::AccessCodeOperator),
                0x14 => labels!(ValueLabel::AccessCodeSystemOperator),
                0x15 => labels!(ValueLabel::AccessCodeDeveloper),
                0x16 => labels!(ValueLabel::Password),
                0x17 => labels!(ValueLabel::ErrorFlags),
                0x18 => labels!(ValueLabel::ErrorMask),
                0x19 => labels!(ValueLabel::SecurityKey),
                0x1A => VifInfo {
                    labels: &[ValueLabel::DigitalOutput, ValueLabel::Binary],
                    ..VifInfo::EMPTY
                },
                0x1B => VifInfo {
                    labels: &[ValueLabel::DigitalInput, ValueLabel::Binary],
                    ..VifInfo::EMPTY
                },
                0x1C => VifInfo {
                    labels: &[ValueLabel::BaudRate],
                    units: &[unit!(Symbol), unit!(Second ^ -1)],
                    ..VifInfo::EMPTY
                },
                0x1D => VifInfo {
                    labels: &[ValueLabel::ResponseDelayTime],
                    units: &[unit!(BitTime)],
                    ..VifInfo::EMPTY
                },
                0x1E => labels!(ValueLabel::Retry),
                0x1F => labels!(ValueLabel::RemoteControl),
                0x20 => labels!(ValueLabel::FirstStorageForCycleStorage),
                0x21 => labels!(ValueLabel::LastStorageForCycleStorage),
                0x22 => labels!(ValueLabel::SizeOfStorageBlock),
                0x23 => labels!(ValueLabel::DescriptionOfTariffAndSubunit),
                0x24 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Second)],
                    ..VifInfo::EMPTY
                },
                0x25 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Minute)],
                    ..VifInfo::EMPTY
                },
                0x26 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Hour)],
                    ..VifInfo::EMPTY
                },
                0x27 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Day)],
                    ..VifInfo::EMPTY
                },
                0x28 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Month)],
                    ..VifInfo::EMPTY
                },
                0x29 => VifInfo {
                    labels: &[ValueLabel::StorageInterval],
                    units: &[unit!(Year)],
                    ..VifInfo::EMPTY
                },
                0x30 => labels!(ValueLabel::DimensionlessHCA),
                0x31 => labels!(ValueLabel::DataContainerForWmbusProtocol),
                0x32 => VifInfo {
                    labels: &[ValueLabel::PeriodOfNormalDataTransmission],
                    units: &[unit!(Second)],
                    ..VifInfo::EMPTY
                },
                0x33 => VifInfo {
                    labels: &[ValueLabel::PeriodOfNormalDataTransmission],
                    units: &[unit!(Meter)],
                    ..VifInfo::EMPTY
                },
                0x34 => VifInfo {
                    labels: &[ValueLabel::PeriodOfNormalDataTransmission],
                    units: &[unit!(Hour)],
                    ..VifInfo::EMPTY
                },
                0x35 => VifInfo {
                    labels: &[ValueLabel::PeriodOfNormalDataTransmission],
                    units: &[unit!(Day)],
                    ..VifInfo::EMPTY
                },
                0x3A => labels!(ValueLabel::Dimensionless),
                0x40..=0x4F => VifInfo {
                    labels: &[ValueLabel::Voltage],
                    units: &[unit!(Volt)],
                    scale: (first_vife_data & 0b1111) as isize - 9,
                    ..VifInfo::EMPTY
                },
                0x50..=0x5F => VifInfo {
                    labels: &[ValueLabel::Current],
                    units: &[unit!(Ampere)],
                    scale: (first_vife_data & 0b1111) as isize - 12,
                    ..VifInfo::EMPTY
                },
                0x60 => labels!(ValueLabel::ResetCounter),
                0x61 => labels!(ValueLabel::CumulationCounter),
                0x62 => labels!(ValueLabel::ControlSignal),
                0x63 => labels!(ValueLabel::DayOfWeek),
                0x64 => labels!(ValueLabel::WeekNumber),
                0x65 => labels!(ValueLabel::TimePointOfChangeOfTariff),
                0x66 => labels!(ValueLabel::StateOfParameterActivation),
                0x67 => labels!(ValueLabel::SpecialSupplierInformation),
                0x68 => VifInfo {
                    labels: &[ValueLabel::DurationSinceLastCumulation],
                    units: &[unit!(Hour)],
                    ..VifInfo::EMPTY
                },
                0x69 => VifInfo {
                    labels: &[ValueLabel::DurationSinceLastCumulation],
                    units: &[unit!(Day)],
                    ..VifInfo::EMPTY
                },
                0x6A => VifInfo {
                    labels: &[ValueLabel::DurationSinceLastCumulation],
                    units: &[unit!(Month)],
                    ..VifInfo::EMPTY
                },
                0x6B => VifInfo {
                    labels: &[ValueLabel::DurationSinceLastCumulation],
                    units: &[unit!(Year)],
                    ..VifInfo::EMPTY
                },
                0x6C => VifInfo {
                    labels: &[ValueLabel::OperatingTimeBattery],
                    units: &[unit!(Hour)],
                    ..VifInfo::EMPTY
                },
                0x6D => VifInfo {
                    labels: &[ValueLabel::OperatingTimeBattery],
                    units: &[unit!(Day)],
                    ..VifInfo::EMPTY
                },
                0x6E => VifInfo {
                    labels: &[ValueLabel::OperatingTimeBattery],
                    units: &[unit!(Month)],
                    ..VifInfo::EMPTY
                },
                0x6F => VifInfo {
                    labels: &[ValueLabel::OperatingTimeBattery],
                    units: &[unit!(Hour)],
                    ..VifInfo::EMPTY
                },
                0x70 => VifInfo {
                    labels: &[ValueLabel::DateAndTimeOfBatteryChange],
                    units: &[unit!(Second)],
                    ..VifInfo::EMPTY
                },
                0x71 => VifInfo {
                    labels: &[ValueLabel::RFPowerLevel],
                    units: &[unit!(DecibelMilliWatt)],
                    ..VifInfo::EMPTY
                },
                0x72 => labels!(ValueLabel::DaylightSavingBeginningEndingDeviation),
                0x73 => labels!(ValueLabel::ListeningWindowManagementData),
                0x74 => labels!(ValueLabel::RemainingBatteryLifeTime),
                0x75 => labels!(ValueLabel::NumberOfTimesTheMeterWasStopped),
                0x76 => VifInfo {
                    labels: &[ValueLabel::DataContainerForManufacturerSpecificProtocol],
                    ..VifInfo::EMPTY
                },
                0x7D => match second_vife_data.map(|s| s & 0x7F) {
                    Some(0x00) => labels!(ValueLabel::CurrentlySelectedApplication),
                    Some(0x02) => VifInfo {
                        labels: &[ValueLabel::RemainingBatteryLifeTime],
                        units: &[unit!(Month)],
                        ..VifInfo::EMPTY
                    },
                    Some(0x03) => VifInfo {
                        labels: &[ValueLabel::RemainingBatteryLifeTime],
                        units: &[unit!(Year)],
                        ..VifInfo::EMPTY
                    },
                    Some(0x3E) => VifInfo {
                        labels: &[ValueLabel::MoistureLevel],
                        units: &[unit!(Percent)],
                        ..VifInfo::EMPTY
                    },
                    _ => labels!(ValueLabel::Reserved),
                },
                _ => labels!(ValueLabel::Reserved),
            }
        }
        ValueInformationCoding::AlternateVIFExtension => {
            use UnitName::*;
            use ValueLabel::*;
            macro_rules! populate {
                ($name:ident / h, $exp:expr, dec: $d:literal, $label:expr) => {
                    VifInfo {
                        units: &[
                            Unit {
                                name: $name,
                                exponent: $exp,
                            },
                            Unit {
                                name: Hour,
                                exponent: -1,
                            },
                        ],
                        labels: &[$label],
                        scale: $d,
                        offset: 0,
                    }
                };
                ($name:ident / min, $exp:expr, dec: $d:literal, $label:expr) => {
                    VifInfo {
                        units: &[
                            Unit {
                                name: $name,
                                exponent: $exp,
                            },
                            Unit {
                                name: Minute,
                                exponent: -1,
                            },
                        ],
                        labels: &[$label],
                        scale: $d,
                        offset: 0,
                    }
                };
                ($name:ident * h, $exp:expr, dec: $d:literal, $label:expr) => {
                    VifInfo {
                        units: &[
                            Unit {
                                name: $name,
                                exponent: $exp,
                            },
                            Unit {
                                name: Hour,
                                exponent: 1,
                            },
                        ],
                        labels: &[$label],
                        scale: $d,
                        offset: 0,
                    }
                };
                ($name:ident , $exp:expr, dec: $d:literal, $label:expr) => {
                    VifInfo {
                        units: &[Unit {
                            name: $name,
                            exponent: $exp,
                        }],
                        labels: &[$label],
                        scale: $d,
                        offset: 0,
                    }
                };
            }

            let Some(first_vife_data) = first_vife else {
                return Ok(VifInfo::EMPTY);
            };
            match first_vife_data & 0x7F {
                0b0 => populate!(Watt / h, 3, dec: 5, Energy),
                0b000_0001 => populate!(Watt / h, 3, dec: 6, Energy),
                0b000_0010 => populate!(ReactiveWatt * h, 1, dec: 3, ReactiveEnergy),
                0b000_0011 => populate!(ReactiveWatt * h, 1, dec: 4, ReactiveEnergy),
                0b000_0100 => populate!(ApparentWatt * h, 1, dec: 3, ApparentEnergy),
                0b000_0101 => populate!(ApparentWatt * h, 1, dec: 4, ApparentEnergy),
                0b000_0110 => VifInfo {
                    labels: &[CoefficientOfPerformance],
                    scale: -1,
                    ..VifInfo::EMPTY
                },
                0b000_1000 => populate!(Joul, 1, dec: 8, Energy),
                0b000_1001 => populate!(Joul, 1, dec: 9, Energy),
                0b000_1100 => populate!(Calorie, 1, dec: 5, Energy),
                0b000_1101 => populate!(Calorie, 1, dec: 6, Energy),
                0b000_1110 => populate!(Calorie, 1, dec: 7, Energy),
                0b000_1111 => populate!(Calorie, 1, dec: 8, Energy),
                0b001_0000 => populate!(Meter, 3, dec: 2, Volume),
                0b001_0001 => populate!(Meter, 3, dec: 3, Volume),
                0b001_0100 => populate!(ReactiveWatt, 1, dec: 0, ReactivePower),
                0b001_0101 => populate!(ReactiveWatt, 1, dec: 1, ReactivePower),
                0b001_0110 => populate!(ReactiveWatt, 1, dec: 2, ReactivePower),
                0b001_0111 => populate!(ReactiveWatt, 1, dec: 3, ReactivePower),
                0b001_1000 => populate!(Tonne, 1, dec: 2, Mass),
                0b001_1001 => populate!(Tonne, 1, dec: 3, Mass),
                0b001_1010 => populate!(Percent, 1, dec: -1, RelativeHumidity),
                0b001_1011 => populate!(Percent, 1, dec: 0, RelativeHumidity),
                0b010_0000 => populate!(Feet, 3, dec: 0, Volume),
                0b010_0001 => populate!(Feet, 3, dec: -1, Volume),
                0b010_0011 => populate!(Degree, 1, dec: -1, PhaseItoU),
                0b010_1000 => populate!(Watt, 1, dec: 5, Power),
                0b010_1001 => populate!(Watt, 1, dec: 6, Power),
                0b010_1010 => populate!(Degree, 1, dec: -1, PhaseUtoU),
                0b010_1011 => populate!(Degree, 1, dec: -1, PhaseUtoI),
                0b010_1100 => populate!(Hertz, 1, dec: -3, Frequency),
                0b010_1101 => populate!(Hertz, 1, dec: -2, Frequency),
                0b010_1110 => populate!(Hertz, 1, dec: -1, Frequency),
                0b010_1111 => populate!(Hertz, 1, dec: 0, Frequency),
                0b011_0000 => populate!(Joul / h, 1, dec: 8, Power),
                0b011_0001 => populate!(Joul / h, 1, dec: 9, Power),
                0b011_0100 => populate!(ApparentWatt, 1, dec: 0, ApparentPower),
                0b011_0101 => populate!(ApparentWatt, 1, dec: 1, ApparentPower),
                0b011_0110 => populate!(ApparentWatt, 1, dec: 2, ApparentPower),
                0b011_0111 => populate!(ApparentWatt, 1, dec: 3, ApparentPower),
                0b101_1000 => populate!(Fahrenheit, 1, dec: -3, FlowTemperature),
                0b101_1001 => populate!(Fahrenheit, 1, dec: -2, FlowTemperature),
                0b101_1010 => populate!(Fahrenheit, 1, dec: -1, FlowTemperature),
                0b101_1011 => populate!(Fahrenheit, 1, dec: 0, FlowTemperature),
                0b101_1100 => populate!(Fahrenheit, 1, dec: -3, ReturnTemperature),
                0b101_1101 => populate!(Fahrenheit, 1, dec: -2, ReturnTemperature),
                0b101_1110 => populate!(Fahrenheit, 1, dec: -1, ReturnTemperature),
                0b101_1111 => populate!(Fahrenheit, 1, dec: 0, ReturnTemperature),
                0b110_0000 => populate!(Fahrenheit, 1, dec: -3, TemperatureDifference),
                0b110_0001 => populate!(Fahrenheit, 1, dec: -2, TemperatureDifference),
                0b110_0010 => populate!(Fahrenheit, 1, dec: -1, TemperatureDifference),
                0b110_0011 => populate!(Fahrenheit, 1, dec: 0, TemperatureDifference),
                0b110_0100 => populate!(Fahrenheit, 1, dec: -3, ExternalTemperature),
                0b110_0101 => populate!(Fahrenheit, 1, dec: -2, ExternalTemperature),
                0b110_0110 => populate!(Fahrenheit, 1, dec: -1, ExternalTemperature),
                0b110_0111 => populate!(Fahrenheit, 1, dec: 0, ExternalTemperature),
                0b111_0000 => populate!(Fahrenheit, 1, dec: -3, ColdWarmTemperatureLimit),
                0b111_0001 => populate!(Fahrenheit, 1, dec: -2, ColdWarmTemperatureLimit),
                0b111_0010 => populate!(Fahrenheit, 1, dec: -1, ColdWarmTemperatureLimit),
                0b111_0011 => populate!(Fahrenheit, 1, dec: 0, ColdWarmTemperatureLimit),
                0b111_0100 => populate!(Celsius, 1, dec: -3, ColdWarmTemperatureLimit),
                0b111_0101 => populate!(Celsius, 1, dec: -2, ColdWarmTemperatureLimit),
                0b111_0110 => populate!(Celsius, 1, dec: -1, ColdWarmTemperatureLimit),
                0b111_0111 => populate!(Celsius, 1, dec: 0, ColdWarmTemperatureLimit),
                0b111_1000 => populate!(Watt, 1, dec: -3, CumulativeMaximumOfActivePower),
                0b111_1001 => populate!(Watt, 1, dec: -2, CumulativeMaximumOfActivePower),
                0b111_1010 => populate!(Watt, 1, dec: -1, CumulativeMaximumOfActivePower),
                0b111_1011 => populate!(Watt, 1, dec: 0, CumulativeMaximumOfActivePower),
                0b111_1100 => populate!(Watt, 1, dec: 1, CumulativeMaximumOfActivePower),
                0b111_1101 => populate!(Watt, 1, dec: 2, CumulativeMaximumOfActivePower),
                0b111_1110 => populate!(Watt, 1, dec: 3, CumulativeMaximumOfActivePower),
                0b111_1111 => populate!(Watt, 1, dec: 4, CumulativeMaximumOfActivePower),
                0b110_1000 => populate!(HCAUnit, 1,dec: 0, ResultingRatingFactor),
                0b110_1001 => populate!(HCAUnit, 1,dec: 0, ThermalOutputRatingFactor),
                0b110_1010 => {
                    populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingFactorOverall)
                }
                0b110_1011 => populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingRoomSide),
                0b110_1100 => {
                    populate!(HCAUnit, 1,dec: 0, ThermalCouplingRatingFactorHeatingSide)
                }
                0b110_1101 => populate!(HCAUnit, 1,dec: 0, LowTemperatureRatingFactor),
                0b110_1110 => populate!(HCAUnit, 1,dec: 0, DisplayOutputScalingFactor),

                _ => labels!(ValueLabel::Reserved),
            }
        }
        ValueInformationCoding::PlainText => labels!(ValueLabel::PlainText),
        ValueInformationCoding::ManufacturerSpecific => labels!(ValueLabel::ManufacturerSpecific),
    })
}
fn orthogonal_vife_info(data: u8, combinable_ext: bool) -> VifInfo {
    if combinable_ext {
        match data & 0x7F {
            0x00 => labels!(ValueLabel::Reserved),
            0x01 => labels!(ValueLabel::AtPhaseL1),
            0x02 => labels!(ValueLabel::AtPhaseL2),
            0x03 => labels!(ValueLabel::AtPhaseL3),
            0x04 => labels!(ValueLabel::AtNeutral),
            0x05 => labels!(ValueLabel::BetweenPhasesL1L2),
            0x06 => labels!(ValueLabel::BetweenPhasesL2L3),
            0x07 => labels!(ValueLabel::BetweenPhasesL3L1),
            0x08 => labels!(ValueLabel::AtQuadrant1),
            0x09 => labels!(ValueLabel::AtQuadrant2),
            0x0A => labels!(ValueLabel::AtQuadrant3),
            0x0B => labels!(ValueLabel::AtQuadrant4),
            0x0C => labels!(ValueLabel::DeltaBetweenImportAndExport),
            0x0D => labels!(ValueLabel::AlternativeNonMetricUnits),
            0x0E => labels!(ValueLabel::SecondarySensorMeasurement),
            0x0F => labels!(ValueLabel::HigherResolutionRegister),
            0x10 => {
                labels!(ValueLabel::AccumulationOfAbsoluteValueBothPositiveAndNegativeContribution)
            }
            0x11 => labels!(ValueLabel::DataPresentedWithTypeC),
            0x12 => labels!(ValueLabel::DataPresentedWithTypeD),
            0x13 => labels!(ValueLabel::EndDate),
            0x14 => labels!(ValueLabel::DirectionFromCommunicationPartnerToMeter),
            0x15 => labels!(ValueLabel::DirectionFromMeterToCommunicationPartner),
            _ => labels!(ValueLabel::Reserved),
        }
    } else {
        match data & 0x7F {
            0x00..=0x0F => labels!(ValueLabel::ReservedForObjectActions),
            0x10..=0x11 => labels!(ValueLabel::Reserved),
            0x12 => labels!(ValueLabel::Averaged),
            0x13 => labels!(ValueLabel::InverseCompactProfile),
            0x14 => labels!(ValueLabel::RelativeDeviation),
            0x15..=0x1C => labels!(ValueLabel::RecordErrorCodes),
            0x1D => labels!(ValueLabel::StandardConformDataContent),
            0x1E => labels!(ValueLabel::CompactProfileWithRegisterNumbers),
            0x1F => labels!(ValueLabel::CompactProfile),
            0x20 => units!(unit!(Second ^ -1)),
            0x21 => units!(unit!(Minute ^ -1)),
            0x22 => units!(unit!(Hour ^ -1)),
            0x23 => units!(unit!(Day ^ -1)),
            0x24 => units!(unit!(Week ^ -1)),
            0x25 => units!(unit!(Month ^ -1)),
            0x26 => units!(unit!(Year ^ -1)),
            0x27 => units!(unit!(Revolution ^ -1)),
            0x28 => VifInfo {
                units: &[unit!(Increment), unit!(InputPulseOnChannel0 ^ -1)],
                ..VifInfo::EMPTY
            },
            0x29 => VifInfo {
                units: &[unit!(Increment), unit!(InputPulseOnChannel1 ^ -1)],
                ..VifInfo::EMPTY
            },
            0x2A => VifInfo {
                units: &[unit!(Increment), unit!(OutputPulseOnChannel0 ^ -1)],
                ..VifInfo::EMPTY
            },
            0x2B => VifInfo {
                units: &[unit!(Increment), unit!(OutputPulseOnChannel1 ^ -1)],
                ..VifInfo::EMPTY
            },
            0x2C => units!(unit!(Liter)),
            0x2D => units!(unit!(Meter ^ -3)),
            0x2E => units!(unit!(Kilogram ^ -1)),
            0x2F => units!(unit!(Kelvin ^ -1)),
            0x30 => VifInfo {
                units: &[unit!(Watt ^ -1), unit!(Hour ^ -1)],
                scale: -(3),
                ..VifInfo::EMPTY
            },
            0x31 => VifInfo {
                units: &[unit!(Joul ^ -1)],
                scale: -9,
                ..VifInfo::EMPTY
            },
            0x32 => VifInfo {
                units: &[unit!(Watt ^ -1)],
                scale: -3,
                ..VifInfo::EMPTY
            },
            0x33 => VifInfo {
                units: &[unit!(Kelvin ^ -1), unit!(Liter ^ -1)],
                ..VifInfo::EMPTY
            },
            0x34 => units!(unit!(Volt ^ -1)),
            0x35 => units!(unit!(Ampere ^ -1)),
            0x36 => units!(unit!(Second ^ 1)),
            0x37 => VifInfo {
                units: &[unit!(Second ^ 1), unit!(Volt ^ -1)],
                ..VifInfo::EMPTY
            },
            0x38 => VifInfo {
                units: &[unit!(Second ^ 1), unit!(Ampere ^ -1)],
                ..VifInfo::EMPTY
            },
            0x39 => labels!(ValueLabel::StartDateOf),
            0x3A => labels!(ValueLabel::VifContainsUncorrectedUnitOrValue),
            0x3B => labels!(ValueLabel::AccumulationOnlyIfValueIsPositive),
            0x3C => labels!(ValueLabel::AccumulationOnlyIfValueIsNegative),
            0x3D => labels!(ValueLabel::NonMetricUnits),
            0x3E => labels!(ValueLabel::ValueAtBaseConditions),
            0x3F => labels!(ValueLabel::ObisDeclaration),
            // E100 u000 where u = 0: Lower; u = 1: Upper
            0x40 => labels!(ValueLabel::LowerLimitValue),
            0x48 => labels!(ValueLabel::UpperLimitValue),
            // E100 u001 where u = 0: Lower; u = 1: Upper
            0x41 => labels!(ValueLabel::NumberOfExceedsOfLowerLimitValue),
            0x49 => labels!(ValueLabel::NumberOfExceedsOfUpperLimitValue),
            /* E100 uf1b where
            b = 0: Begin; b = 1: End
            f = 0: First; b = 1: Last
            u = 0: Lower; u = 1: Upper
            */
            0x42 => labels!(ValueLabel::DateOfBeginFirstLowerLimitExceed),
            0x43 => labels!(ValueLabel::DateOfEndFirstLowerLimitExceed),
            0x46 => labels!(ValueLabel::DateOfBeginLastLowerLimitExceed),
            0x47 => labels!(ValueLabel::DateOfEndLastLowerLimitExceed),
            0x4A => labels!(ValueLabel::DateOfBeginFirstUpperLimitExceed),
            0x4B => labels!(ValueLabel::DateOfEndFirstUpperLimitExceed),
            0x4E => labels!(ValueLabel::DateOfBeginLastUpperLimitExceed),
            0x4F => labels!(ValueLabel::DateOfEndLastUpperLimitExceed),
            0x50 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstLowerLimitExceed],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x51 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstLowerLimitExceed],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x52 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstLowerLimitExceed],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x53 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstLowerLimitExceed],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x54 => VifInfo {
                labels: &[ValueLabel::DurationOfLastLowerLimitExceed],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x55 => VifInfo {
                labels: &[ValueLabel::DurationOfLastLowerLimitExceed],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x56 => VifInfo {
                labels: &[ValueLabel::DurationOfLastLowerLimitExceed],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x57 => VifInfo {
                labels: &[ValueLabel::DurationOfLastLowerLimitExceed],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x58 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstUpperLimitExceed],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x59 => VifInfo {
                labels: &[ValueLabel::DurationOfFirstUpperLimitExceed],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x5A => VifInfo {
                labels: &[ValueLabel::DurationOfFirstUpperLimitExceed],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x5B => VifInfo {
                labels: &[ValueLabel::DurationOfFirstUpperLimitExceed],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x5C => VifInfo {
                labels: &[ValueLabel::DurationOfLastUpperLimitExceed],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x5D => VifInfo {
                labels: &[ValueLabel::DurationOfLastUpperLimitExceed],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x5E => VifInfo {
                labels: &[ValueLabel::DurationOfLastUpperLimitExceed],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x5F => VifInfo {
                labels: &[ValueLabel::DurationOfLastUpperLimitExceed],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x60 => VifInfo {
                labels: &[ValueLabel::DurationOfFirst],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x61 => VifInfo {
                labels: &[ValueLabel::DurationOfFirst],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x62 => VifInfo {
                labels: &[ValueLabel::DurationOfFirst],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x63 => VifInfo {
                labels: &[ValueLabel::DurationOfFirst],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x64 => VifInfo {
                labels: &[ValueLabel::DurationOfLast],
                units: &[unit!(Second)],
                ..VifInfo::EMPTY
            },
            0x65 => VifInfo {
                labels: &[ValueLabel::DurationOfLast],
                units: &[unit!(Minute)],
                ..VifInfo::EMPTY
            },
            0x66 => VifInfo {
                labels: &[ValueLabel::DurationOfLast],
                units: &[unit!(Hour)],
                ..VifInfo::EMPTY
            },
            0x67 => VifInfo {
                labels: &[ValueLabel::DurationOfLast],
                units: &[unit!(Day)],
                ..VifInfo::EMPTY
            },
            0x68 => labels!(ValueLabel::ValueDuringLowerValueExceed),
            0x6C => labels!(ValueLabel::ValueDuringUpperValueExceed),
            0x69 => labels!(ValueLabel::LeakageValues),
            0x6D => labels!(ValueLabel::OverflowValues),
            0x6A => labels!(ValueLabel::DateOfBeginFirst),
            0x6B => labels!(ValueLabel::DateOfBeginLast),
            0x6E => labels!(ValueLabel::DateOfEndLast),
            0x6F => labels!(ValueLabel::DateOfEndFirst),
            0x70..=0x77 => VifInfo {
                scale: (data & 0b111) as isize - 6,
                ..VifInfo::EMPTY
            },
            0x78..=0x7B => VifInfo {
                offset: (data & 0b11) as isize - 3,
                ..VifInfo::EMPTY
            },
            0x7D => VifInfo {
                scale: 3,
                ..VifInfo::EMPTY
            },
            0x7E => labels!(ValueLabel::FutureValue),
            0x7F => labels!(ValueLabel::NextVIFEAndDataOfThisBlockAreManufacturerSpecific),
            _ => labels!(ValueLabel::Reserved),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ValueInformationError {
    InvalidValueInformation,
    DataTooShort,
}

impl From<u8> for ValueInformationField {
    fn from(data: u8) -> Self {
        Self { data }
    }
}
/// Selects the orthogonal part of a VIFE chain.
///
/// Primary and PlainText start at VIFE[0]; MainVIFExtension and
/// AlternateVIFExtension start at VIFE[1]; ManufacturerSpecific has no chain.
/// Main extension 0x7D deliberately reuses VIFE[1] as both its sub-code and
/// the first orthogonal VIFE, preserving the original decoder's behavior.
fn orthogonal_chain(
    coding: ValueInformationCoding,
    ext: Option<ValueInformationFieldExtensions<'_>>,
) -> ValueInformationFieldExtensions<'_> {
    let mut chain = ext.unwrap_or(ValueInformationFieldExtensions(&[]));
    match coding {
        ValueInformationCoding::MainVIFExtension
        | ValueInformationCoding::AlternateVIFExtension => {
            chain.next();
        }
        ValueInformationCoding::ManufacturerSpecific => return ValueInformationFieldExtensions(&[]),
        _ => {}
    }
    chain
}

#[derive(Clone)]
struct OrthogonalVifes<'a> {
    vife: ValueInformationFieldExtensions<'a>,
    combinable_ext: bool,
}
impl Iterator for OrthogonalVifes<'_> {
    type Item = VifInfo;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let v = self.vife.next()?;
            // Whole-byte, unconditional comparison: repeated 0xFC prefixes keep
            // the extension table selected; a terminal 0x7C is ordinary data.
            if v.data == 0xFC {
                self.combinable_ext = true;
                continue;
            }
            let ext = core::mem::replace(&mut self.combinable_ext, false);
            if !ext && v.data & 0x7F == 0x7F {
                // Everything following the manufacturer escape is vendor data,
                // so it must not be matched against the standard VIFE table.
                self.vife = ValueInformationFieldExtensions(&[]);
            }
            return Some(orthogonal_vife_info(v.data, ext));
        }
    }
}

/// Substitutes the metric units of a VIF followed by VIFE 0x3D, per
/// EN 13757-3 Annex C Table C.1. Returns the non-metric units and the change
/// to the decimal exponent; kBTU and mBTU/s are expressed as BTU and BTU/s.
fn non_metric_units(
    coding: ValueInformationCoding,
    vif: u8,
    first_vife: Option<u8>,
) -> Option<(&'static [Unit], isize)> {
    match coding {
        ValueInformationCoding::Primary => match vif & 0x7F {
            // 10^(nnn-3) Wh -> 10^(nnn-3) kBTU
            0x00..=0x07 => Some((&[unit!(BritishThermalUnit)], 3)),
            // 10^(nnn-6) m³ -> 10^(nnn-3) USgal
            0x10..=0x17 => Some((&[unit!(AmericanGallon)], 3)),
            // 10^(nnn-3) W -> 10^(nnn-3) mBTU/s
            0x28..=0x2F => Some((&[unit!(BritishThermalUnit), unit!(Second ^ -1)], -3)),
            // 10^(nnn-7) m³/min -> 10^(nnn-4) USgal/min
            0x40..=0x47 => Some((&[unit!(AmericanGallon), unit!(Minute ^ -1)], 3)),
            // Flow, return, difference and external temperature: °C/K -> °F
            0x58..=0x67 => Some((&[unit!(Fahrenheit)], 0)),
            _ => None,
        },
        // Cold/warm temperature limit: °C -> °F
        ValueInformationCoding::AlternateVIFExtension => match first_vife? & 0x7F {
            0x74..=0x77 => Some((&[unit!(Fahrenheit)], 0)),
            _ => None,
        },
        _ => None,
    }
}

/// A borrowed, allocation-free view of decoded VIF and VIFE information.
///
/// Labels and units are produced in wire order, including duplicates. The view
/// borrows frame bytes, independently of the block used to construct it.
#[derive(Clone)]
pub struct ValueInformation<'a> {
    head_labels: &'static [ValueLabel],
    head_units: &'static [Unit],
    orthogonal: ValueInformationFieldExtensions<'a>,
    pub decimal_scale_exponent: isize,
    pub decimal_offset_exponent: isize,
}
impl<'a> ValueInformation<'a> {
    /// Iterates over all decoded labels in wire order.
    #[must_use]
    pub fn labels(&self) -> ValueLabels<'a> {
        ValueLabels {
            current: self.head_labels,
            rest: OrthogonalVifes {
                vife: self.orthogonal.clone(),
                combinable_ext: false,
            },
        }
    }
    /// Iterates over all decoded units in wire order.
    #[must_use]
    pub fn units(&self) -> Units<'a> {
        Units {
            current: self.head_units,
            rest: OrthogonalVifes {
                vife: self.orthogonal.clone(),
                combinable_ext: false,
            },
        }
    }
    #[must_use]
    pub fn has_label(&self, label: ValueLabel) -> bool {
        self.labels().any(|item| item == label)
    }
    #[must_use]
    pub fn first_unit(&self) -> Option<Unit> {
        self.units().next()
    }
}
impl<'a> TryFrom<&ValueInformationBlock<'a>> for ValueInformation<'a> {
    type Error = DataInformationError;
    fn try_from(block: &ValueInformationBlock<'a>) -> Result<Self, Self::Error> {
        let coding = ValueInformationCoding::from(&block.value_information);
        let ext = block.value_information_extension.clone();
        // Peek at the remaining borrowed bytes directly. Flattening an optional
        // iterator adds state transitions even for the common no-extension case.
        let bytes = ext.as_ref().map_or(&[][..], |ext| ext.0);
        let first = bytes.first().copied();
        let second = bytes.get(1).copied();
        // A present but exhausted extension iterator was an error in the old
        // decoder; absent extensions on manually constructed blocks were empty.
        if matches!(
            coding,
            ValueInformationCoding::MainVIFExtension
                | ValueInformationCoding::AlternateVIFExtension
        ) && ext.is_some()
            && first.is_none()
        {
            return Err(DataInformationError::DataTooShort);
        }
        let head = head_vif_info(block.value_information.clone(), first, second)?;
        let orthogonal = OrthogonalVifes {
            vife: orthogonal_chain(coding, ext),
            combinable_ext: false,
        };
        let (scale, offset, non_metric) =
            orthogonal
                .clone()
                .fold((head.scale, head.offset, false), |(s, o, n), v| {
                    (
                        s + v.scale,
                        o + v.offset,
                        n || v.labels == [ValueLabel::NonMetricUnits],
                    )
                });
        let (head_units, scale) = match non_metric
            .then(|| non_metric_units(coding, block.value_information.data, first))
            .flatten()
        {
            Some((units, delta)) => (units, scale + delta),
            None => (head.units, scale),
        };
        Ok(Self {
            head_labels: head.labels,
            head_units,
            orthogonal: orthogonal.vife,
            decimal_scale_exponent: scale,
            decimal_offset_exponent: offset,
        })
    }
}
impl PartialEq for ValueInformation<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.decimal_scale_exponent == other.decimal_scale_exponent
            && self.decimal_offset_exponent == other.decimal_offset_exponent
            && self.labels().eq(other.labels())
            && self.units().eq(other.units())
    }
}
impl core::fmt::Debug for ValueInformation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ValueInformation")
            .field("decimal_offset_exponent", &self.decimal_offset_exponent)
            .field("labels", &self.labels())
            .field("decimal_scale_exponent", &self.decimal_scale_exponent)
            .field("units", &self.units())
            .finish()
    }
}
#[cfg(feature = "serde")]
impl serde::Serialize for ValueInformation<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ValueInformation", 4)?;
        state.serialize_field("decimal_offset_exponent", &self.decimal_offset_exponent)?;
        state.serialize_field("labels", &self.labels())?;
        state.serialize_field("decimal_scale_exponent", &self.decimal_scale_exponent)?;
        state.serialize_field("units", &self.units())?;
        state.end()
    }
}

// Binary serializers need a known length. Count a clone so serialization stays
// allocation-free and leaves the caller's iterator position unchanged.
#[cfg(feature = "serde")]
fn serialize_counted_sequence<I, S>(items: I, serializer: S) -> Result<S::Ok, S::Error>
where
    I: Iterator + Clone,
    I::Item: serde::Serialize,
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut sequence = serializer.serialize_seq(Some(items.clone().count()))?;
    for item in items {
        sequence.serialize_element(&item)?;
    }
    sequence.end()
}

/// Cloneable iterator over decoded labels in wire order.
#[derive(Clone)]
pub struct ValueLabels<'a> {
    current: &'static [ValueLabel],
    rest: OrthogonalVifes<'a>,
}
impl Iterator for ValueLabels<'_> {
    type Item = ValueLabel;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((first, tail)) = self.current.split_first() {
                self.current = tail;
                return Some(*first);
            }
            self.current = self.rest.next()?.labels;
        }
    }
}
impl core::fmt::Debug for ValueLabels<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
#[cfg(feature = "serde")]
impl serde::Serialize for ValueLabels<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_counted_sequence(self.clone(), serializer)
    }
}

/// Cloneable iterator over decoded units in wire order.
#[derive(Clone)]
pub struct Units<'a> {
    current: &'static [Unit],
    rest: OrthogonalVifes<'a>,
}
impl Iterator for Units<'_> {
    type Item = Unit;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((first, tail)) = self.current.split_first() {
                self.current = tail;
                return Some(*first);
            }
            self.current = self.rest.next()?.units;
        }
    }
}
impl core::fmt::Debug for Units<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
#[cfg(feature = "serde")]
impl serde::Serialize for Units<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_counted_sequence(self.clone(), serializer)
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for ValueInformation<'_> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "ValueInformation{{ decimal_offset_exponent: {}, decimal_scale_exponent: {}",
            self.decimal_offset_exponent,
            self.decimal_scale_exponent
        );
        let mut labels = self.labels().peekable();
        if labels.peek().is_some() {
            defmt::write!(f, ", labels: [");
            for (i, label) in labels.enumerate() {
                if i != 0 {
                    defmt::write!(f, ", ");
                }
                defmt::write!(f, "{:?}", label);
            }
            defmt::write!(f, "]");
        }
        let mut units = self.units().peekable();
        if units.peek().is_some() {
            defmt::write!(f, ", units: [");
            for (i, unit) in units.enumerate() {
                if i != 0 {
                    defmt::write!(f, ", ");
                }
                defmt::write!(f, "{:?}", unit);
            }
            defmt::write!(f, "]");
        }
        defmt::write!(f, " }}");
    }
}

#[cfg(feature = "std")]
impl fmt::Display for ValueInformation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.decimal_offset_exponent != 0 {
            write!(f, "+{})", self.decimal_offset_exponent)?;
        } else {
            write!(f, ")")?;
        }
        if self.decimal_scale_exponent != 0 {
            write!(f, "e{}", self.decimal_scale_exponent)?;
        }
        let mut units = self.units().peekable();
        if units.peek().is_some() {
            write!(f, "[")?;
            for unit in units {
                write!(f, "{}", unit)?;
            }
            write!(f, "]")?;
        }
        let mut labels = self.labels().peekable();
        if labels.peek().is_some() {
            write!(f, "(")?;
            for (i, label) in labels.enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:?}", label)?;
            }

            return write!(f, ")");
        }
        Ok(())
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ValueLabel {
    Instantaneous,
    ReservedForObjectActions,
    Reserved,
    Averaged,
    Integral,
    Parameter,
    InverseCompactProfile,
    RelativeDeviation,
    RecordErrorCodes,
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
    VifContainsUncorrectedUnitOrValue,
    AccumulationOnlyIfValueIsPositive,
    AccumulationOnlyIfValueIsNegative,
    NonMetricUnits,
    AlternativeNonMetricUnits,
    ValueAtBaseConditions,
    ObisDeclaration,
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
    ValueDuringLowerValueExceed,
    ValueDuringUpperValueExceed,
    LeakageValues,
    OverflowValues,
    DateOfBeginLast,
    DateOfBeginFirst,
    DateOfEndLast,
    DateOfEndFirst,
    ExtensionOfCombinableOrthogonalVIFE,
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
    DescriptionOfTariffAndSubunit,
    StorageInterval,
    Dimensionless,
    DimensionlessHCA,
    DataContainerForWmbusProtocol,
    PeriodOfNormalDataTransmission,
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
    ReactiveEnergy,
    ApparentEnergy,
    CoefficientOfPerformance,
    ReactivePower,
    Frequency,
    ApparentPower,
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
    SecondarySensorMeasurement,
    HigherResolutionRegister,
    DataPresentedWithTypeC,
    DataPresentedWithTypeD,
    EndDate,
    DirectionFromCommunicationPartnerToMeter,
    DirectionFromMeterToCommunicationPartner,
    RelativeHumidity,
    MoistureLevel,
    PhaseUtoU,
    PhaseUtoI,
    PhaseItoU,
    ColdWarmTemperatureLimit,
    CumulativeMaximumOfActivePower,
    ResultingRatingFactor,
    ThermalOutputRatingFactor,
    ThermalCouplingRatingFactorOverall,
    ThermalCouplingRatingRoomSide,
    ThermalCouplingRatingFactorHeatingSide,
    LowTemperatureRatingFactor,
    DisplayOutputScalingFactor,
    ManufacturerSpecific,
    OnTime,
    OperatingTime,
    Volume,
    Mass,
    Power,
    VolumeFlow,
    MassFlow,
    Pressure,
    Voltage,
    Current,
    FlowTemperature,
    ReturnTemperature,
    TemperatureDifference,
    ExternalTemperature,
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
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
    Fahrenheit,
    AmericanGallon,
    Calorie,
    BritishThermalUnit,
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
            UnitName::Volt => write!(f, "V"),
            UnitName::Ampere => write!(f, "A"),
            UnitName::LocalMoneyCurrency => write!(f, "$ (local)"),
            UnitName::Symbol => write!(f, "Symbol"),
            UnitName::BitTime => write!(f, "BitTime"),
            UnitName::DecibelMilliWatt => write!(f, "dBmW"),
            UnitName::Percent => write!(f, "%"),
            UnitName::Degree => write!(f, "°"),
            UnitName::Hertz => write!(f, "Hz"),
            UnitName::HCAUnit => write!(f, "HCAUnit"),
            UnitName::Fahrenheit => write!(f, "°F"),
            UnitName::AmericanGallon => write!(f, "UsGal"),
            UnitName::BritishThermalUnit => write!(f, "BTU"),
            UnitName::Calorie => write!(f, "cal"),
        }
    }
}

#[cfg(test)]
mod tests {
    fn assert_information(
        actual: super::ValueInformation<'_>,
        offset: isize,
        scale: isize,
        labels: &[super::ValueLabel],
        units: &[super::Unit],
    ) {
        assert_eq!(actual.decimal_offset_exponent, offset);
        assert_eq!(actual.decimal_scale_exponent, scale);
        assert!(actual.labels().eq(labels.iter().copied()));
        assert!(actual.units().eq(units.iter().copied()));
    }

    #[test]
    fn value_information_peeks_remaining_extension_bytes_without_consuming_them() {
        use super::{
            ValueInformation, ValueInformationBlock, ValueInformationFieldExtensions, ValueLabel,
        };
        let bytes = [0x80, 0xfd, 0x3e];
        let mut extensions = ValueInformationFieldExtensions::new(&bytes).unwrap();
        assert_eq!(extensions.next().unwrap().data, 0x80);
        let block = ValueInformationBlock::new(0xfd.into(), Some(extensions), None);
        let info = ValueInformation::try_from(&block).unwrap();
        assert!(info.has_label(ValueLabel::MoistureLevel));
        assert_eq!(
            info,
            ValueInformation::try_from(
                &ValueInformationBlock::try_from([0xfd, 0xfd, 0x3e].as_slice()).unwrap()
            )
            .unwrap()
        );
        assert_eq!(block.value_information_extension.as_ref().unwrap().len(), 2);
        assert_eq!(ValueInformation::try_from(&block).unwrap(), info);
    }

    #[test]
    fn test_single_byte_primary_value_information_parsing() {
        use crate::value_information::UnitName;
        use crate::value_information::{
            Unit, ValueInformation, ValueInformationBlock, ValueInformationField, ValueLabel,
        };

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
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            -3,
            &[ValueLabel::Volume],
            &[unit!(Meter ^ 3)],
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
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            -2,
            &[ValueLabel::Volume],
            &[unit!(Meter ^ 3)],
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
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            -1,
            &[ValueLabel::Volume],
            &[unit!(Meter ^ 3)],
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
        use crate::value_information::UnitName;
        use crate::value_information::{
            Unit, ValueInformation, ValueInformationBlock, ValueInformationField, ValueLabel,
        };

        /* 1 VIF, 1 - 10 orthogonal VIFE */

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x12 => Combinable Orthogonal VIFE meaning "averaged" */
        /* VIB = 0x96, 0x12 */
        let data = [0x96, 0x12];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);
        assert_eq!(result.value_information, ValueInformationField::from(0x96));
        assert!(ValueInformation::try_from(&result)
            .unwrap()
            .labels()
            .eq([ValueLabel::Volume, ValueLabel::Averaged]));

        /* VIF 0x96 = 0x16 | 0x80  => m3^-3*1e-1 with extension*/
        /* VIFE 0x92 = 0x12 | 0x80  => Combinable Orthogonal VIFE meaning "averaged" with extension */
        /* VIFE 0x20 => Combinable Orthogonal VIFE meaning "per second" */
        /* VIB = 0x96, 0x92,0x20 */

        let data = [0x96, 0x92, 0x20];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 3);
        assert_eq!(result.value_information, ValueInformationField::from(0x96));
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            0,
            &[ValueLabel::Volume, ValueLabel::Averaged],
            &[unit!(Meter ^ 3), unit!(Second ^ -1)],
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
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            0,
            &[ValueLabel::Volume, ValueLabel::Averaged],
            &[unit!(Meter ^ 3), unit!(Second ^ -1), unit!(Meter ^ -3)],
        );
    }

    #[cfg(not(feature = "plaintext-before-extension"))]
    #[test]
    fn test_plain_text_vif_norm_conform() {
        use crate::value_information::{ValueInformation, ValueLabel};

        use crate::value_information::ValueInformationBlock;
        // This is the ascii conform method of encoding the VIF
        // VIF  VIFE  LEN(3) 'H'   'R'  '%'
        // 0xFC, 0x74, 0x03, 0x48, 0x52, 0x25,
        // %RH
        // Combinable (orthogonal) VIFE-Code extension table
        // VIFE = 0x74 => E111 0nnn Multiplicative correction factor for value (not unit): 10nnn–6 => 10^-2
        //
        // according to the Norm the LEN and ASCII is not part of the VIB however this makes parsing
        // cumbersome so we include it in the VIB

        let data = [0xFC, 0x74, 0x03, 0x48, 0x52, 0x25];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 6);
        assert_eq!(result.value_information.data, 0xFC);
        assert_information(
            ValueInformation::try_from(&result).unwrap(),
            0,
            -2,
            &[ValueLabel::PlainText],
            &[],
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
        use crate::value_information::ValueInformationBlock;
        let data = [253, 27];
        let result = ValueInformationBlock::try_from(data.as_slice()).unwrap();
        assert_eq!(result.get_size(), 2);
    }

    #[test]
    fn test_vif_fd_voltage_and_ampere() {
        use crate::value_information::UnitName;
        use crate::value_information::{ValueInformation, ValueInformationBlock};

        // VIF=0xFD VIFE=0x48: Voltage 10^(8-9) = 0.1 V
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFD, 0x48].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Volt);
        assert_eq!(vi.decimal_scale_exponent, -1);

        // VIF=0xFD VIFE=0x59: Ampere 10^(9-12) = 0.001 A
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFD, 0x59].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Ampere);
        assert_eq!(vi.decimal_scale_exponent, -3);
    }

    #[test]
    fn test_vif_fb_added_codes_and_reserved_fallback() {
        use crate::value_information::UnitName;
        use crate::value_information::{ValueInformation, ValueInformationBlock, ValueLabel};

        // VIF=0xFB VIFE=0x20 (E010 0000): ft³, dec: 0
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x20].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Feet);
        assert_eq!(vi.first_unit().unwrap().exponent, 3);
        assert_eq!(vi.decimal_scale_exponent, 0);
        assert!(vi.has_label(ValueLabel::Volume));

        // VIF=0xFB VIFE=0x23 (E010 0011): Phase angle I-U, 0.1°
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x23].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Degree);
        assert_eq!(vi.decimal_scale_exponent, -1);
        assert!(vi.has_label(ValueLabel::PhaseItoU));

        // VIF=0xFB VIFE=0x70: °F cold/warm temp limit, 10^-3
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x70].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Fahrenheit);
        assert_eq!(vi.decimal_scale_exponent, -3);

        // VIF=0xFB VIFE=0x22 (E010 0010): Reserved — should not error
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x22].as_slice()).unwrap(),
        )
        .unwrap();
        assert!(vi.has_label(ValueLabel::Reserved));
    }

    #[test]
    fn test_primary_vif_on_time_and_operating_time_labels() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // VIF 0x21 = 0010 0001 = On time (0x20-0x23), nn=01 => minutes
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0x21].as_slice()).unwrap(),
        )
        .unwrap();
        assert!(vi.has_label(ValueLabel::OnTime));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Minute);

        // VIF 0x27 = 0010 0111 = Operating time (0x24-0x27), nn=11 => days
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0x27].as_slice()).unwrap(),
        )
        .unwrap();
        assert!(vi.has_label(ValueLabel::OperatingTime));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Day);
    }

    #[test]
    fn test_fb_cumulative_maximum_of_active_power() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // VIF=0xFB VIFE=0x78 (0b0111_1000): CumulativeMaximumOfActivePower, W 10^-3
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x78].as_slice()).unwrap(),
        )
        .unwrap();
        assert!(vi.has_label(ValueLabel::CumulativeMaximumOfActivePower));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Watt);
        assert_eq!(vi.decimal_scale_exponent, -3);
    }

    #[test]
    fn test_fb_humidity_with_combinatorial_scale() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // VIF=0xFB, VIFE=0x9B (0x1B + extension bit), VIFE2=0x74 (multiplicative 10^(4-6) = 10^-2)
        // OMS RH01: relative humidity 10^0 %, shifted to 10^-2 by combinatorial.
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFB, 0x9B, 0x74].as_slice()).unwrap(),
        )
        .unwrap();

        assert!(vi.has_label(ValueLabel::RelativeHumidity));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Percent);
        assert_eq!(vi.decimal_scale_exponent, -2);
    }

    #[test]
    fn test_fd_ampere_with_phase_combinatorial() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // VIF=0xFD, VIFE=0xD9 (0x59 + extension bit = Ampere 10^-3),
        // VIFE2=0xFC (combinatorial extension), VIFE3=0x01 (AtPhaseL1)
        // OMS CA01: per-phase current.
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFD, 0xD9, 0xFC, 0x01].as_slice()).unwrap(),
        )
        .unwrap();

        assert_eq!(vi.first_unit().unwrap().name, UnitName::Ampere);
        assert_eq!(vi.decimal_scale_exponent, -3);
        assert!(vi.has_label(ValueLabel::AtPhaseL1));
    }

    #[test]
    fn test_primary_vif_combinatorial_not_skipped() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // VIF=0xE5 (0x65 + extension bit = External temperature 10^-2),
        // VIFE=0x74 (multiplicative 10^(4-6) = 10^-2)
        // First VIFE must NOT be skipped for primary VIFs — total should be 10^-4.
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xE5, 0x74].as_slice()).unwrap(),
        )
        .unwrap();

        assert!(vi.has_label(ValueLabel::ExternalTemperature));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Celsius);
        assert_eq!(vi.decimal_scale_exponent, -4);
    }

    #[test]
    fn test_fd_moisture_level() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // OMS RH03: VIF=0xFD, VIFE1=0xFD (0x7D + extension bit), VIFE2=0x3E
        // Moisture Level, % 10^0
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0xFD, 0xFD, 0x3E].as_slice()).unwrap(),
        )
        .unwrap();

        assert!(vi.has_label(ValueLabel::MoistureLevel));
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Percent);
        assert_eq!(vi.decimal_scale_exponent, 0);
    }

    #[test]
    fn test_non_metric_vife_substitutes_units() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        let decode = |bytes: &[u8]| {
            let vi = ValueInformation::try_from(&ValueInformationBlock::try_from(bytes).unwrap())
                .unwrap();
            assert!(vi.has_label(ValueLabel::NonMetricUnits));
            let units: Vec<_> = vi.units().map(|u| (u.name, u.exponent)).collect();
            (units, vi.decimal_scale_exponent)
        };

        // EN 13757-3 Annex C Table C.1, VIFE 0x3D after each metric VIF.
        // Issue #62: VIF 0x93 is 10^-3 m³ = 1 l, which maps to 1 USgal.
        assert_eq!(
            decode(&[0x93, 0x3D]),
            (vec![(UnitName::AmericanGallon, 1)], 0)
        );
        // 10^-3 Wh -> 10^-3 kBTU = 1 BTU
        assert_eq!(
            decode(&[0x80, 0x3D]),
            (vec![(UnitName::BritishThermalUnit, 1)], 0)
        );
        // 10^-3 W -> 10^-3 mBTU/s = 10^-6 BTU/s
        assert_eq!(
            decode(&[0xA8, 0x3D]),
            (
                vec![(UnitName::BritishThermalUnit, 1), (UnitName::Second, -1)],
                -6
            )
        );
        // 10^-7 m³/min -> 10^-4 USgal/min
        assert_eq!(
            decode(&[0xC0, 0x3D]),
            (
                vec![(UnitName::AmericanGallon, 1), (UnitName::Minute, -1)],
                -4
            )
        );
        // Flow, return, difference and external temperature: 10^-3 °F
        for vif in [0xD8, 0xDC, 0xE0, 0xE4] {
            assert_eq!(decode(&[vif, 0x3D]), (vec![(UnitName::Fahrenheit, 1)], -3));
        }
        // Cold/warm temperature limit via 0xFB 0x74: 10^-3 °F
        assert_eq!(
            decode(&[0xFB, 0xF4, 0x3D]),
            (vec![(UnitName::Fahrenheit, 1)], -3)
        );

        // Without the VIFE the unit stays metric.
        let vi = ValueInformation::try_from(
            &ValueInformationBlock::try_from([0x13].as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(vi.first_unit().unwrap().name, UnitName::Meter);
        assert_eq!(vi.decimal_scale_exponent, -3);
    }

    #[test]
    fn test_vib_struct_layout() {
        use crate::value_information::ValueInformationBlock;

        // FD extension with orthogonal VIFEs: Ampere 10^-3, AtPhaseL1
        // VIF=0xFD, VIFE[0]=0xD9 (true VIF), VIFE[1]=0xFC, VIFE[2]=0x01
        let vib = ValueInformationBlock::try_from([0xFD, 0xD9, 0xFC, 0x01].as_slice()).unwrap();

        assert_eq!(vib.value_information.data, 0xFD);
        assert_eq!(vib.get_size(), 4);
        assert!(vib.plaintext_vife.is_none());

        let mut ext = vib.value_information_extension.unwrap();
        assert_eq!(ext.len(), 3);
        assert_eq!(ext.next().unwrap().data, 0xD9);
        assert_eq!(ext.next().unwrap().data, 0xFC);
        assert_eq!(ext.next().unwrap().data, 0x01);

        // Primary VIF with one orthogonal VIFE
        // VIF=0x96 (Volume + extension bit), VIFE=0x12 (Averaged)
        let vib = ValueInformationBlock::try_from([0x96, 0x12].as_slice()).unwrap();

        assert_eq!(vib.value_information.data, 0x96);
        assert_eq!(vib.get_size(), 2);

        let mut ext = vib.value_information_extension.unwrap();
        assert_eq!(ext.len(), 1);
        assert_eq!(ext.next().unwrap().data, 0x12);

        // Single primary VIF, no extension
        let vib = ValueInformationBlock::try_from([0x13].as_slice()).unwrap();

        assert_eq!(vib.value_information.data, 0x13);
        assert_eq!(vib.get_size(), 1);
        assert!(vib.value_information_extension.is_none());
    }

    #[test]
    fn test_combinable_orthogonal_vife_limit_exceed_mappings() {
        use crate::value_information::{
            UnitName, ValueInformation, ValueInformationBlock, ValueLabel,
        };

        // (vife_byte, expected_label, optional expected unit name)
        let cases: &[(u8, ValueLabel, Option<UnitName>)] = &[
            (0x40, ValueLabel::LowerLimitValue, None),
            (0x48, ValueLabel::UpperLimitValue, None),
            (0x41, ValueLabel::NumberOfExceedsOfLowerLimitValue, None),
            (0x49, ValueLabel::NumberOfExceedsOfUpperLimitValue, None),
            // E100 uf1b: b=Begin/End, f=First/Last, u=Lower/Upper
            (0x42, ValueLabel::DateOfBeginFirstLowerLimitExceed, None),
            (0x43, ValueLabel::DateOfEndFirstLowerLimitExceed, None),
            (0x46, ValueLabel::DateOfBeginLastLowerLimitExceed, None),
            (0x47, ValueLabel::DateOfEndLastLowerLimitExceed, None),
            (0x4A, ValueLabel::DateOfBeginFirstUpperLimitExceed, None),
            (0x4B, ValueLabel::DateOfEndFirstUpperLimitExceed, None),
            (0x4E, ValueLabel::DateOfBeginLastUpperLimitExceed, None),
            (0x4F, ValueLabel::DateOfEndLastUpperLimitExceed, None),
            // Duration of first lower (0x50-0x53)
            (
                0x50,
                ValueLabel::DurationOfFirstLowerLimitExceed,
                Some(UnitName::Second),
            ),
            (
                0x51,
                ValueLabel::DurationOfFirstLowerLimitExceed,
                Some(UnitName::Minute),
            ),
            (
                0x52,
                ValueLabel::DurationOfFirstLowerLimitExceed,
                Some(UnitName::Hour),
            ),
            (
                0x53,
                ValueLabel::DurationOfFirstLowerLimitExceed,
                Some(UnitName::Day),
            ),
            // Duration of last lower (0x54-0x57)
            (
                0x54,
                ValueLabel::DurationOfLastLowerLimitExceed,
                Some(UnitName::Second),
            ),
            (
                0x55,
                ValueLabel::DurationOfLastLowerLimitExceed,
                Some(UnitName::Minute),
            ),
            (
                0x56,
                ValueLabel::DurationOfLastLowerLimitExceed,
                Some(UnitName::Hour),
            ),
            (
                0x57,
                ValueLabel::DurationOfLastLowerLimitExceed,
                Some(UnitName::Day),
            ),
            // Duration of first upper (0x58-0x5B)
            (
                0x58,
                ValueLabel::DurationOfFirstUpperLimitExceed,
                Some(UnitName::Second),
            ),
            (
                0x59,
                ValueLabel::DurationOfFirstUpperLimitExceed,
                Some(UnitName::Minute),
            ),
            (
                0x5A,
                ValueLabel::DurationOfFirstUpperLimitExceed,
                Some(UnitName::Hour),
            ),
            (
                0x5B,
                ValueLabel::DurationOfFirstUpperLimitExceed,
                Some(UnitName::Day),
            ),
            // Duration of last upper (0x5C-0x5F)
            (
                0x5C,
                ValueLabel::DurationOfLastUpperLimitExceed,
                Some(UnitName::Second),
            ),
            (
                0x5D,
                ValueLabel::DurationOfLastUpperLimitExceed,
                Some(UnitName::Minute),
            ),
            (
                0x5E,
                ValueLabel::DurationOfLastUpperLimitExceed,
                Some(UnitName::Hour),
            ),
            (
                0x5F,
                ValueLabel::DurationOfLastUpperLimitExceed,
                Some(UnitName::Day),
            ),
        ];

        for (vife_byte, expected_label, expected_unit) in cases {
            let data = [0x93, *vife_byte];
            let vib = ValueInformationBlock::try_from(data.as_slice()).unwrap();
            let vi = ValueInformation::try_from(&vib).unwrap();
            assert!(
                vi.has_label(*expected_label),
                "VIFE 0x{vife_byte:02X}: expected label {expected_label:?}, got {:?}",
                vi.labels()
            );
            if let Some(unit_name) = expected_unit {
                assert!(
                    vi.units().any(|u| u.name == *unit_name),
                    "VIFE 0x{vife_byte:02X}: expected unit {unit_name:?}, got {:?}",
                    vi.units()
                );
            }
        }
    }

    #[test]
    fn test_combinable_orthogonal_vife_fc_extension_mappings() {
        use crate::value_information::{ValueInformation, ValueInformationBlock, ValueLabel};

        let cases: &[(u8, ValueLabel)] = &[
            (0x02, ValueLabel::AtPhaseL2),
            (0x0D, ValueLabel::AlternativeNonMetricUnits),
            (0x0E, ValueLabel::SecondarySensorMeasurement),
            (0x13, ValueLabel::EndDate),
        ];

        for (vife_byte, expected_label) in cases {
            let data = [0x93, 0xFC, *vife_byte];
            let vib = ValueInformationBlock::try_from(data.as_slice()).unwrap();
            let vi = ValueInformation::try_from(&vib).unwrap();
            assert!(
                vi.has_label(*expected_label),
                "FC VIFE 0x{vife_byte:02X}: expected {expected_label:?}, got {:?}",
                vi.labels()
            );
        }
    }
    #[test]
    fn units_exceed_old_capacity() {
        use super::*;
        let block =
            ValueInformationBlock::try_from([0xB8, 0xA8, 0xA8, 0xA8, 0xA8, 0x28].as_slice())
                .unwrap();
        let vi = ValueInformation::try_from(&block).unwrap();
        assert!(vi
            .units()
            .eq([unit!(Meter ^ 3), unit!(Hour ^ -1)].into_iter().chain(
                core::iter::repeat_n([unit!(Increment), unit!(InputPulseOnChannel0 ^ -1)], 5)
                    .flatten()
            )));
        assert_eq!(vi.units().count(), 12);
    }

    #[test]
    fn labels_exceed_old_capacity() {
        use super::*;
        let block = ValueInformationBlock::try_from(
            [
                0xFD, 0x9A, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x12,
            ]
            .as_slice(),
        )
        .unwrap();
        let vi = ValueInformation::try_from(&block).unwrap();
        assert!(vi
            .labels()
            .eq([ValueLabel::DigitalOutput, ValueLabel::Binary]
                .into_iter()
                .chain(core::iter::repeat_n(ValueLabel::Averaged, 9))));
        assert_eq!(vi.labels().count(), 11);
    }

    #[test]
    fn repeated_fc_and_terminal_7c_preserve_table_selection() {
        use super::*;
        for (bytes, expected) in [
            (
                &[0x93, 0xFC, 0xFC, 0x01][..],
                &[ValueLabel::Volume, ValueLabel::AtPhaseL1][..],
            ),
            (
                &[0x93, 0x7C][..],
                &[ValueLabel::Volume, ValueLabel::Reserved][..],
            ),
            (
                &[0x93, 0xFC, 0x81, 0x12][..],
                &[
                    ValueLabel::Volume,
                    ValueLabel::AtPhaseL1,
                    ValueLabel::Averaged,
                ][..],
            ),
        ] {
            let block = ValueInformationBlock::try_from(bytes).unwrap();
            assert!(ValueInformation::try_from(&block)
                .unwrap()
                .labels()
                .eq(expected.iter().copied()));
        }
    }

    #[test]
    fn main_extension_subcode_is_also_orthogonal() {
        use super::*;
        let block = ValueInformationBlock::try_from([0xFD, 0xFD, 0x3E].as_slice()).unwrap();
        let vi = ValueInformation::try_from(&block).unwrap();
        assert!(vi
            .labels()
            .eq([ValueLabel::MoistureLevel, ValueLabel::ValueAtBaseConditions]));
        assert!(vi.units().eq([unit!(Percent)]));
    }

    #[test]
    fn equality_is_semantic_and_iterators_outlive_the_view() {
        use super::*;
        let short = ValueInformationBlock::try_from([0x13].as_slice()).unwrap();
        // 0x76 contributes zero scale and no labels or units.
        let equivalent = ValueInformationBlock::try_from([0x93, 0x76].as_slice()).unwrap();
        assert_eq!(
            ValueInformation::try_from(&short).unwrap(),
            ValueInformation::try_from(&equivalent).unwrap()
        );
        let (mut labels, mut units) = {
            let vi = ValueInformation::try_from(&short).unwrap();
            (vi.labels(), vi.units())
        };
        assert_eq!(labels.next(), Some(ValueLabel::Volume));
        assert_eq!(units.next(), Some(unit!(Meter ^ 3)));
        assert!(labels.clone().eq(labels));
        assert!(units.clone().eq(units));
    }

    #[test]
    fn manufacturer_escape_stops_standard_vife_decoding() {
        use super::*;
        // ABB B21 energy record: VIF 0x04 (energy, x10^1) followed by the
        // manufacturer escape 0xFF; 0xF2 and 0x00 are vendor data and must not
        // be read as the standard multiplicative correction 0x70..=0x77.
        let block = ValueInformationBlock::try_from([0x84, 0xFF, 0xF2, 0x00].as_slice()).unwrap();
        let vi = ValueInformation::try_from(&block).unwrap();
        assert_eq!(vi.decimal_scale_exponent, 1);
        let mut labels = vi.labels();
        assert_eq!(labels.next(), Some(ValueLabel::Energy));
        assert_eq!(
            labels.next(),
            Some(ValueLabel::NextVIFEAndDataOfThisBlockAreManufacturerSpecific)
        );
        assert_eq!(labels.next(), None);
    }

    #[test]
    fn exponents_accumulate_and_iterators_clone_mid_chain() {
        use super::*;
        let block =
            ValueInformationBlock::try_from([0x93, 0x92, 0xA8, 0xF5, 0x78].as_slice()).unwrap();
        let vi = ValueInformation::try_from(&block).unwrap();
        assert_eq!(vi.decimal_scale_exponent, -4);
        assert_eq!(vi.decimal_offset_exponent, -3);
        let mut units = vi.units();
        assert_eq!(units.next(), Some(unit!(Meter ^ 3)));
        assert_eq!(units.next(), Some(unit!(Increment)));
        assert!(units.clone().eq(units));
        let mut labels = vi.labels();
        assert_eq!(labels.next(), Some(ValueLabel::Volume));
        assert!(labels.clone().eq(labels));
    }

    #[test]
    fn head_table_and_missing_extensions() {
        use super::*;
        let cases = [
            (
                0x13,
                None,
                None,
                VifInfo {
                    labels: &[ValueLabel::Volume],
                    units: &[unit!(Meter ^ 3)],
                    scale: -3,
                    ..VifInfo::EMPTY
                },
            ),
            (
                0xFD,
                Some(0x1A),
                None,
                labels!(ValueLabel::DigitalOutput, ValueLabel::Binary),
            ),
            (
                0xFB,
                Some(0x1A),
                None,
                VifInfo {
                    labels: &[ValueLabel::RelativeHumidity],
                    units: &[unit!(Percent)],
                    scale: -1,
                    ..VifInfo::EMPTY
                },
            ),
            (
                0xFD,
                Some(0x7D),
                Some(0x3E),
                VifInfo {
                    labels: &[ValueLabel::MoistureLevel],
                    units: &[unit!(Percent)],
                    ..VifInfo::EMPTY
                },
            ),
            (0x7C, None, None, labels!(ValueLabel::PlainText)),
            (0x7F, None, None, labels!(ValueLabel::ManufacturerSpecific)),
        ];
        for (vif, first, second, expected) in cases {
            assert_eq!(head_vif_info(vif.into(), first, second).unwrap(), expected);
        }
        assert!(matches!(
            head_vif_info(0x6F.into(), None, None),
            Err(DataInformationError::Unimplemented { .. })
        ));
        for vif in [0xFD, 0xFB] {
            let mut block = ValueInformationBlock {
                value_information: vif.into(),
                value_information_extension: None,
                plaintext_vife: None,
            };
            assert!(ValueInformation::try_from(&block)
                .unwrap()
                .labels()
                .next()
                .is_none());
            block.value_information_extension = Some(ValueInformationFieldExtensions(&[]));
            assert_eq!(
                ValueInformation::try_from(&block),
                Err(DataInformationError::DataTooShort)
            );
        }
    }
}
