//! is a part of the application layer
#[cfg(feature = "std")]
use std::fmt;

use variable_user_data::DataRecordError;

use self::data_record::DataRecord;

pub mod data_information;
pub mod data_record;
pub mod value_information;
pub mod variable_user_data;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "Vec<DataRecord>"))]
#[derive(Clone, Debug, PartialEq)]
pub struct DataRecords<'a> {
    offset: usize,
    data: &'a [u8],
}

#[cfg(feature = "serde")]
impl<'a> From<DataRecords<'a>> for Vec<DataRecord<'a>> {
    fn from(value: DataRecords<'a>) -> Self {
        let value: Result<Vec<_>, _> = value.collect();
        value.unwrap_or_default()
    }
}

impl<'a> Iterator for DataRecords<'a> {
    type Item = Result<DataRecord<'a>, DataRecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut _more_records_follow = false;

        while self.offset < self.data.len() {
            match self.data.get(self.offset)? {
                0x0F => {
                    /* TODO: parse manufacturer specific */
                    self.offset = self.data.len();
                }
                0x1F => {
                    /* TODO: parse manufacturer specific */
                    _more_records_follow = true;
                    self.offset = self.data.len();
                }
                0x2F => {
                    self.offset += 1;
                }
                _ => {
                    let record = DataRecord::try_from(self.data.get(self.offset..)?);
                    if let Ok(record) = record {
                        self.offset += record.get_size();
                        return Some(Ok(record));
                    } else {
                        self.offset = self.data.len();
                    }
                }
            }
        }
        None
    }
}

impl<'a> DataRecords<'a> {
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        DataRecords { offset: 0, data }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct StatusField: u8 {
        const COUNTER_BINARY_SIGNED     = 0b0000_0001;
        const COUNTER_FIXED_DATE        = 0b0000_0010;
        const POWER_LOW                 = 0b0000_0100;
        const PERMANENT_ERROR           = 0b0000_1000;
        const TEMPORARY_ERROR           = 0b0001_0000;
        const MANUFACTURER_SPECIFIC_1   = 0b0010_0000;
        const MANUFACTURER_SPECIFIC_2   = 0b0100_0000;
        const MANUFACTURER_SPECIFIC_3   = 0b1000_0000;
    }
}

#[cfg(feature = "std")]
impl fmt::Display for StatusField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut status = String::new();
        if self.contains(StatusField::COUNTER_BINARY_SIGNED) {
            status.push_str("Counter binary signed, ");
        }
        if self.contains(StatusField::COUNTER_FIXED_DATE) {
            status.push_str("Counter fixed date, ");
        }
        if self.contains(StatusField::POWER_LOW) {
            status.push_str("Power low, ");
        }
        if self.contains(StatusField::PERMANENT_ERROR) {
            status.push_str("Permanent error, ");
        }
        if self.contains(StatusField::TEMPORARY_ERROR) {
            status.push_str("Temporary error, ");
        }
        if self.contains(StatusField::MANUFACTURER_SPECIFIC_1) {
            status.push_str("Manufacturer specific 1, ");
        }
        if self.contains(StatusField::MANUFACTURER_SPECIFIC_2) {
            status.push_str("Manufacturer specific 2, ");
        }
        if self.contains(StatusField::MANUFACTURER_SPECIFIC_3) {
            status.push_str("Manufacturer specific 3, ");
        }
        if status.is_empty() {
            status.push_str("No Error(s)");
        }
        write!(f, "{}", status.trim_end_matches(", "))
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    SlaveToMaster,
    MasterToSlave,
}

