//! Parser for the M-Bus application layer defined by EN 13757-3.
//!
//! Use [`parse_application_layer`] when the input starts with a control
//! information (CI) field. Use [`parse_data_records`] when the transport
//! header has already been removed and the input starts with a DIF/VIF data
//! record.
//!
//! # Parse data records directly
//!
//! ```rust
//! use m_bus_application_layer::{
//!     data_information::DataType, parse_data_records, DataRecordError,
//! };
//!
//! fn main() -> Result<(), DataRecordError> {
//!     // 24-bit integer, volume in m³ with a 10^-3 scale.
//!     let data = [0x03, 0x13, 0x15, 0x31, 0x00];
//!     let mut records = parse_data_records(&data);
//!     let record = records.next().expect("one data record")?;
//!
//!     assert_eq!(record.value(), Some(&DataType::Number(12_565.0)));
//!     assert_eq!(record.raw_bytes(), &data);
//!     Ok(())
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use std::fmt;

pub use m_bus_core::decryption;

use m_bus_core::{
    bcd_hex_digits_to_u32, ConfigurationField, DeviceType, IdentificationNumber, ManufacturerCode,
};
pub use variable_user_data::DataRecordError;

pub use self::data_record::DataRecord;
#[cfg(feature = "decryption")]
use m_bus_core::decryption::DecryptionError::{NotEncrypted, UnknownEncryptionState};
pub use m_bus_core::ApplicationLayerError;

pub mod data_information;
pub mod data_record;
pub mod extended_link_layer;
pub mod value_information;
pub mod variable_user_data;

use extended_link_layer::ExtendedLinkLayer;

/// Parses an application-layer user data block beginning with a CI field.
pub fn parse_application_layer(data: &[u8]) -> Result<UserDataBlock<'_>, ApplicationLayerError> {
    UserDataBlock::try_from(data)
}

/// Parses DIF/VIF data records after any CI and transport headers were removed.
///
/// The returned iterator yields one result per record and does not allocate.
#[must_use]
pub const fn parse_data_records(data: &[u8]) -> DataRecords<'_> {
    DataRecords::new(data, None)
}

/// Parses DIF/VIF data records using context from a long transport header.
///
/// Header context is needed for data encodings whose interpretation depends on
/// transport-layer metadata, such as two-digit years.
#[must_use]
pub const fn parse_data_records_with_header<'a>(
    data: &'a [u8],
    header: &'a LongTplHeader,
) -> DataRecords<'a> {
    DataRecords::new(data, Some(header))
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "Vec<DataRecord>"))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Debug, PartialEq)]
pub struct DataRecords<'a> {
    offset: usize,
    data: &'a [u8],
    long_tpl_header: Option<&'a LongTplHeader>,
}

#[cfg(feature = "std")]
impl<'a> From<DataRecords<'a>> for Vec<DataRecord<'a>> {
    fn from(value: DataRecords<'a>) -> Self {
        let value: Result<Vec<_>, _> = value.collect();
        value.unwrap_or_default()
    }
}

impl<'a> Iterator for DataRecords<'a> {
    type Item = Result<DataRecord<'a>, DataRecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset < self.data.len() {
            let dif = data_information::DataInformationField::from(*self.data.get(self.offset)?);

            if dif.is_special_function() {
                match dif.special_function() {
                    data_information::SpecialFunctions::IdleFiller => {
                        self.offset += 1;
                    }
                    data_information::SpecialFunctions::ManufacturerSpecific
                    | data_information::SpecialFunctions::MoreRecordsFollow => {
                        let remaining = self.data.get(self.offset..)?;
                        self.offset = self.data.len();
                        let record = if let Some(long_tpl_header) = self.long_tpl_header {
                            DataRecord::try_from((remaining, long_tpl_header))
                        } else {
                            DataRecord::try_from(remaining)
                        };
                        return Some(record);
                    }
                    data_information::SpecialFunctions::GlobalReadoutRequest => {
                        let remaining = self.data.get(self.offset..)?;
                        self.offset += 1;
                        let record = if let Some(long_tpl_header) = self.long_tpl_header {
                            DataRecord::try_from((remaining, long_tpl_header))
                        } else {
                            DataRecord::try_from(remaining)
                        };
                        return Some(record);
                    }
                    data_information::SpecialFunctions::Reserved => {
                        self.offset += 1;
                    }
                }
            } else {
                let record = if let Some(long_tpl_header) = self.long_tpl_header {
                    DataRecord::try_from((self.data.get(self.offset..)?, long_tpl_header))
                } else {
                    DataRecord::try_from(self.data.get(self.offset..)?)
                };
                match record {
                    Ok(record) => {
                        self.offset += record.get_size();
                        return Some(Ok(record));
                    }
                    Err(error) => {
                        self.offset = self.data.len();
                        return Some(Err(error));
                    }
                }
            }
        }
        None
    }
}

