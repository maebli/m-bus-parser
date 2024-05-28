use arrayvec::ArrayVec;

use super::data_information::DataInformationError;

const MAX_VIFE_RECORDS: usize = 10;

impl TryFrom<&[u8]> for ValueInformationBlock {
    type Error = DataInformationError;

    fn try_from(data: &[u8]) -> Result<Self, DataInformationError> {
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
                    return Err(DataInformationError::InvalidValueInformation);
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
            0xFD => ValueInformationCoding::MainVIFExtension,
            0xFB => ValueInformationCoding::AlternateVIFExtension,
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
    MainVIFExtension,
    AlternateVIFExtension,
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
    pub fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.value_information_extension {
            size += vife.len();
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
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x08..=0x0F => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize;
                    }
                    0x10..=0x17 => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 6;
                    }
                    0x18..=0x1F => {
                        units.push(Unit {
                            name: UnitName::Kilogram,
                            exponent: 1,
                        });
                        decimal_scale_exponent =
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x20 | 0x24 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x21 | 0x25 => units.push(Unit {
                        name: UnitName::Minute,
                        exponent: 1,
                    }),
                    0x22 | 0x26 => units.push(Unit {
                        name: UnitName::Hour,
                        exponent: 1,
                    }),
                    0x23 | 0x27 => units.push(Unit {
                        name: UnitName::Day,
                        exponent: 1,
                    }),
                    0x28..=0x2F => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x30..=0x37 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize;
                    }
                    0x38..=0x3F => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 6;
                    }
                    0x40..=0x47 => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: -1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 7;
                    }
                    0x48..=0x4F => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: -1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 9;
                    }
                    0x50..=0x57 => {
                        units.push(Unit {
                            name: UnitName::Kilogram,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b111) as isize - 3;
                    }
                    0x58..=0x5F | 0x64..=0x67 => {
                        units.push(Unit {
                            name: UnitName::Celsius,
                            exponent: 1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x60..=0x63 => {
                        units.push(Unit {
                            name: UnitName::Kelvin,
                            exponent: 1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x68..=0x6B => {
                        units.push(Unit {
                            name: UnitName::Bar,
                            exponent: 1,
                        });
                        decimal_scale_exponent +=
                            (value_information_block.value_information.data & 0b11) as isize - 3;
                    }
                    0x6C..=0x6D => labels.push(ValueLabel::TimePoint),
                    0x72..=0x73 => labels.push(ValueLabel::AveragingDuration),
                    0x74..=0x77 => labels.push(ValueLabel::ActualityDuration),
                    0x78 => labels.push(ValueLabel::FabricationNumber),
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
                match vife[0].data & 0x7F {
                    0x00..=0x03 => {
                        units.push(Unit {
                            name: UnitName::LocalMoneyCurrency,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Credit);
                        decimal_scale_exponent = (vife[0].data & 0b11) as isize - 3;
                    }
                    0x04..=0x07 => {
                        units.push(Unit {
                            name: UnitName::LocalMoneyCurrency,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Debit);
                        decimal_scale_exponent = (vife[0].data & 0b11) as isize - 3;
                    }
                    0x08 => {
                        labels.push(ValueLabel::UniqueMessageIdentificationOrAccessNumber);
                    }
                    0x09 => {
                        labels.push(ValueLabel::DeviceType);
                    }
                    0x0A => {
                        labels.push(ValueLabel::Manufacturer);
                    }
                    0x0B => {
                        labels.push(ValueLabel::ParameterSetIdentification);
                    }
                    0x0C => {
                        labels.push(ValueLabel::ModelOrVersion);
                    }
                    0x0D => {
                        labels.push(ValueLabel::HardwareVersion);
                    }
                    0x0E => {
                        labels.push(ValueLabel::MetrologyFirmwareVersion);
                    }
                    0x0F => {
                        labels.push(ValueLabel::OtherSoftwareVersion);
                    }
                    0x10 => {
                        labels.push(ValueLabel::CustomerLocation);
                    }
                    0x11 => {
                        labels.push(ValueLabel::Customer);
                    }
                    0x12 => {
                        labels.push(ValueLabel::AccessCodeUser);
                    }
                    0x13 => {
                        labels.push(ValueLabel::AccessCodeOperator);
                    }
                    0x14 => {
                        labels.push(ValueLabel::AccessCodeSystemOperator);
                    }
                    0x15 => {
                        labels.push(ValueLabel::AccessCodeDeveloper);
                    }
                    0x16 => {
                        labels.push(ValueLabel::Password);
                    }
                    0x17 => {
                        labels.push(ValueLabel::ErrorFlags);
                    }
                    0x18 => {
                        labels.push(ValueLabel::ErrorMask);
                    }
                    0x19 => {
                        labels.push(ValueLabel::SecurityKey);
                    }
                    0x1A => {
                        labels.push(ValueLabel::DigitalOutput);
                        labels.push(ValueLabel::Binary);
                    }
                    0x1B => {
                        labels.push(ValueLabel::DigitalInput);
                        labels.push(ValueLabel::Binary);
                    }
                    0x1C => {
                        units.push(Unit {
                            name: UnitName::Symbol,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: -1,
                        });
                        labels.push(ValueLabel::BaudRate);
                    }
                    0x1D => {
                        units.push(Unit {
                            name: UnitName::BitTime,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::ResponseDelayTime);
                    }
                    0x1E => {
                        labels.push(ValueLabel::Retry);
                    }
                    0x1F => {
                        labels.push(ValueLabel::RemoteControl);
                    }
                    0x20 => {
                        labels.push(ValueLabel::FirstStorageForCycleStorage);
                    }
                    0x21 => {
                        labels.push(ValueLabel::LastStorageForCycleStorage);
                    }
                    0x22 => {
                        labels.push(ValueLabel::SizeOfStorageBlock);
                    }
                    0x23 => {
                        labels.push(ValueLabel::DescripitonOfTariffAndSubunit);
                    }
                    0x24 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x25 => {
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x26 => {
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x27 => {
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x28 => {
                        units.push(Unit {
                            name: UnitName::Month,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x29 => {
                        units.push(Unit {
                            name: UnitName::Year,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::StorageInterval);
                    }
                    0x30 => {
                        labels.push(ValueLabel::Dimensionless);
                    }
                    0x31 => {
                        labels.push(ValueLabel::DataContainerForWmbusProtocol);
                    }
                    0x32 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x33 => {
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x34 => {
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x35 => {
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::PeriodOfNormalDataTransmition);
                    }
                    0x50..=0x5F => {
                        units.push(Unit {
                            name: UnitName::Volt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = (vife[0].data & 0b1111) as isize - 9;
                    }
                    0x60 => {
                        labels.push(ValueLabel::ResetCounter);
                    }
                    0x61 => {
                        labels.push(ValueLabel::CumulationCounter);
                    }
                    0x62 => {
                        labels.push(ValueLabel::ControlSignal);
                    }
                    0x63 => {
                        labels.push(ValueLabel::DayOfWeek);
                    }
                    0x64 => {
                        labels.push(ValueLabel::WeekNumber);
                    }
                    0x65 => {
                        labels.push(ValueLabel::TimePointOfChangeOfTariff);
                    }
                    0x66 => {
                        labels.push(ValueLabel::StateOfParameterActivation);
                    }
                    0x67 => {
                        labels.push(ValueLabel::SpecialSupplierInformation);
                    }
                    0x68 => {
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x69 => {
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6A => {
                        units.push(Unit {
                            name: UnitName::Month,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6B => {
                        units.push(Unit {
                            name: UnitName::Year,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::DurationSinceLastCumulation);
                    }
                    0x6C => {
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6D => {
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6E => {
                        units.push(Unit {
                            name: UnitName::Month,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x6F => {
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::OperatingTimeBattery);
                    }
                    0x70 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::DateAndTimeOfBatteryChange);
                    }
                    0x71 => {
                        units.push(Unit {
                            name: UnitName::DecibelMilliWatt,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::RFPowerLevel);
                    }
                    0x72 => {
                        labels.push(ValueLabel::DaylightSavingBeginningEndingDeviation);
                    }
                    0x73 => {
                        labels.push(ValueLabel::ListeningWindowManagementData);
                    }
                    0x74 => {
                        labels.push(ValueLabel::RemainingBatteryLifeTime);
                    }
                    0x75 => {
                        labels.push(ValueLabel::NumberOfTimesTheMeterWasStopped);
                    }
                    0x76 => {
                        labels.push(ValueLabel::DataContainerForManufacturerSpecificProtocol);
                    }
                    0x7D => match vife[1].data & 0x7F {
                        0x00 => {
                            labels.push(ValueLabel::CurrentlySelectedApplication);
                        }
                        0x02 => {
                            units.push(Unit {
                                name: UnitName::Month,
                                exponent: 1,
                            });
                            labels.push(ValueLabel::RemainingBatteryLifeTime);
                        }
                        0x03 => {
                            units.push(Unit {
                                name: UnitName::Year,
                                exponent: 1,
                            });
                            labels.push(ValueLabel::RemainingBatteryLifeTime);
                        }
                        _ => {
                            labels.push(ValueLabel::Reserved);
                        }
                    },
                    _ => {
                        labels.push(ValueLabel::Reserved);
                    }
                }
            }
            ValueInformationCoding::AlternateVIFExtension => {
                let vife = value_information_block
                    .value_information_extension
                    .as_ref()
                    .ok_or(Self::Error::InvalidValueInformation)?;
                match vife[0].data & 0x7F {
                    0x00 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 5;
                    }
                    0x01 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 3,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 6;
                    }
                    0x02 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 3;
                    }
                    0x03 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 4;
                    }
                    0x08 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 8;
                    }
                    0x09 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 9;
                    }
                    0x0C => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 5;
                    }
                    0x0D => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 6;
                    }
                    0x0E => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 7;
                    }
                    0x0F => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        labels.push(ValueLabel::Energy);
                        decimal_scale_exponent = 8;
                    }
                    0x10 => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        decimal_scale_exponent = 2;
                    }
                    0x11 => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: 3,
                        });
                        decimal_scale_exponent = 3;
                    }
                    0x14 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -3;
                    }
                    0x15 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -2;
                    }
                    0x16 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                    }
                    0x17 => {
                        units.push(Unit {
                            name: UnitName::ReactiveWatt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                    }
                    0x18 => {
                        units.push(Unit {
                            name: UnitName::Tonne,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 2;
                    }
                    0x19 => {
                        units.push(Unit {
                            name: UnitName::Tonne,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 3;
                    }
                    0x1A => {
                        units.push(Unit {
                            name: UnitName::Percent,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                        labels.push(ValueLabel::RelativeHumidity);
                    }
                    0x20 => {
                        units.push(Unit {
                            name: UnitName::Feet,
                            exponent: 3,
                        });
                        decimal_scale_exponent = 0;
                    }
                    0x21 => {
                        units.push(Unit {
                            name: UnitName::Feet,
                            exponent: 3,
                        });
                        decimal_scale_exponent = 1;
                    }
                    0x28 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 5;
                    }
                    0x29 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 6;
                    }
                    0x2A => {
                        units.push(Unit {
                            name: UnitName::Degree,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                        labels.push(ValueLabel::PhaseUtoU);
                    }
                    0x2B => {
                        units.push(Unit {
                            name: UnitName::Degree,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                        labels.push(ValueLabel::PhaseUtoI);
                    }
                    0x2C => {
                        units.push(Unit {
                            name: UnitName::Hertz,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -3;
                    }
                    0x2D => {
                        units.push(Unit {
                            name: UnitName::Hertz,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -2;
                    }
                    0x2E => {
                        units.push(Unit {
                            name: UnitName::Hertz,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                    }
                    0x2F => {
                        units.push(Unit {
                            name: UnitName::Hertz,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                    }
                    0x30 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = -8;
                    }
                    0x31 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = -7;
                    }
                    0x34 => {
                        units.push(Unit {
                            name: UnitName::ApparentWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = 0;
                    }
                    0x35 => {
                        units.push(Unit {
                            name: UnitName::ApparentWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = 1;
                    }
                    0x36 => {
                        units.push(Unit {
                            name: UnitName::ApparentWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = 2;
                    }
                    0x37 => {
                        units.push(Unit {
                            name: UnitName::ApparentWatt,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        decimal_scale_exponent = 3;
                    }
                    0x74 => {
                        units.push(Unit {
                            name: UnitName::Celsius,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -3;
                        labels.push(ValueLabel::ColdWarmTemperatureLimit);
                    }
                    0x75 => {
                        units.push(Unit {
                            name: UnitName::Celsius,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -2;
                        labels.push(ValueLabel::ColdWarmTemperatureLimit);
                    }
                    0x76 => {
                        units.push(Unit {
                            name: UnitName::Celsius,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                        labels.push(ValueLabel::ColdWarmTemperatureLimit);
                    }
                    0x77 => {
                        units.push(Unit {
                            name: UnitName::Celsius,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ColdWarmTemperatureLimit);
                    }
                    0x78 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -3;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x79 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -2;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7A => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = -1;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7B => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7C => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 1;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7D => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 2;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7E => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 3;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x7F => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 4;
                        labels.push(ValueLabel::CumaltiveMaximumOfActivePower);
                    }
                    0x68 => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ResultingRatingFactor);
                    }
                    0x69 => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ThermalOutputRatingFactor);
                    }
                    0x6A => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ThermalCouplingRatingFactorOverall);
                    }
                    0x6B => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ThermalCouplingRatingRoomSide);
                    }
                    0x6C => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::ThermalCouplingRatingFactorHeatingSide);
                    }
                    0x6D => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::LowTemperatureRatingFactor);
                    }
                    0x6E => {
                        units.push(Unit {
                            name: UnitName::HCAUnit,
                            exponent: 1,
                        });
                        decimal_scale_exponent = 0;
                        labels.push(ValueLabel::DisplayOutputScalingFacttor);
                    }

                    _ => todo!("Implement the rest of the units: {:X?}", vife[0].data),
                }
            }
            ValueInformationCoding::PlainText => {
                labels.push(ValueLabel::PlainText);
            }
            x => todo!("Implement the rest of the units: {:?}", x),
        }

        Ok(ValueInformation {
            decimal_offset_exponent,
            decimal_scale_exponent,
            units,
            labels,
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
                    0x20 => units.push(Unit {
                        name: UnitName::Second,
                        exponent: -1,
                    }),
                    0x21 => units.push(Unit {
                        name: UnitName::Minute,
                        exponent: -1,
                    }),
                    0x22 => units.push(Unit {
                        name: UnitName::Hour,
                        exponent: -1,
                    }),
                    0x23 => units.push(Unit {
                        name: UnitName::Day,
                        exponent: -1,
                    }),
                    0x24 => units.push(Unit {
                        name: UnitName::Week,
                        exponent: -1,
                    }),
                    0x25 => units.push(Unit {
                        name: UnitName::Month,
                        exponent: -1,
                    }),
                    0x26 => units.push(Unit {
                        name: UnitName::Year,
                        exponent: -1,
                    }),
                    0x27 => units.push(Unit {
                        name: UnitName::Revolution,
                        exponent: -1,
                    }),
                    0x28 => {
                        units.push(Unit {
                            name: UnitName::Increment,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::InputPulseOnChannel0,
                            exponent: -1,
                        })
                    }
                    0x29 => {
                        units.push(Unit {
                            name: UnitName::Increment,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::OutputPulseOnChannel0,
                            exponent: -1,
                        })
                    }
                    0x2A => {
                        units.push(Unit {
                            name: UnitName::Increment,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::InputPulseOnChannel1,
                            exponent: -1,
                        })
                    }
                    0x2B => {
                        units.push(Unit {
                            name: UnitName::Increment,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::OutputPulseOnChannel1,
                            exponent: -1,
                        })
                    }
                    0x2C => {
                        units.push(Unit {
                            name: UnitName::Liter,
                            exponent: 1,
                        });
                    }
                    0x2D => {
                        units.push(Unit {
                            name: UnitName::Meter,
                            exponent: -3,
                        });
                    }
                    0x2E => {
                        units.push(Unit {
                            name: UnitName::Kilogram,
                            exponent: -1,
                        });
                    }
                    0x2F => {
                        units.push(Unit {
                            name: UnitName::Kelvin,
                            exponent: -1,
                        });
                    }
                    0x30 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: -1,
                        });
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: -1,
                        });
                        *decimal_scale_exponent -= 3;
                    }
                    0x31 => {
                        units.push(Unit {
                            name: UnitName::Joul,
                            exponent: -1,
                        });
                        *decimal_scale_exponent += -9;
                    }
                    0x32 => {
                        units.push(Unit {
                            name: UnitName::Watt,
                            exponent: -1,
                        });
                        *decimal_scale_exponent += -3;
                    }
                    0x33 => {
                        units.push(Unit {
                            name: UnitName::Kelvin,
                            exponent: -1,
                        });
                        units.push(Unit {
                            name: UnitName::Liter,
                            exponent: -1,
                        });
                    }
                    0x34 => {
                        units.push(Unit {
                            name: UnitName::Volt,
                            exponent: -1,
                        });
                    }
                    0x35 => {
                        units.push(Unit {
                            name: UnitName::Ampere,
                            exponent: -1,
                        });
                    }
                    0x36 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x37 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Volt,
                            exponent: -1,
                        });
                    }
                    0x38 => {
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                        units.push(Unit {
                            name: UnitName::Ampere,
                            exponent: -1,
                        });
                    }
                    0x39 => {
                        labels.push(ValueLabel::StartDateOf);
                    }
                    0x3A => {
                        labels.push(ValueLabel::VifContinsUncorrectedUnitOrValue);
                    }
                    0x3B => {
                        labels.push(ValueLabel::AccumulationOnlyIfValueIsPositive);
                    }
                    0x3C => {
                        labels.push(ValueLabel::AccumulationOnlyIfValueIsNegative);
                    }
                    0x3D => {
                        labels.push(ValueLabel::NoneMetricUnits);
                    }
                    0x3E => {
                        labels.push(ValueLabel::ValueAtBaseConditions);
                    }
                    0x3F => {
                        labels.push(ValueLabel::ObisDecleration);
                    }
                    0x40 => {
                        labels.push(ValueLabel::UpperLimitValue);
                    }
                    0x48 => {
                        labels.push(ValueLabel::LowerLimitValue);
                    }
                    0x41 => {
                        labels.push(ValueLabel::NumberOfExceedsOfUpperLimitValue);
                    }
                    0x49 => {
                        labels.push(ValueLabel::NumberOfExceedsOfLowerLimitValue);
                    }
                    0x42 => {
                        labels.push(ValueLabel::DateOfBeginFirstLowerLimitExceed);
                    }
                    0x43 => {
                        labels.push(ValueLabel::DateOfBeginFirstUpperLimitExceed);
                    }
                    0x46 => {
                        labels.push(ValueLabel::DateOfBeginLastLowerLimitExceed);
                    }
                    0x47 => {
                        labels.push(ValueLabel::DateOfBeginLastUpperLimitExceed);
                    }
                    0x4A => {
                        labels.push(ValueLabel::DateOfEndLastLowerLimitExceed);
                    }
                    0x4B => {
                        labels.push(ValueLabel::DateOfEndLastUpperLimitExceed);
                    }
                    0x4E => {
                        labels.push(ValueLabel::DateOfEndFirstLowerLimitExceed);
                    }
                    0x4F => {
                        labels.push(ValueLabel::DateOfEndFirstUpperLimitExceed);
                    }
                    0x50 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x51 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x52 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                    }
                    0x53 => {
                        labels.push(ValueLabel::DurationOfFirstLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                    }
                    0x54 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x55 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x56 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                    }
                    0x57 => {
                        labels.push(ValueLabel::DurationOfFirstUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                    }
                    0x58 => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x59 => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x5A => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                    }
                    0x5B => {
                        labels.push(ValueLabel::DurationOfLastLowerLimitExceed);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                    }
                    0x5C => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x5D => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x5E => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                    }
                    0x5F => {
                        labels.push(ValueLabel::DurationOfLastUpperLimitExceed);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                    }
                    0x60 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x61 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x62 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(Unit {
                            name: UnitName::Hour,
                            exponent: 1,
                        });
                    }
                    0x63 => {
                        labels.push(ValueLabel::DurationOfFirst);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
                    }
                    0x64 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(Unit {
                            name: UnitName::Second,
                            exponent: 1,
                        });
                    }
                    0x65 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(Unit {
                            name: UnitName::Minute,
                            exponent: 1,
                        });
                    }
                    0x66 => {
                        labels.push(ValueLabel::DurationOfLast);
                        units.push(Unit {
                            name: UnitName::Day,
                            exponent: 1,
                        });
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
                    0x7D => {
                        labels.push(ValueLabel::MultiplicativeCorrectionFactor103);
                    }
                    0x7E => {
                        labels.push(ValueLabel::FutureValue);
                    }
                    0x7F => {
                        labels.push(ValueLabel::NextVIFEAndDataOfThisBlockAreManufacturerSpecific);
                    }
                    _ => {
                        labels.push(ValueLabel::Reserved);
                    }
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
        ValueInformationField { data }
    }
}
/// This is the most important type of the this file and represents
/// the whole information inside the value information block
/// value(x) = (multiplier * value + offset) * units
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
        write!(
            f,
            "+{})e{}",
            self.decimal_offset_exponent, self.decimal_scale_exponent
        )?;
        write!(f, "[")?;
        for unit in &self.units {
            write!(f, "{} ", unit)?;
        }
        write!(f, "]")?;
        write!(f, "(")?;
        for label in &self.labels {
            write!(f, "{:?}, ", label)?;
        }
        write!(f, ")")
    }
}

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
    TimePoint,
    FabricationNumber,
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
    Dimensionless,
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
    DisplayOutputScalingFacttor,
}
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Unit {
    pub name: UnitName,
    pub exponent: i32,
}

#[cfg(feature = "std")]
impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.exponent == 1 {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}^{}", self.name, self.exponent)
        }
    }
}

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
                value_information_extension: None
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
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    });
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
                value_information_extension: None
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
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    });
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
                value_information_extension: None
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
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    });
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
                value_information_extension: None
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
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    });
                    x.push(Unit {
                        name: UnitName::Second,
                        exponent: -1,
                    });
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
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: 3,
                    });
                    x.push(Unit {
                        name: UnitName::Second,
                        exponent: -1,
                    });
                    x.push(Unit {
                        name: UnitName::Meter,
                        exponent: -3,
                    });
                    x
                }
            }
        );
    }

    #[test]
    fn test_plain_text_vif_norm_conform() {
        use arrayvec::ArrayVec;

        use crate::user_data::value_information::{Unit, ValueInformation, ValueLabel};

        use crate::user_data::value_information::ValueInformationBlock;
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
        assert_eq!(
            ValueInformation::try_from(&result).unwrap(),
            ValueInformation {
                decimal_offset_exponent: 0,
                decimal_scale_exponent: 0,
                units: {
                    let x = ArrayVec::<Unit, 10>::new();
                    x
                },
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
}