// implement from trait for diirection
impl From<ControlInformation> for Direction {
    fn from(single_byte: ControlInformation) -> Self {
        match single_byte {
            ControlInformation::ResetAtApplicationLevel => Self::MasterToSlave,
            ControlInformation::SendData => Self::MasterToSlave,
            ControlInformation::SelectSlave => Self::MasterToSlave,
            ControlInformation::SynchronizeSlave => Self::MasterToSlave,
            ControlInformation::SetBaudRate300 => Self::MasterToSlave,
            ControlInformation::SetBaudRate600 => Self::MasterToSlave,
            ControlInformation::SetBaudRate1200 => Self::MasterToSlave,
            ControlInformation::SetBaudRate2400 => Self::MasterToSlave,
            ControlInformation::SetBaudRate4800 => Self::MasterToSlave,
            ControlInformation::SetBaudRate9600 => Self::MasterToSlave,
            ControlInformation::SetBaudRate19200 => Self::MasterToSlave,
            ControlInformation::SetBaudRate38400 => Self::MasterToSlave,
            ControlInformation::OutputRAMContent => Self::MasterToSlave,
            ControlInformation::WriteRAMContent => Self::MasterToSlave,
            ControlInformation::StartCalibrationTestMode => Self::MasterToSlave,
            ControlInformation::ReadEEPROM => Self::MasterToSlave,
            ControlInformation::StartSoftwareTest => Self::MasterToSlave,
            ControlInformation::HashProcedure(_) => Self::MasterToSlave,
            ControlInformation::SendErrorStatus => Self::SlaveToMaster,
            ControlInformation::SendAlarmStatus => Self::SlaveToMaster,
            ControlInformation::ResponseWithVariableDataStructure => Self::SlaveToMaster,
            ControlInformation::ResponseWithFixedDataStructure => Self::SlaveToMaster,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ControlInformation {
    SendData,
    SelectSlave,
    ResetAtApplicationLevel,
    SynchronizeSlave,
    SetBaudRate300,
    SetBaudRate600,
    SetBaudRate1200,
    SetBaudRate2400,
    SetBaudRate4800,
    SetBaudRate9600,
    SetBaudRate19200,
    SetBaudRate38400,
    OutputRAMContent,
    WriteRAMContent,
    StartCalibrationTestMode,
    ReadEEPROM,
    StartSoftwareTest,
    HashProcedure(u8),
    SendErrorStatus,
    SendAlarmStatus,
    ResponseWithVariableDataStructure,
    ResponseWithFixedDataStructure,
}

impl ControlInformation {
    const fn from(byte: u8) -> Result<Self, ApplicationLayerError> {
        match byte {
            0x50 => Ok(Self::ResetAtApplicationLevel),
            0x51 => Ok(Self::SendData),
            0x52 => Ok(Self::SelectSlave),
            0x54 => Ok(Self::SynchronizeSlave),
            0xB8 => Ok(Self::SetBaudRate300),
            0xB9 => Ok(Self::SetBaudRate600),
            0xBA => Ok(Self::SetBaudRate1200),
            0xBB => Ok(Self::SetBaudRate2400),
            0xBC => Ok(Self::SetBaudRate4800),
            0xBD => Ok(Self::SetBaudRate9600),
            0xBE => Ok(Self::SetBaudRate19200),
            0xBF => Ok(Self::SetBaudRate38400),
            0xB1 => Ok(Self::OutputRAMContent),
            0xB2 => Ok(Self::WriteRAMContent),
            0xB3 => Ok(Self::StartCalibrationTestMode),
            0xB4 => Ok(Self::ReadEEPROM),
            0xB6 => Ok(Self::StartSoftwareTest),
            0x90..=0x97 => Ok(Self::HashProcedure(byte - 0x90)),
            0x70 => Ok(Self::SendErrorStatus),
            0x71 => Ok(Self::SendAlarmStatus),
            0x72 | 0x76 => Ok(Self::ResponseWithVariableDataStructure),
            0x73 | 0x77 => Ok(Self::ResponseWithFixedDataStructure),
            _ => Err(ApplicationLayerError::InvalidControlInformation { byte }),
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ApplicationLayerError {
    MissingControlInformation,
    InvalidControlInformation { byte: u8 },
    IdentificationNumberError { digits: [u8; 4], number: u32 },
    InvalidManufacturerCode { code: u16 },
    InsufficientData,
}

#[cfg(feature = "std")]
impl fmt::Display for ApplicationLayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationLayerError::MissingControlInformation => {
                write!(f, "Missing control information")
            }
            ApplicationLayerError::InvalidControlInformation { byte } => {
                write!(f, "Invalid control information: {}", byte)
            }
            ApplicationLayerError::InvalidManufacturerCode { code } => {
                write!(f, "Invalid manufacturer code: {}", code)
            }
            ApplicationLayerError::IdentificationNumberError { digits, number } => {
                write!(
                    f,
                    "Invalid identification number: {:?}, number: {}",
                    digits, number
                )
            }
            ApplicationLayerError::InsufficientData => {
                write!(f, "Insufficient data")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ApplicationLayerError {}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum ApplicationResetSubcode {
    All(u8),
    UserData(u8),
    SimpleBilling(u8),
    EnhancedBilling(u8),
    MultiTariffBilling(u8),
    InstantaneousValues(u8),
    LoadManagementValues(u8),
    Reserved1(u8),
    InstallationStartup(u8),
    Testing(u8),
    Calibration(u8),
    ConfigurationUpdates(u8),
    Manufacturing(u8),
    Development(u8),
    Selftest(u8),
    Reserved2(u8),
}

#[cfg(feature = "std")]
impl fmt::Display for ApplicationResetSubcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let subcode = match self {
            Self::All(_) => "All",
            Self::UserData(_) => "User data",
            Self::SimpleBilling(_) => "Simple billing",
            Self::EnhancedBilling(_) => "Enhanced billing",
            Self::MultiTariffBilling(_) => "Multi-tariff billing",
            Self::InstantaneousValues(_) => "Instantaneous values",
            Self::LoadManagementValues(_) => "Load management values",
            Self::Reserved1(_) => "Reserved",
            Self::InstallationStartup(_) => "Installation startup",
            Self::Testing(_) => "Testing",
            Self::Calibration(_) => "Calibration",
            Self::ConfigurationUpdates(_) => "Configuration updates",
            Self::Manufacturing(_) => "Manufacturing",
            Self::Development(_) => "Development",
            Self::Selftest(_) => "Self-test",
            Self::Reserved2(_) => "Reserved",
        };
        write!(f, "{}", subcode)
    }
}

impl ApplicationResetSubcode {
    #[must_use]
    pub const fn from(value: u8) -> Self {
        match value & 0b1111 {
            // Extracting the lower 4 bits
            0b0000 => Self::All(value),
            0b0001 => Self::UserData(value),
            0b0010 => Self::SimpleBilling(value),
            0b0011 => Self::EnhancedBilling(value),
            0b0100 => Self::MultiTariffBilling(value),
            0b0101 => Self::InstantaneousValues(value),
            0b0110 => Self::LoadManagementValues(value),
            0b0111 => Self::Reserved1(value),
            0b1000 => Self::InstallationStartup(value),
            0b1001 => Self::Testing(value),
            0b1010 => Self::Calibration(value),
            0b1011 => Self::ConfigurationUpdates(value),
            0b1100 => Self::Manufacturing(value),
            0b1101 => Self::Development(value),
            0b1110 => Self::Selftest(value),
            _ => Self::Reserved2(value),
        }
    }
}

fn bcd_hex_digits_to_u32(digits: [u8; 4]) -> Result<u32, ApplicationLayerError> {
    let mut number = 0u32;

    for &digit in digits.iter().rev() {
        let lower = digit & 0x0F;
        let upper = digit >> 4;
        if lower > 9 || upper > 9 {
            return Err(ApplicationLayerError::IdentificationNumberError { digits, number });
        }
        number = number * 100 + (u32::from(upper) * 10) + u32::from(lower);
    }

    Ok(number)
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct Counter {
    count: u32,
}

#[cfg(feature = "std")]
impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08}", self.count)
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct IdentificationNumber {
    pub number: u32,
}

#[cfg(feature = "std")]
impl fmt::Display for IdentificationNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08}", self.number)
    }
}

impl From<IdentificationNumber> for u32 {
    fn from(id: IdentificationNumber) -> Self {
        id.number
    }
}

impl IdentificationNumber {
    pub fn from_bcd_hex_digits(digits: [u8; 4]) -> Result<Self, ApplicationLayerError> {
        let number = bcd_hex_digits_to_u32(digits)?;
        Ok(Self { number })
    }
}

impl Counter {
    pub fn from_bcd_hex_digits(digits: [u8; 4]) -> Result<Self, ApplicationLayerError> {
        let count = bcd_hex_digits_to_u32(digits)?;
        Ok(Self { count })
    }
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub struct FixedDataHeder {
    identification_number: IdentificationNumber,
    manufacturer_code: ManufacturerCode,
    version: u8,
    medium: Medium,
    access_number: u8,
    status: StatusField,
    signature: u16,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq)]
pub enum UserDataBlock<'a> {
    ResetAtApplicationLevel {
        subcode: ApplicationResetSubcode,
    },
    FixedDataStructure {
        identification_number: IdentificationNumber,
        access_number: u8,
        status: StatusField,
        medium_ad_unit: u16,
        counter1: Counter,
        counter2: Counter,
    },
    VariableDataStructure {
        fixed_data_header: FixedDataHeader,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        variable_data_block: &'a [u8],
    },
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Medium {
    Other,
    Oil,
    Electricity,
    Gas,
    Heat,
    Steam,
    HotWater,
    Water,
    HeatCostAllocator,
    Reserved,
    GasMode2,
    HeatMode2,
    HotWaterMode2,
    WaterMode2,
    HeatCostAllocator2,
    ReservedMode2,
    Unknown,
    ColdWater,
    DualWater,
    Pressure,
    ADConverter,
}

impl Medium {
    #[must_use]
    pub const fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => Self::Other,
            0x01 => Self::Oil,
            0x02 => Self::Electricity,
            0x03 => Self::Gas,
            0x04 => Self::Heat,
            0x05 => Self::Steam,
            0x06 => Self::HotWater,
            0x07 => Self::Water,
            0x08 => Self::HeatCostAllocator,
            0x09 => Self::Reserved, // Note: Reserved for 0x09 from the first set
            0x0A => Self::GasMode2,
            0x0B => Self::HeatMode2,
            0x0C => Self::HotWaterMode2,
            0x0D => Self::WaterMode2,
            0x0E => Self::HeatCostAllocator2,
            0x0F => Self::ReservedMode2,
            // Unique mediums from the second set
            0x10 => Self::Reserved, // Reserved range
            0x11 => Self::Reserved, // Reserved range
            0x12 => Self::Reserved, // Reserved range
            0x13 => Self::Reserved, // Reserved range
            0x14 => Self::Reserved, // Reserved range
            0x15 => Self::Reserved, // Reserved range
            0x16 => Self::ColdWater,
            0x17 => Self::DualWater,
            0x18 => Self::Pressure,
            0x19 => Self::ADConverter,
            // Extended reserved range from the second set
            0x20..=0xFF => Self::Reserved,
            _ => Self::Unknown,
        }
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Medium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let medium = match self {
            Self::Other => "Other",
            Self::Oil => "Oil",
            Self::Electricity => "Electricity",
            Self::Gas => "Gas",
            Self::Heat => "Heat",
            Self::Steam => "Steam",
            Self::HotWater => "Hot water",
            Self::Water => "Water",
            Self::HeatCostAllocator => "Heat Cost Allocator",
            Self::Reserved => "Reserved",
            Self::GasMode2 => "Gas Mode 2",
            Self::HeatMode2 => "Heat Mode 2",
            Self::HotWaterMode2 => "Hot Water Mode 2",
            Self::WaterMode2 => "Water Mode 2",
            Self::HeatCostAllocator2 => "Heat Cost Allocator 2",
            Self::ReservedMode2 => "Reserved",
            Self::Unknown => "Unknown",
            Self::ColdWater => "Cold Water",
            Self::DualWater => "Dual Water",
            Self::Pressure => "Pressure",
            Self::ADConverter => "AD Converter",
        };
        write!(f, "{}", medium)
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct FixedDataHeader {
    pub identification_number: IdentificationNumber,
    pub manufacturer: ManufacturerCode,
    pub version: u8,
    pub medium: Medium,
    pub access_number: u8,
    pub status: StatusField,
    pub signature: u16,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ManufacturerCode {
    pub code: [char; 3],
}

impl ManufacturerCode {
    pub const fn from_id(id: u16) -> Result<Self, ApplicationLayerError> {
        let first_letter = ((id / (32 * 32)) + 64) as u8 as char;
        let second_letter = (((id % (32 * 32)) / 32) + 64) as u8 as char;
        let third_letter = ((id % 32) + 64) as u8 as char;

        if first_letter.is_ascii_uppercase()
            && second_letter.is_ascii_uppercase()
            && third_letter.is_ascii_uppercase()
        {
            Ok(Self {
                code: [first_letter, second_letter, third_letter],
            })
        } else {
            Err(ApplicationLayerError::InvalidManufacturerCode { code: id })
        }
    }
}

#[cfg(feature = "std")]
impl fmt::Display for ManufacturerCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.code[0], self.code[1], self.code[2])
    }
}