impl<'a> DataRecords<'a> {
    #[must_use]
    pub const fn new(data: &'a [u8], long_tpl_header: Option<&'a LongTplHeader>) -> Self {
        DataRecords {
            offset: 0,
            data,
            long_tpl_header,
        }
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

#[cfg(feature = "defmt")]
impl defmt::Format for StatusField {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{:?}", self);
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
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
            ControlInformation::DataSentWithShortTransportLayer => Self::MasterToSlave,
            ControlInformation::DataSentWithLongTransportLayer => Self::MasterToSlave,
            ControlInformation::CosemDataWithLongTransportLayer => Self::MasterToSlave,
            ControlInformation::CosemDataWithShortTransportLayer => Self::MasterToSlave,
            ControlInformation::ObisDataReservedLongTransportLayer => Self::MasterToSlave,
            ControlInformation::ObisDataReservedShortTransportLayer => Self::MasterToSlave,
            ControlInformation::ApplicationLayerFormatFrameNoTransport => Self::MasterToSlave,
            ControlInformation::ApplicationLayerFormatFrameShortTransport => Self::MasterToSlave,
            ControlInformation::ApplicationLayerFormatFrameLongTransport => Self::MasterToSlave,
            ControlInformation::ClockSyncAbsolute => Self::MasterToSlave,
            ControlInformation::ClockSyncRelative => Self::MasterToSlave,
            ControlInformation::ApplicationErrorShortTransport => Self::SlaveToMaster,
            ControlInformation::ApplicationErrorLongTransport => Self::SlaveToMaster,
            ControlInformation::AlarmShortTransport => Self::SlaveToMaster,
            ControlInformation::AlarmLongTransport => Self::SlaveToMaster,
            ControlInformation::ApplicationLayerNoTransport => Self::SlaveToMaster,
            ControlInformation::ApplicationLayerCompactFrameNoTransport => Self::SlaveToMaster,
            ControlInformation::ApplicationLayerShortTransport => Self::SlaveToMaster,
            ControlInformation::ApplicationLayerCompactFrameShortTransport => Self::SlaveToMaster,
            ControlInformation::CosemApplicationLayerLongTransport => Self::SlaveToMaster,
            ControlInformation::CosemApplicationLayerShortTransport => Self::SlaveToMaster,
            ControlInformation::ObisApplicationLayerReservedLongTransport => Self::SlaveToMaster,
            ControlInformation::ObisApplicationLayerReservedShortTransport => Self::SlaveToMaster,
            ControlInformation::TransportLayerLongReadoutToMeter => Self::MasterToSlave,
            ControlInformation::NetworkLayerData => Self::MasterToSlave,
            ControlInformation::FutureUse => Self::MasterToSlave,
            ControlInformation::NetworkManagementApplication => Self::MasterToSlave,
            ControlInformation::TransportLayerCompactFrame => Self::MasterToSlave,
            ControlInformation::TransportLayerFormatFrame => Self::MasterToSlave,
            ControlInformation::NetworkManagementDataReserved => Self::MasterToSlave,
            ControlInformation::TransportLayerShortMeterToReadout => Self::SlaveToMaster,
            ControlInformation::TransportLayerLongMeterToReadout => Self::SlaveToMaster,
            ControlInformation::ExtendedLinkLayerI => Self::SlaveToMaster,
            ControlInformation::ExtendedLinkLayerII => Self::SlaveToMaster,
            ControlInformation::ExtendedLinkLayerIII => Self::SlaveToMaster,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
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
    // Wireless M-Bus CI values
    DataSentWithShortTransportLayer,
    DataSentWithLongTransportLayer,
    CosemDataWithLongTransportLayer,
    CosemDataWithShortTransportLayer,
    ObisDataReservedLongTransportLayer,
    ObisDataReservedShortTransportLayer,
    ApplicationLayerFormatFrameNoTransport,
    ApplicationLayerFormatFrameShortTransport,
    ApplicationLayerFormatFrameLongTransport,
    ClockSyncAbsolute,
    ClockSyncRelative,
    ApplicationErrorShortTransport,
    ApplicationErrorLongTransport,
    AlarmShortTransport,
    AlarmLongTransport,
    ApplicationLayerNoTransport,
    ApplicationLayerCompactFrameNoTransport,
    ApplicationLayerShortTransport,
    ApplicationLayerCompactFrameShortTransport,
    CosemApplicationLayerLongTransport,
    CosemApplicationLayerShortTransport,
    ObisApplicationLayerReservedLongTransport,
    ObisApplicationLayerReservedShortTransport,
    TransportLayerLongReadoutToMeter,
    NetworkLayerData,
    FutureUse,
    NetworkManagementApplication,
    TransportLayerCompactFrame,
    TransportLayerFormatFrame,
    NetworkManagementDataReserved,
    TransportLayerShortMeterToReadout,
    TransportLayerLongMeterToReadout,
    ExtendedLinkLayerI,
    ExtendedLinkLayerII,
    ExtendedLinkLayerIII,
}

impl ControlInformation {
    const fn from(byte: u8) -> Result<Self, ApplicationLayerError> {
        match byte {
            0x50 => Ok(Self::ResetAtApplicationLevel),
            0x51 => Ok(Self::SendData),
            0x52 => Ok(Self::SelectSlave),
            0x54 => Ok(Self::SynchronizeSlave),
            0x5A => Ok(Self::DataSentWithShortTransportLayer),
            0x5B => Ok(Self::DataSentWithLongTransportLayer),
            0x60 => Ok(Self::CosemDataWithLongTransportLayer),
            0x61 => Ok(Self::CosemDataWithShortTransportLayer),
            0x64 => Ok(Self::ObisDataReservedLongTransportLayer),
            0x65 => Ok(Self::ObisDataReservedShortTransportLayer),
            0x69 => Ok(Self::ApplicationLayerFormatFrameNoTransport),
            0x6A => Ok(Self::ApplicationLayerFormatFrameShortTransport),
            0x6B => Ok(Self::ApplicationLayerFormatFrameLongTransport),
            0x6C => Ok(Self::ClockSyncAbsolute),
            0x6D => Ok(Self::ClockSyncRelative),
            0x6E => Ok(Self::ApplicationErrorShortTransport),
            0x6F => Ok(Self::ApplicationErrorLongTransport),
            0x70 => Ok(Self::SendErrorStatus),
            0x71 => Ok(Self::SendAlarmStatus),
            0x72 | 0x76 => Ok(Self::ResponseWithVariableDataStructure {
                lsb_order: byte & 0x04 != 0,
            }),
            0x73 | 0x77 => Ok(Self::ResponseWithFixedDataStructure),
            0x74 => Ok(Self::AlarmShortTransport),
            0x75 => Ok(Self::AlarmLongTransport),
            0x78 => Ok(Self::ApplicationLayerNoTransport),
            0x79 => Ok(Self::ApplicationLayerCompactFrameNoTransport),
            0x7A => Ok(Self::ApplicationLayerShortTransport),
            0x7B => Ok(Self::ApplicationLayerCompactFrameShortTransport),
            0x7C => Ok(Self::CosemApplicationLayerLongTransport),
            0x7D => Ok(Self::CosemApplicationLayerShortTransport),
            0x7E => Ok(Self::ObisApplicationLayerReservedLongTransport),
            0x7F => Ok(Self::ObisApplicationLayerReservedShortTransport),
            0x80 => Ok(Self::TransportLayerLongReadoutToMeter),
            0x81 => Ok(Self::NetworkLayerData),
            0x82 => Ok(Self::FutureUse),
            0x83 => Ok(Self::NetworkManagementApplication),
            0x84 => Ok(Self::TransportLayerCompactFrame),
            0x85 => Ok(Self::TransportLayerFormatFrame),
            0x89 => Ok(Self::NetworkManagementDataReserved),
            0x8A => Ok(Self::TransportLayerShortMeterToReadout),
            0x8B => Ok(Self::TransportLayerLongMeterToReadout),
            0x8C => Ok(Self::ExtendedLinkLayerI),
            0x8D => Ok(Self::ExtendedLinkLayerII),
            0x8E => Ok(Self::ExtendedLinkLayerIII),
            0x90..=0x97 => Ok(Self::HashProcedure(byte - 0x90)),
            // Encrypted CI codes (0xA0-0xAF) - these are encrypted variants of 0x7A (ApplicationLayerShortTransport)
            // When used with NOKEY, they should be parsed as unencrypted ApplicationLayerShortTransport
            // The lower 4 bits indicate the encryption mode:
            //   0xA0 = Mode 5 (AES-CBC-IV zero) or NOKEY
            //   0xA2 = Mode 7 (AES-CBC-IV non-zero) or NOKEY
            //   0xA4, 0xA6, etc. = other modes
            0xA0..=0xAF => Ok(Self::ApplicationLayerShortTransport),
            0xB1 => Ok(Self::OutputRAMContent),
            0xB2 => Ok(Self::WriteRAMContent),
            0xB3 => Ok(Self::StartCalibrationTestMode),
            0xB4 => Ok(Self::ReadEEPROM),
            0xB6 => Ok(Self::StartSoftwareTest),
            0xB8 => Ok(Self::SetBaudRate300),
            0xB9 => Ok(Self::SetBaudRate600),
            0xBA => Ok(Self::SetBaudRate1200),
            0xBB => Ok(Self::SetBaudRate2400),
            0xBC => Ok(Self::SetBaudRate4800),
            0xBD => Ok(Self::SetBaudRate9600),
            0xBE => Ok(Self::SetBaudRate19200),
            0xBF => Ok(Self::SetBaudRate38400),
            _ => Err(ApplicationLayerError::InvalidControlInformation { byte }),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
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

impl Counter {
    pub fn from_bcd_hex_digits(digits: [u8; 4]) -> Result<Self, ApplicationLayerError> {
        let count = bcd_hex_digits_to_u32(digits)?;
        Ok(Self { count })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum UserDataBlock<'a> {
    ResetAtApplicationLevel {
        subcode: ApplicationResetSubcode,
    },
    FixedDataStructure {
        identification_number: IdentificationNumber,
        access_number: u8,
        status: StatusField,
        device_type_and_unit: u16,
        counter1: Counter,
        counter2: Counter,
    },
    VariableDataStructureWithLongTplHeader {
        extended_link_layer: Option<ExtendedLinkLayer>,
        long_tpl_header: LongTplHeader,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        variable_data_block: &'a [u8],
    },

    VariableDataStructureWithShortTplHeader {
        extended_link_layer: Option<ExtendedLinkLayer>,
        short_tpl_header: ShortTplHeader,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        variable_data_block: &'a [u8],
    },

    VariableDataStructureWithoutTplHeader {
        extended_link_layer: Option<ExtendedLinkLayer>,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        variable_data_block: &'a [u8],
    },
}

impl<'a> UserDataBlock<'a> {
    /// Returns an iterator over the variable data records in this block.
    ///
    /// Fixed data structures, control messages, and encrypted payloads do not
    /// expose records. Decrypt an encrypted payload before parsing its records.
    #[must_use]
    pub fn data_records(&self) -> Option<DataRecords<'_>> {
        match self {
            Self::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                variable_data_block,
                ..
            } if !long_tpl_header.is_encrypted() => Some(parse_data_records_with_header(
                variable_data_block,
                long_tpl_header,
            )),
            Self::VariableDataStructureWithShortTplHeader {
                short_tpl_header,
                variable_data_block,
                ..
            } if !short_tpl_header.is_encrypted() => Some(parse_data_records(variable_data_block)),
            Self::VariableDataStructureWithoutTplHeader {
                variable_data_block,
                ..
            } => Some(parse_data_records(variable_data_block)),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_encrypted(&self) -> Option<bool> {
        match self {
            Self::VariableDataStructureWithLongTplHeader {
                long_tpl_header, ..
            } => Some(long_tpl_header.is_encrypted()),
            _ => None,
        }
    }

    /// Returns the length of the variable data block (encrypted payload size)
    #[must_use]
    pub fn variable_data_len(&self) -> usize {
        match self {
            Self::VariableDataStructureWithLongTplHeader {
                variable_data_block,
                ..
            } => variable_data_block.len(),
            Self::VariableDataStructureWithShortTplHeader {
                variable_data_block,
                ..
            } => variable_data_block.len(),
            Self::VariableDataStructureWithoutTplHeader {
                variable_data_block,
                ..
            } => variable_data_block.len(),
            _ => 0,
        }
    }

    #[cfg(feature = "decryption")]
    pub fn decrypt_variable_data<K: crate::decryption::KeyProvider>(
        &self,
        provider: &K,
        output: &mut [u8],
    ) -> Result<usize, crate::decryption::DecryptionError> {
        use crate::decryption::{DecryptionError, EncryptedPayload, KeyContext};

        match self {
            Self::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                variable_data_block,
                ..
            } => {
                if !long_tpl_header.is_encrypted() {
                    return Err(NotEncrypted);
                }

                let security_mode = long_tpl_header
                    .short_tpl_header
                    .configuration_field
                    .security_mode();

                let manufacturer = long_tpl_header
                    .manufacturer
                    .map_err(|_| DecryptionError::DecryptionFailed)?;

                let context = KeyContext {
                    manufacturer,
                    identification_number: long_tpl_header.identification_number.number,
                    version: long_tpl_header.version,
                    device_type: long_tpl_header.device_type,
                    security_mode,
                    access_number: long_tpl_header.short_tpl_header.access_number,
                };

                let payload = EncryptedPayload::new(variable_data_block, context);
                payload.decrypt_into(provider, output)
            }
            Self::VariableDataStructureWithShortTplHeader {
                short_tpl_header, ..
            } => {
                if !short_tpl_header.is_encrypted() {
                    Err(NotEncrypted)
                } else {
                    // Short TPL header doesn't contain manufacturer info,
                    // use decrypt_variable_data_with_context() instead
                    Err(UnknownEncryptionState)
                }
            }
            _ => Err(DecryptionError::UnknownEncryptionState),
        }
    }

    /// Decrypt variable data when manufacturer info is not available in the TPL header.
    /// Use this for frames with Short TPL header where manufacturer info comes from the link layer.
    #[cfg(feature = "decryption")]
    pub fn decrypt_variable_data_with_context<K: crate::decryption::KeyProvider>(
        &self,
        provider: &K,
        manufacturer: ManufacturerCode,
        identification_number: u32,
        version: u8,
        device_type: DeviceType,
        output: &mut [u8],
    ) -> Result<usize, crate::decryption::DecryptionError> {
        use crate::decryption::{DecryptionError, EncryptedPayload, KeyContext};

        match self {
            Self::VariableDataStructureWithShortTplHeader {
                short_tpl_header,
                variable_data_block,
                ..
            } => {
                if !short_tpl_header.is_encrypted() {
                    return Err(NotEncrypted);
                }

                let security_mode = short_tpl_header.configuration_field.security_mode();

                let context = KeyContext {
                    manufacturer,
                    identification_number,
                    version,
                    device_type,
                    security_mode,
                    access_number: short_tpl_header.access_number,
                };

                let payload = EncryptedPayload::new(variable_data_block, context);
                payload.decrypt_into(provider, output)
            }
            Self::VariableDataStructureWithLongTplHeader { .. } => {
                // Long TPL header has its own manufacturer info, use decrypt_variable_data() instead
                self.decrypt_variable_data(provider, output)
            }
            _ => Err(DecryptionError::UnknownEncryptionState),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LongTplHeader {
    pub identification_number: IdentificationNumber,
    #[cfg_attr(
        feature = "serde",
        serde(skip_deserializing, default = "default_manufacturer_result")
    )]
    pub manufacturer: Result<ManufacturerCode, ApplicationLayerError>,
    pub version: u8,
    pub device_type: DeviceType,
    pub short_tpl_header: ShortTplHeader,
    pub lsb_order: bool,
}

#[cfg(feature = "serde")]
fn default_manufacturer_result() -> Result<ManufacturerCode, ApplicationLayerError> {
    Err(ApplicationLayerError::InsufficientData)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ShortTplHeader {
    pub access_number: u8,
    pub status: StatusField,
    pub configuration_field: ConfigurationField,
}

impl LongTplHeader {
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        use m_bus_core::SecurityMode;
        !matches!(
            self.short_tpl_header.configuration_field.security_mode(),
            SecurityMode::NoEncryption
        )
    }
}

impl ShortTplHeader {
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        use m_bus_core::SecurityMode;
        !matches!(
            self.configuration_field.security_mode(),
            SecurityMode::NoEncryption
        )
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
            ControlInformation::SendData => Err(ApplicationLayerError::Unimplemented {
                feature: "SendData control information",
            }),
            ControlInformation::SelectSlave => Err(ApplicationLayerError::Unimplemented {
                feature: "SelectSlave control information",
            }),
            ControlInformation::SynchronizeSlave => Err(ApplicationLayerError::Unimplemented {
                feature: "SynchronizeSlave control information",
            }),
            ControlInformation::SetBaudRate300 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate300 control information",
            }),
            ControlInformation::SetBaudRate600 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate600 control information",
            }),
            ControlInformation::SetBaudRate1200 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate1200 control information",
            }),
            ControlInformation::SetBaudRate2400 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate2400 control information",
            }),
            ControlInformation::SetBaudRate4800 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate4800 control information",
            }),
            ControlInformation::SetBaudRate9600 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate9600 control information",
            }),
            ControlInformation::SetBaudRate19200 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate19200 control information",
            }),
            ControlInformation::SetBaudRate38400 => Err(ApplicationLayerError::Unimplemented {
                feature: "SetBaudRate38400 control information",
            }),
            ControlInformation::OutputRAMContent => Err(ApplicationLayerError::Unimplemented {
                feature: "OutputRAMContent control information",
            }),
            ControlInformation::WriteRAMContent => Err(ApplicationLayerError::Unimplemented {
                feature: "WriteRAMContent control information",
            }),
            ControlInformation::StartCalibrationTestMode => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "StartCalibrationTestMode control information",
                })
            }
            ControlInformation::ReadEEPROM => Err(ApplicationLayerError::Unimplemented {
                feature: "ReadEEPROM control information",
            }),
            ControlInformation::StartSoftwareTest => Err(ApplicationLayerError::Unimplemented {
                feature: "StartSoftwareTest control information",
            }),
            ControlInformation::HashProcedure(_) => Err(ApplicationLayerError::Unimplemented {
                feature: "HashProcedure control information",
            }),
            ControlInformation::SendErrorStatus => Err(ApplicationLayerError::Unimplemented {
                feature: "SendErrorStatus control information",
            }),
            ControlInformation::SendAlarmStatus => Err(ApplicationLayerError::Unimplemented {
                feature: "SendAlarmStatus control information",
            }),
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

                Ok(UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header: LongTplHeader {
                        identification_number: IdentificationNumber::from_bcd_hex_digits(
                            identification_number_bytes,
                        )?,
                        manufacturer: ManufacturerCode::from_id(u16::from_le_bytes([
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ])),
                        version: *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        device_type: DeviceType::from(
                            *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                        ),
                        short_tpl_header: ShortTplHeader {
                            access_number: *iter
                                .next()
                                .ok_or(ApplicationLayerError::InsufficientData)?,
                            status: {
                                StatusField::from_bits_truncate(
                                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                                )
                            },
                            configuration_field: {
                                ConfigurationField::from_bytes(
                                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                                    *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                                )
                            },
                        },
                        lsb_order,
                    },
                    variable_data_block: data
                        .get(13..data.len())
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                    extended_link_layer: None,
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
                let device_type_and_unit = u16::from_be_bytes([
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
                    device_type_and_unit,
                    counter1,
                    counter2,
                })
            }
            ControlInformation::DataSentWithShortTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "DataSentWithShortTransportLayer control information",
                })
            }
            ControlInformation::DataSentWithLongTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "DataSentWithLongTransportLayer control information",
                })
            }
            ControlInformation::CosemDataWithLongTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "CosemDataWithLongTransportLayer control information",
                })
            }
            ControlInformation::CosemDataWithShortTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "CosemDataWithShortTransportLayer control information",
                })
            }
            ControlInformation::ObisDataReservedLongTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ObisDataReservedLongTransportLayer control information",
                })
            }
            ControlInformation::ObisDataReservedShortTransportLayer => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ObisDataReservedShortTransportLayer control information",
                })
            }
            ControlInformation::ApplicationLayerFormatFrameNoTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerFormatFrameNoTransport control information",
                })
            }
            ControlInformation::ApplicationLayerFormatFrameShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerFormatFrameShortTransport control information",
                })
            }
            ControlInformation::ApplicationLayerFormatFrameLongTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerFormatFrameLongTransport control information",
                })
            }
            ControlInformation::ClockSyncAbsolute => Err(ApplicationLayerError::Unimplemented {
                feature: "ClockSyncAbsolute control information",
            }),
            ControlInformation::ClockSyncRelative => Err(ApplicationLayerError::Unimplemented {
                feature: "ClockSyncRelative control information",
            }),
            ControlInformation::ApplicationErrorShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationErrorShortTransport control information",
                })
            }
            ControlInformation::ApplicationErrorLongTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationErrorLongTransport control information",
                })
            }
            ControlInformation::AlarmShortTransport => Err(ApplicationLayerError::Unimplemented {
                feature: "AlarmShortTransport control information",
            }),
            ControlInformation::AlarmLongTransport => Err(ApplicationLayerError::Unimplemented {
                feature: "AlarmLongTransport control information",
            }),
            ControlInformation::ApplicationLayerNoTransport => {
                Ok(UserDataBlock::VariableDataStructureWithoutTplHeader {
                    extended_link_layer: None,
                    variable_data_block: data
                        .get(1..data.len())
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                })
            }
            ControlInformation::ApplicationLayerCompactFrameNoTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerCompactFrameNoTransport control information",
                })
            }
            ControlInformation::ApplicationLayerShortTransport => {
                // CI=0xA0 (encryption mode 5) has an additional encryption configuration byte after CI
                // Other encrypted CI codes (0xA2, 0xA4, etc.) do not have this byte
                let has_encryption_config_byte = data[0] == 0xA0;
                let skip_count = if has_encryption_config_byte { 2 } else { 1 };
                let data_block_offset = if has_encryption_config_byte { 6 } else { 5 };

                let mut iter = data.iter().skip(skip_count);

                Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                    short_tpl_header: ShortTplHeader {
                        access_number: *iter
                            .next()
                            .ok_or(ApplicationLayerError::InsufficientData)?,
                        status: {
                            StatusField::from_bits_truncate(
                                *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            )
                        },
                        configuration_field: {
                            ConfigurationField::from_bytes(
                                *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                                *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                            )
                        },
                    },
                    variable_data_block: data
                        .get(data_block_offset..data.len())
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                    extended_link_layer: None,
                })
            }
            ControlInformation::ApplicationLayerCompactFrameShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerCompactFrameShortTransport control information",
                })
            }
            ControlInformation::CosemApplicationLayerLongTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "CosemApplicationLayerLongTransport control information",
                })
            }
            ControlInformation::CosemApplicationLayerShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "CosemApplicationLayerShortTransport control information",
                })
            }
            ControlInformation::ObisApplicationLayerReservedLongTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ObisApplicationLayerReservedLongTransport control information",
                })
            }
            ControlInformation::ObisApplicationLayerReservedShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ObisApplicationLayerReservedShortTransport control information",
                })
            }
            ControlInformation::TransportLayerLongReadoutToMeter => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "TransportLayerLongReadoutToMeter control information",
                })
            }
            ControlInformation::NetworkLayerData => Err(ApplicationLayerError::Unimplemented {
                feature: "NetworkLayerData control information",
            }),
            ControlInformation::FutureUse => Err(ApplicationLayerError::Unimplemented {
                feature: "FutureUse control information",
            }),
            ControlInformation::NetworkManagementApplication => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "NetworkManagementApplication control information",
                })
            }
            ControlInformation::TransportLayerCompactFrame => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "TransportLayerCompactFrame control information",
                })
            }
            ControlInformation::TransportLayerFormatFrame => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "TransportLayerFormatFrame control information",
                })
            }
            ControlInformation::NetworkManagementDataReserved => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "NetworkManagementDataReserved control information",
                })
            }
            ControlInformation::TransportLayerShortMeterToReadout => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "TransportLayerShortMeterToReadout control information",
                })
            }
            ControlInformation::TransportLayerLongMeterToReadout => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "TransportLayerLongMeterToReadout control information",
                })
            }
            ControlInformation::ExtendedLinkLayerI => {
                let mut iter = data.iter();
                iter.next();
                let extended_link_layer = Some(ExtendedLinkLayer {
                    communication_control: *iter
                        .next()
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                    access_number: *iter.next().ok_or(ApplicationLayerError::InsufficientData)?,
                    receiver_address: None,
                    encryption: None,
                });
                match UserDataBlock::try_from(iter.as_slice()) {
                    Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                        short_tpl_header,
                        variable_data_block,
                        ..
                    }) => Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                        extended_link_layer,
                        short_tpl_header,
                        variable_data_block,
                    }),
                    Ok(UserDataBlock::VariableDataStructureWithoutTplHeader {
                        variable_data_block,
                        ..
                    }) => Ok(UserDataBlock::VariableDataStructureWithoutTplHeader {
                        extended_link_layer,
                        variable_data_block,
                    }),
                    _ => Err(ApplicationLayerError::MissingControlInformation),
                }
            }
            ControlInformation::ExtendedLinkLayerII => {
                // CI byte + ELL II (8 bytes) = 9 bytes total before application data
                let (ell, ell_size) = ExtendedLinkLayer::parse(
                    data.get(1..)
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                    extended_link_layer::EllFormat::FormatII,
                )?;
                let app_data_offset = 1 + ell_size;

                // Create a ShortTplHeader from ELL fields
                // For encrypted ELL frames, the ELL fields become the "header"
                let short_tpl_header = ShortTplHeader {
                    access_number: ell.access_number,
                    status: StatusField::from_bits_truncate(ell.communication_control),
                    configuration_field: ConfigurationField::from_bytes(0x00, 0x00),
                };

                Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                    extended_link_layer: Some(ell),
                    short_tpl_header,
                    variable_data_block: data
                        .get(app_data_offset..)
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                })
            }
            ControlInformation::ExtendedLinkLayerIII => {
                // CI byte + ELL III (16 bytes) = 17 bytes total before application data
                let (ell, ell_size) = ExtendedLinkLayer::parse(
                    data.get(1..)
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                    extended_link_layer::EllFormat::FormatIII,
                )?;
                let app_data_offset = 1 + ell_size;

                // Create a ShortTplHeader from ELL fields
                // For encrypted ELL frames, the ELL fields become the "header"
                let short_tpl_header = ShortTplHeader {
                    access_number: ell.access_number,
                    status: StatusField::from_bits_truncate(ell.communication_control),
                    configuration_field: ConfigurationField::from_bytes(0x00, 0x00),
                };

                Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                    extended_link_layer: Some(ell),
                    short_tpl_header,
                    variable_data_block: data
                        .get(app_data_offset..)
                        .ok_or(ApplicationLayerError::InsufficientData)?,
                })
            }
        }
    }
}

