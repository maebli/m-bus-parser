//! is a part of the application layer
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use std::fmt;

pub use m_bus_core::decryption;

use m_bus_core::{
    bcd_hex_digits_to_u32, ConfigurationField, DeviceType, IdentificationNumber, ManufacturerCode,
};
use variable_user_data::DataRecordError;

use self::data_record::DataRecord;
#[cfg(feature = "decryption")]
use m_bus_core::decryption::DecryptionError::{NotEncrypted, UnknownEncryptionState};
use m_bus_core::ApplicationLayerError;

pub mod data_information;
pub mod data_record;
pub mod extended_link_layer;
pub mod value_information;
pub mod variable_user_data;

use extended_link_layer::ExtendedLinkLayer;

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
                    let record = if let Some(long_tpl_header) = self.long_tpl_header {
                        DataRecord::try_from((self.data.get(self.offset..)?, long_tpl_header))
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
}

impl<'a> UserDataBlock<'a> {
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
    pub manufacturer: Result<ManufacturerCode, ApplicationLayerError>,
    pub version: u8,
    pub device_type: DeviceType,
    pub short_tpl_header: ShortTplHeader,
    pub lsb_order: bool,
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
            ControlInformation::SendData => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SendData control information",
                })
            }
            ControlInformation::SelectSlave => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SelectSlave control information",
                })
            }
            ControlInformation::SynchronizeSlave => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SynchronizeSlave control information",
                })
            }
            ControlInformation::SetBaudRate300 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate300 control information",
                })
            }
            ControlInformation::SetBaudRate600 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate600 control information",
                })
            }
            ControlInformation::SetBaudRate1200 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate1200 control information",
                })
            }
            ControlInformation::SetBaudRate2400 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate2400 control information",
                })
            }
            ControlInformation::SetBaudRate4800 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate4800 control information",
                })
            }
            ControlInformation::SetBaudRate9600 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate9600 control information",
                })
            }
            ControlInformation::SetBaudRate19200 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate19200 control information",
                })
            }
            ControlInformation::SetBaudRate38400 => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SetBaudRate38400 control information",
                })
            }
            ControlInformation::OutputRAMContent => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "OutputRAMContent control information",
                })
            }
            ControlInformation::WriteRAMContent => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "WriteRAMContent control information",
                })
            }
            ControlInformation::StartCalibrationTestMode => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "StartCalibrationTestMode control information",
                })
            }
            ControlInformation::ReadEEPROM => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ReadEEPROM control information",
                })
            }
            ControlInformation::StartSoftwareTest => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "StartSoftwareTest control information",
                })
            }
            ControlInformation::HashProcedure(_) => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "HashProcedure control information",
                })
            }
            ControlInformation::SendErrorStatus => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SendErrorStatus control information",
                })
            }
            ControlInformation::SendAlarmStatus => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "SendAlarmStatus control information",
                })
            }
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
            ControlInformation::ClockSyncAbsolute => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ClockSyncAbsolute control information",
                })
            }
            ControlInformation::ClockSyncRelative => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ClockSyncRelative control information",
                })
            }
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
            ControlInformation::AlarmShortTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "AlarmShortTransport control information",
                })
            }
            ControlInformation::AlarmLongTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "AlarmLongTransport control information",
                })
            }
            ControlInformation::ApplicationLayerNoTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerNoTransport control information",
                })
            }
            ControlInformation::ApplicationLayerCompactFrameNoTransport => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ApplicationLayerCompactFrameNoTransport control information",
                })
            }
            ControlInformation::ApplicationLayerShortTransport => {
                let mut iter = data.iter().skip(1);

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
                        .get(5..data.len())
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
            ControlInformation::NetworkLayerData => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "NetworkLayerData control information",
                })
            }
            ControlInformation::FutureUse => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "FutureUse control information",
                })
            }
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
                let user_block = UserDataBlock::try_from(iter.as_slice());
                if let Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                    extended_link_layer: _,
                    short_tpl_header,
                    variable_data_block: _,
                }) = user_block
                {
                    Ok(UserDataBlock::VariableDataStructureWithShortTplHeader {
                        extended_link_layer,
                        short_tpl_header,
                        variable_data_block: {
                            data.get(8..data.len())
                                .ok_or(ApplicationLayerError::InsufficientData)?
                        },
                    })
                } else {
                    Err(ApplicationLayerError::MissingControlInformation)
                }
            }
            ControlInformation::ExtendedLinkLayerII => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ExtendedLinkLayerII control information",
                })
            }
            ControlInformation::ExtendedLinkLayerIII => {
                Err(ApplicationLayerError::Unimplemented {
                    feature: "ExtendedLinkLayerIII control information",
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
    fn test_lsb_frame() {
        use crate::data_information::DataType;
        use wired_mbus_link_layer::WiredFrame;

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
            let frame = WiredFrame::try_from(frame).unwrap();

            if let WiredFrame::LongFrame {
                function: _,
                address: _,
                data,
            } = frame
            {
                let user_data_block = UserDataBlock::try_from(data).unwrap();
                if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                    long_tpl_header: fixed_data_header,
                    variable_data_block,
                    ..
                } = user_data_block
                {
                    assert_eq!(
                        fixed_data_header.identification_number.number,
                        expected_iden_nr
                    );

                    let mut data_records =
                        DataRecords::from((variable_data_block, &fixed_data_header)).flatten();
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
        use crate::data_information::DataType;
        use wired_mbus_link_layer::WiredFrame;

        let manufacturer_specific_data_frame: &[u8] = &[
            0x68, 0x55, 0x55, 0x68, 0x8, 0x1e, 0x72, 0x34, 0x35, 0x58, 0x12, 0x92, 0x26, 0x18, 0x4,
            0x14, 0x0, 0x0, 0x0, 0xc, 0x78, 0x34, 0x35, 0x58, 0x12, 0x4, 0xe, 0x57, 0x64, 0x3, 0x0,
            0xc, 0x14, 0x73, 0x58, 0x44, 0x0, 0xb, 0x2d, 0x6, 0x0, 0x0, 0xb, 0x3b, 0x55, 0x0, 0x0,
            0xa, 0x5a, 0x87, 0x6, 0xa, 0x5e, 0x77, 0x5, 0xb, 0x61, 0x1, 0x11, 0x0, 0x4, 0x6d, 0x10,
            0x2, 0x4, 0x3c, 0x2, 0x27, 0x79, 0x11, 0x9, 0xfd, 0xe, 0x6, 0x9, 0xfd, 0xf, 0x6, 0x8c,
            0xc0, 0x0, 0x15, 0x71, 0x25, 0x0, 0x0, 0xf, 0x0, 0x0, 0x86, 0x16,
        ];

        let frame = WiredFrame::try_from(manufacturer_specific_data_frame).unwrap();

        if let WiredFrame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            let user_data_block = UserDataBlock::try_from(data).unwrap();
            if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header: fixed_data_header,
                variable_data_block,
                ..
            } = user_data_block
            {
                let mut data_records: Vec<_> =
                    DataRecords::from((variable_data_block, &fixed_data_header))
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
        use crate::data_information::DataType;
        use crate::value_information::ValueLabel;
        use wired_mbus_link_layer::WiredFrame;

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

        let frame = WiredFrame::try_from(real32bit).unwrap();

        if let WiredFrame::LongFrame {
            function: _,
            address: _,
            data,
        } = frame
        {
            let user_data_block = UserDataBlock::try_from(data).unwrap();
            if let UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header: fixed_data_header,
                variable_data_block,
                ..
            } = user_data_block
            {
                let data_records: Vec<DataRecord> =
                    DataRecords::from((variable_data_block, &fixed_data_header))
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