#[derive(Debug, PartialEq)]
pub struct MeasuredMedium {
    pub medium: Medium,
}

impl MeasuredMedium {
    #[must_use]
    pub const fn new(byte: u8) -> Self {
        Self {
            medium: Medium::from_byte(byte),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for UserDataBlock<'a> {
    type Error = ApplicationLayerError;

    fn try_from(data: &'a [u8]) -> Result<Self, ApplicationLayerError> {
        if data.is_empty() {
            return Err(ApplicationLayerError::MissingControlInformation);
        }

        let control_information = ControlInformation::from(
            *data
                .first()
                .ok_or(ApplicationLayerError::InsufficientData)?,
        )?;

        match control_information {
            ControlInformation::ResetAtApplicationLevel => {
                let subcode = ApplicationResetSubcode::from(
                    *data.get(1).ok_or(ApplicationLayerError::InsufficientData)?,
                );
                Ok(UserDataBlock::ResetAtApplicationLevel { subcode })
            }
            ControlInformation::SendData => todo!(),
            ControlInformation::SelectSlave => todo!(),
            ControlInformation::SynchronizeSlave => todo!(),
            ControlInformation::SetBaudRate300 => todo!(),
            ControlInformation::SetBaudRate600 => todo!(),
            ControlInformation::SetBaudRate1200 => todo!(),
            ControlInformation::SetBaudRate2400 => todo!(),
            ControlInformation::SetBaudRate4800 => todo!(),
            ControlInformation::SetBaudRate9600 => todo!(),
            ControlInformation::SetBaudRate19200 => todo!(),
            ControlInformation::SetBaudRate38400 => todo!(),
            ControlInformation::OutputRAMContent => todo!(),
            ControlInformation::WriteRAMContent => todo!(),
            ControlInformation::StartCalibrationTestMode => todo!(),
            ControlInformation::ReadEEPROM => todo!(),
            ControlInformation::StartSoftwareTest => todo!(),
            ControlInformation::HashProcedure(_) => todo!(),
            ControlInformation::SendErrorStatus => todo!(),
            ControlInformation::SendAlarmStatus => todo!(),
            ControlInformation::ResponseWithVariableDataStructure => {
                let mut iter = data.iter().skip(1);
                Ok(UserDataBlock::VariableDataStructure {
                    fixed_data_header: FixedDataHeader {
                        identification_number: IdentificationNumber::from_bcd_hex_digits([
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ])?,
                        manufacturer: ManufacturerCode::from_id(u16::from_le_bytes([
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ]))?,
                        version: *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        medium: MeasuredMedium::new(
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        )
                        .medium,
                        access_number: *iter
                            .next()
                            .ok_or(ApplicationLayerError::InsufficientData)?,
                        status: StatusField::from_bits_truncate(
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ),
                        signature: u16::from_le_bytes([
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ]),
                    },
                    variable_data_block: data
                        .get(13..data.len())
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                })
            }
            ControlInformation::ResponseWithFixedDataStructure => {
                let mut iter = data.iter().skip(1);
                let identification_number = IdentificationNumber::from_bcd_hex_digits([
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                ])?;

                let access_number = *iter.next().ok_or(ApplicationLayerError::InsufficientData)?;

                let status = StatusField::from_bits_truncate(
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                );
                let medium_and_unit = u16::from_be_bytes([
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                ]);
                let counter1 = Counter::from_bcd_hex_digits([
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                ])?;
                let counter2 = Counter::from_bcd_hex_digits([
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                ])?;
                Ok(UserDataBlock::FixedDataStructure {
                    identification_number,
                    access_number,
                    status,
                    medium_ad_unit: medium_and_unit,
                    counter1,
                    counter2,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_control_information() {
        assert_eq!(
            ControlInformation::from(0x50),
            Ok(ControlInformation::ResetAtApplicationLevel)
        );
        assert_eq!(
            ControlInformation::from(0x51),
            Ok(ControlInformation::SendData)
        );
        assert_eq!(
            ControlInformation::from(0x52),
            Ok(ControlInformation::SelectSlave)
        );
        assert_eq!(
            ControlInformation::from(0x54),
            Ok(ControlInformation::SynchronizeSlave)
        );
        assert_eq!(
            ControlInformation::from(0xB8),
            Ok(ControlInformation::SetBaudRate300)
        );
        assert_eq!(
            ControlInformation::from(0xB9),
            Ok(ControlInformation::SetBaudRate600)
        );
        assert_eq!(
            ControlInformation::from(0xBA),
            Ok(ControlInformation::SetBaudRate1200)
        );
        assert_eq!(
            ControlInformation::from(0xBB),
            Ok(ControlInformation::SetBaudRate2400)
        );
        assert_eq!(
            ControlInformation::from(0xBC),
            Ok(ControlInformation::SetBaudRate4800)
        );
        assert_eq!(
            ControlInformation::from(0xBD),
            Ok(ControlInformation::SetBaudRate9600)
        );
        assert_eq!(
            ControlInformation::from(0xBE),
            Ok(ControlInformation::SetBaudRate19200)
        );
        assert_eq!(
            ControlInformation::from(0xBF),
            Ok(ControlInformation::SetBaudRate38400)
        );
        assert_eq!(
            ControlInformation::from(0xB1),
            Ok(ControlInformation::OutputRAMContent)
        );
        assert_eq!(
            ControlInformation::from(0xB2),
            Ok(ControlInformation::WriteRAMContent)
        );
        assert_eq!(
            ControlInformation::from(0xB3),
            Ok(ControlInformation::StartCalibrationTestMode)
        );
        assert_eq!(
            ControlInformation::from(0xB4),
            Ok(ControlInformation::ReadEEPROM)
        );
        assert_eq!(
            ControlInformation::from(0xB6),
            Ok(ControlInformation::StartSoftwareTest)
        );
        assert_eq!(
            ControlInformation::from(0x90),
            Ok(ControlInformation::HashProcedure(0,))
        );
        assert_eq!(
            ControlInformation::from(0x91),
            Ok(ControlInformation::HashProcedure(1,))
        );
    }

    #[test]
    fn test_reset_subcode() {
        // Application layer of frame | 68 04 04 68 | 53 FE 50 | 10 | B1 16
        let data = [0x50, 0x10];
        let result = UserDataBlock::try_from(data.as_slice());
        assert_eq!(
            result,
            Ok(UserDataBlock::ResetAtApplicationLevel {
                subcode: ApplicationResetSubcode::All(0x10)
            })
        );
    }

    #[test]
    fn test_identification_number() -> Result<(), ApplicationLayerError> {
        let data = [0x78, 0x56, 0x34, 0x12];
        let result = IdentificationNumber::from_bcd_hex_digits(data)?;
        assert_eq!(result, IdentificationNumber { number: 12345678 });
        Ok(())
    }

    #[test]
    fn test_fixed_data_structure() {
        let data = [
            0x73, 0x78, 0x56, 0x34, 0x12, 0x0A, 0x00, 0xE9, 0x7E, 0x01, 0x00, 0x00, 0x00, 0x35,
            0x01, 0x00, 0x00,
        ];

        let result = UserDataBlock::try_from(data.as_slice());

        assert_eq!(
            result,
            Ok(UserDataBlock::FixedDataStructure {
                identification_number: IdentificationNumber { number: 12345678 },
                access_number: 0x0A,
                status: StatusField::from_bits_truncate(0x00),
                medium_ad_unit: 0xE97E,
                counter1: Counter { count: 1 },
                counter2: Counter { count: 135 },
            })
        );
    }

    #[test]
    fn test_manufacturer_code() -> Result<(), ApplicationLayerError> {
        let code = ManufacturerCode::from_id(0x1ee6)?;
        assert_eq!(
            code,
            ManufacturerCode {
                code: ['G', 'W', 'F']
            }
        );
        Ok(())
    }
}
