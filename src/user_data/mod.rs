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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Debug, PartialEq)]
pub struct DataRecords<'a> {
    offset: usize,
    data: &'a [u8],
    fixed_data_header: Option<&'a FixedDataHeader>,
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
                0x1F => {
                    /* TODO: parse manufacturer specific */
                    _more_records_follow = true;
                    self.offset = self.data.len();
                }
                0x2F => {
                    self.offset += 1;
                }
                _ => {
                    let record = if let Some(fixed_data_header) = self.fixed_data_header {
                        DataRecord::try_from((self.data.get(self.offset..)?, fixed_data_header))
                    } else {
                        DataRecord::try_from(self.data.get(self.offset..)?)
                    };
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
    pub const fn new(data: &'a [u8], fixed_data_header: Option<&'a FixedDataHeader>) -> Self {
        DataRecords {
            offset: 0,
            data,
            fixed_data_header,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Direction {
    SlaveToMaster,
    MasterToSlave,
}

// implement from trait for direction
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
            ControlInformation::ResponseWithVariableDataStructure { lsb_order: _ } => {
                Self::SlaveToMaster
            }
            ControlInformation::ResponseWithFixedDataStructure => Self::SlaveToMaster,
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    ResponseWithVariableDataStructure { lsb_order: bool },
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
            0x72 | 0x76 => Ok(Self::ResponseWithVariableDataStructure {
                lsb_order: byte & 0x04 != 0,
            }),
            0x73 | 0x77 => Ok(Self::ResponseWithFixedDataStructure),
            _ => Err(ApplicationLayerError::InvalidControlInformation { byte }),
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FixedDataHeader {
    pub identification_number: IdentificationNumber,
    pub manufacturer: Result<ManufacturerCode, ApplicationLayerError>,
    pub version: u8,
    pub medium: Medium,
    pub access_number: u8,
    pub status: StatusField,
    pub signature: u16,
    pub lsb_order: bool,
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
            ControlInformation::ResponseWithVariableDataStructure { lsb_order } => {
                let mut iter = data.iter().skip(1);
                let mut identification_number_bytes = [
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                ];
                if lsb_order {
                    identification_number_bytes.reverse();
                }

                Ok(UserDataBlock::VariableDataStructure {
                    fixed_data_header: FixedDataHeader {
                        identification_number: IdentificationNumber::from_bcd_hex_digits(
                            identification_number_bytes,
                        )?,
                        manufacturer: ManufacturerCode::from_id(u16::from_le_bytes([
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ])),
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
                        lsb_order,
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

#[cfg(all(test, feature = "std"))]
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

    #[test]
    fn test_lsb_frame() {
        use crate::frames::Frame;
        use crate::user_data::data_information::DataType;

        let lsb_frame: &[u8] = &[
            0x68, 0x64, 0x64, 0x68, 0x8, 0x7f, 0x76, 0x9, 0x67, 0x1, 0x6, 0x0, 0x0, 0x51, 0x4,
            0x50, 0x0, 0x0, 0x0, 0x2, 0x6c, 0x38, 0x1c, 0xc, 0xf, 0x0, 0x80, 0x87, 0x32, 0x8c,
            0x20, 0xf, 0x0, 0x0, 0x0, 0x0, 0xc, 0x14, 0x13, 0x32, 0x82, 0x58, 0xbc, 0x10, 0x15,
            0x0, 0x25, 0x81, 0x25, 0x8c, 0x20, 0x13, 0x0, 0x0, 0x0, 0x0, 0x8c, 0x30, 0x13, 0x0,
            0x0, 0x1, 0x61, 0x8c, 0x40, 0x13, 0x0, 0x0, 0x16, 0x88, 0xa, 0x3c, 0x1, 0x10, 0xa,
            0x2d, 0x0, 0x80, 0xa, 0x5a, 0x7, 0x18, 0xa, 0x5e, 0x6, 0x53, 0xc, 0x22, 0x0, 0x16, 0x7,
            0x26, 0x3c, 0x22, 0x0, 0x0, 0x33, 0x81, 0x4, 0x7e, 0x0, 0x0, 0x67, 0xc, 0xc, 0x16,
        ];
        let non_lsb_frame: &[u8] = &[
            0x68, 0xc7, 0xc7, 0x68, 0x8, 0x38, 0x72, 0x56, 0x73, 0x23, 0x72, 0x2d, 0x2c, 0x34, 0x4,
            0x87, 0x0, 0x0, 0x0, 0x4, 0xf, 0x7f, 0x1c, 0x1, 0x0, 0x4, 0xff, 0x7, 0x8a, 0xad, 0x8,
            0x0, 0x4, 0xff, 0x8, 0x6, 0xfe, 0x5, 0x0, 0x4, 0x14, 0x4e, 0x55, 0xb, 0x0, 0x84, 0x40,
            0x14, 0x0, 0x0, 0x0, 0x0, 0x84, 0x80, 0x40, 0x14, 0x0, 0x0, 0x0, 0x0, 0x4, 0x22, 0x76,
            0x7f, 0x0, 0x0, 0x34, 0x22, 0x8b, 0x2c, 0x0, 0x0, 0x2, 0x59, 0x61, 0x1b, 0x2, 0x5d,
            0x5f, 0x10, 0x2, 0x61, 0x2, 0xb, 0x4, 0x2d, 0x55, 0x0, 0x0, 0x0, 0x14, 0x2d, 0x83, 0x0,
            0x0, 0x0, 0x4, 0x3b, 0x6, 0x1, 0x0, 0x0, 0x14, 0x3b, 0xaa, 0x1, 0x0, 0x0, 0x4, 0xff,
            0x22, 0x0, 0x0, 0x0, 0x0, 0x4, 0x6d, 0x6, 0x2c, 0x1f, 0x3a, 0x44, 0xf, 0xcf, 0x11, 0x1,
            0x0, 0x44, 0xff, 0x7, 0xb, 0x69, 0x8, 0x0, 0x44, 0xff, 0x8, 0x54, 0xd3, 0x5, 0x0, 0x44,
            0x14, 0x11, 0xf3, 0xa, 0x0, 0xc4, 0x40, 0x14, 0x0, 0x0, 0x0, 0x0, 0xc4, 0x80, 0x40,
            0x14, 0x0, 0x0, 0x0, 0x0, 0x54, 0x2d, 0x3a, 0x0, 0x0, 0x0, 0x54, 0x3b, 0x28, 0x1, 0x0,
            0x0, 0x42, 0x6c, 0x1, 0x3a, 0x2, 0xff, 0x1a, 0x1, 0x1a, 0xc, 0x78, 0x56, 0x73, 0x23,
            0x72, 0x4, 0xff, 0x16, 0xe6, 0x84, 0x1e, 0x0, 0x4, 0xff, 0x17, 0xc1, 0xd5, 0xb4, 0x0,
            0x12, 0x16,
        ];

        let frames = [
            (lsb_frame, 9670106, Some(DataType::Number(808732.0))),
            (non_lsb_frame, 72237356, Some(DataType::Number(568714.0))),
        ];

        for (frame, expected_iden_nr, data_record_value) in frames {
            let frame = Frame::try_from(frame).unwrap();

            if let Frame::LongFrame {
                function: _,
                address: _,
                data,
            } = frame
            {
                let user_data_block = UserDataBlock::try_from(data).unwrap();
                if let UserDataBlock::VariableDataStructure {
                    fixed_data_header,
                    variable_data_block,
                } = user_data_block
                {
                    assert_eq!(
                        fixed_data_header.identification_number.number,
                        expected_iden_nr
                    );

                    let mut data_records =
                        DataRecords::try_from((variable_data_block, &fixed_data_header))
                            .unwrap()
                            .flatten();
                    data_records.next().unwrap();
                    assert_eq!(data_records.next().unwrap().data.value, data_record_value);
                } else {
                    panic!("UserDataBlock is not a variable data structure");
                }
            } else {
                panic!("Frame is not a long frame");
            }
        }
    }

    #[test]
    fn test_manufacturer_specific_data() {
        use crate::frames::Frame;
        use crate::user_data::data_information::DataType;

        let manufacturer_specific_data_frame: &[u8] = &[
            0x68, 0x55, 0x55, 0x68, 0x8, 0x1e, 0x72, 0x34, 0x35, 0x58, 0x12, 0x92, 0x26, 0x18, 0x4,
            0x14, 0x0, 0x0, 0x0, 0xc, 0x78, 0x34, 0x35, 0x58, 0x12, 0x4, 0xe, 0x57, 0x64, 0x3, 0x0,
            0xc, 0x14, 0x73, 0x58, 0x44, 0x0, 0xb, 0x2d, 0x6, 0x0, 0x0, 0xb, 0x3b, 0x55, 0x0, 0x0,
            0xa, 0x5a, 0x87, 0x6, 0xa, 0x5e, 0x77, 0x5, 0xb, 0x61, 0x1, 0x11, 0x0, 0x4, 0x6d, 0x10,
            0x2, 0x4, 0x3c, 0x2, 0x27, 0x79, 0x11, 0x9, 0xfd, 0xe, 0x6, 0x9, 0xfd, 0xf, 0x6, 0x8c,
            0xc0, 0x0, 0x15, 0x71, 0x25, 0x0, 0x0, 0xf, 0x0, 0x0, 0x86, 0x16,
        ];

        let frame = Frame::try_from(manufacturer_specific_data_frame).unwrap();

        if let Frame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            let user_data_block = UserDataBlock::try_from(data).unwrap();
            if let UserDataBlock::VariableDataStructure {
                fixed_data_header,
                variable_data_block,
            } = user_data_block
            {
                let mut data_records: Vec<_> =
                    DataRecords::try_from((variable_data_block, &fixed_data_header))
                        .unwrap()
                        .flatten()
                        .collect();

                assert_eq!(data_records.len(), 14);

                assert_eq!(
                    data_records.pop().unwrap().data.value,
                    Some(DataType::ManufacturerSpecific(&[15, 0, 0]))
                );
                assert_eq!(
                    data_records.pop().unwrap().data.value,
                    Some(DataType::Number(2571.0))
                );
            }
        }
    }

    #[test]
    fn real32bit() {
        use crate::frames::Frame;
        use crate::user_data::data_information::DataType;
        use crate::user_data::value_information::ValueLabel;

        let real32bit: &[u8] = &[
            0x68, 0xa7, 0xa7, 0x68, 0x8, 0x4d, 0x72, 0x82, 0x4, 0x75, 0x30, 0xee, 0x4d, 0x19, 0x4,
            0xc2, 0x0, 0x0, 0x0, 0x4, 0xe, 0x1b, 0xe, 0x0, 0x0, 0x84, 0xa, 0xe, 0x4c, 0x6, 0x0,
            0x0, 0x4, 0x13, 0x7, 0x81, 0x0, 0x0, 0x84, 0xa, 0x13, 0x9d, 0x37, 0x0, 0x0, 0xb, 0xfd,
            0xf, 0x0, 0x7, 0x1, 0xa, 0xfd, 0xd, 0x0, 0x11, 0x8c, 0x40, 0x79, 0x1, 0x0, 0x0, 0x0,
            0x84, 0x40, 0x14, 0x31, 0x5, 0x0, 0x0, 0x84, 0x4a, 0x14, 0xfd, 0x4, 0x0, 0x0, 0x8c,
            0x80, 0x40, 0x79, 0x2, 0x0, 0x0, 0x0, 0x84, 0x80, 0x40, 0x14, 0x27, 0x50, 0x0, 0x0,
            0x84, 0x8a, 0x40, 0x14, 0x8, 0x31, 0x0, 0x0, 0x5, 0xff, 0x1, 0xdf, 0xa3, 0xb1, 0x3e,
            0x5, 0xff, 0x2, 0xa8, 0x59, 0x6b, 0x3f, 0xc, 0x78, 0x82, 0x4, 0x75, 0x30, 0x4, 0x6d,
            0x5, 0xb, 0x2f, 0x31, 0x82, 0xa, 0x6c, 0xe1, 0xf1, 0x5, 0x5b, 0x40, 0x7a, 0x63, 0x42,
            0x5, 0x5f, 0x80, 0xc3, 0x25, 0x42, 0x5, 0x3e, 0x0, 0x0, 0x0, 0x0, 0x5, 0x2b, 0x0, 0x0,
            0x0, 0x0, 0x1, 0xff, 0x2b, 0x0, 0x3, 0x22, 0x17, 0x3b, 0x0, 0x2, 0xff, 0x2c, 0x0, 0x0,
            0x1f, 0xa4, 0x16,
        ];

        let frame = Frame::try_from(real32bit).unwrap();

        if let Frame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            let user_data_block = UserDataBlock::try_from(data).unwrap();
            if let UserDataBlock::VariableDataStructure {
                fixed_data_header,
                variable_data_block,
            } = user_data_block
            {
                let mut data_records: Vec<DataRecord> =
                    DataRecords::try_from((variable_data_block, &fixed_data_header))
                        .unwrap()
                        .flatten()
                        .collect();

                assert_eq!(data_records.len(), 24);

                for data_record in data_records {
                    let labels = data_record
                        .data_record_header
                        .processed_data_record_header
                        .value_information
                        .as_ref()
                        .unwrap()
                        .labels
                        .clone();
                    if labels.contains(&ValueLabel::ReturnTemperature) {
                        assert_eq!(
                            data_record.data.value,
                            Some(DataType::Number(41.44091796875))
                        );
                    }
                    if labels.contains(&ValueLabel::FlowTemperature) {
                        assert_eq!(
                            data_record.data.value,
                            Some(DataType::Number(56.869384765625))
                        );
                    }
                }
            }
        }
    }
}