#[allow(clippy::unwrap_used, clippy::panic)]
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
    fn test_application_layer_no_transport() {
        let data = [0x78, 0x0B, 0x13, 0x43, 0x65, 0x87];
        let result = UserDataBlock::try_from(data.as_slice());

        assert_eq!(
            result,
            Ok(UserDataBlock::VariableDataStructureWithoutTplHeader {
                extended_link_layer: None,
                variable_data_block: &data[1..],
            })
        );
    }

    #[test]
    fn test_ell_i_with_application_layer_no_transport() {
        let data = [0x8C, 0x20, 0x27, 0x78, 0x0B, 0x13, 0x43, 0x65, 0x87];
        let result = UserDataBlock::try_from(data.as_slice());

        match result {
            Ok(UserDataBlock::VariableDataStructureWithoutTplHeader {
                extended_link_layer: Some(ell),
                variable_data_block,
            }) => {
                assert_eq!(ell.communication_control, 0x20);
                assert_eq!(ell.access_number, 0x27);
                assert_eq!(variable_data_block, &data[4..]);
            }
            other => panic!("expected no-TPL user data after ELL I, got {other:?}"),
        }
    }

    #[test]
    fn test_device_type_roundtrip() {
        // Test that to_byte is the inverse of from_byte for specific values
        let test_cases = [
            (0x00, DeviceType::Other),
            (0x01, DeviceType::OilMeter),
            (0x02, DeviceType::ElectricityMeter),
            (0x03, DeviceType::GasMeter),
            (0x04, DeviceType::HeatMeterReturn),
            (0x05, DeviceType::SteamMeter),
            (0x06, DeviceType::WarmWaterMeter),
            (0x07, DeviceType::WaterMeter),
            (0x08, DeviceType::HeatCostAllocator),
            (0x09, DeviceType::CompressedAir),
            (0x0A, DeviceType::CoolingMeterReturn),
            (0x0B, DeviceType::CoolingMeterFlow),
            (0x0C, DeviceType::HeatMeterFlow),
            (0x0D, DeviceType::CombinedHeatCoolingMeter),
            (0x0E, DeviceType::BusSystemComponent),
            (0x0F, DeviceType::UnknownDevice),
            (0x10, DeviceType::IrrigationWaterMeter),
            (0x11, DeviceType::WaterDataLogger),
            (0x12, DeviceType::GasDataLogger),
            (0x13, DeviceType::GasConverter),
            (0x14, DeviceType::CalorificValue),
            (0x15, DeviceType::HotWaterMeter),
            (0x16, DeviceType::ColdWaterMeter),
            (0x17, DeviceType::DualRegisterWaterMeter),
            (0x18, DeviceType::PressureMeter),
            (0x19, DeviceType::AdConverter),
            (0x1A, DeviceType::SmokeDetector),
            (0x1B, DeviceType::RoomSensor),
            (0x1C, DeviceType::GasDetector),
            (0x20, DeviceType::ElectricityBreaker),
            (0x21, DeviceType::Valve),
            (0x25, DeviceType::CustomerUnit),
            (0x28, DeviceType::WasteWaterMeter),
            (0x29, DeviceType::Garbage),
            (0x30, DeviceType::ServiceTool),
            (0x31, DeviceType::CommunicationController),
            (0x32, DeviceType::UnidirectionalRepeater),
            (0x33, DeviceType::BidirectionalRepeater),
            (0x36, DeviceType::RadioConverterSystemSide),
            (0x37, DeviceType::RadioConverterMeterSide),
            (0x38, DeviceType::BusConverterMeterSide),
            (0xFF, DeviceType::Wildcard),
            // Reserved ranges with specific variants
            (0x1D, DeviceType::ReservedSensor(0x1D)), // Reserved for sensors
            (0x22, DeviceType::ReservedSwitch(0x22)), // Reserved for switching devices
            (0x40, DeviceType::Reserved(0x40)),       // Reserved
        ];

        for (byte, expected_device_type) in test_cases {
            let device_type = DeviceType::from(byte);
            assert_eq!(device_type, expected_device_type);
            assert_eq!(u8::from(device_type), byte);
        }

        // Test that Reserved variants map back to their byte values
        assert_eq!(u8::from(DeviceType::Reserved(0x40)), 0x40);
        assert_eq!(u8::from(DeviceType::ReservedSensor(0x1D)), 0x1D);
        assert_eq!(u8::from(DeviceType::ReservedSwitch(0x22)), 0x22);

        // Test that Unknown maps to canonical value 0x0F
        assert_eq!(u8::from(DeviceType::UnknownDevice), 0x0F);
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
                device_type_and_unit: 0xE97E,
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
    fn global_readout_request_does_not_consume_following_records() {
        // 0x7F followed by a valid 8-bit integer record: DIF=0x01, VIF=0x13, data=0x05
        let data: &[u8] = &[0x7F, 0x01, 0x13, 0x05];
        let records: Vec<_> = DataRecords::new(data, None).flatten().collect();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn parse_data_records_api_returns_record_values() {
        use crate::data_information::DataType;

        let data = [0x03, 0x13, 0x15, 0x31, 0x00];
        let mut records = parse_data_records(&data);
        let record = records.next().unwrap().unwrap();

        assert_eq!(record.value(), Some(&DataType::Number(12_565.0)));
        assert_eq!(record.raw_bytes(), &data);
        assert!(record.data_information().is_some());
        assert!(record.value_information().is_some());
        assert!(records.next().is_none());
    }

    #[test]
    fn parse_application_layer_api_exposes_records() {
        let data = [0x78, 0x03, 0x13, 0x15, 0x31, 0x00];
        let application_layer = parse_application_layer(&data).unwrap();
        let records: Result<Vec<_>, _> = application_layer.data_records().unwrap().collect();

        assert_eq!(records.unwrap().len(), 1);
    }

    #[test]
    fn data_record_iterator_reports_parse_errors() {
        let mut records = parse_data_records(&[0x04]);

        assert!(records.next().unwrap().is_err());
        assert!(records.next().is_none());
    }
}
