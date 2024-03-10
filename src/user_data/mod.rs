//! is a part of the application layer

use arrayvec::ArrayVec;
pub mod data_information;
pub mod value_information;
pub mod variable_user_data;

// Maximum 234 bytes for variable data blocks, each block consists of a minimum of 2 bytes
// therefore the maximum number of blocks is 117, see https://m-bus.com/documentation-wired/06-application-layer
const MAXIMUM_VARIABLE_DATA_BLOCKS: usize = 117;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StatusField: u8 {
        const COUNTER_BINARY_SIGNED     = 0b00000001;
        const COUNTER_FIXED_DATE        = 0b00000010;
        const POWER_LOW                 = 0b00000100;
        const PERMANENT_ERROR           = 0b00001000;
        const TEMPORARY_ERROR           = 0b00010000;
        const MANUFACTURER_SPECIFIC_1   = 0b00100000;
        const MANUFACTURER_SPECIFIC_2   = 0b01000000;
        const MANUFACTURER_SPECIFIC_3   = 0b10000000;
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
            ControlInformation::ResetAtApplicationLevel => Direction::MasterToSlave,
            ControlInformation::SendData => Direction::MasterToSlave,
            ControlInformation::SelectSlave => Direction::MasterToSlave,
            ControlInformation::SynchronizeSlave => Direction::MasterToSlave,
            ControlInformation::SetBaudRate300 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate600 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate1200 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate2400 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate4800 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate9600 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate19200 => Direction::MasterToSlave,
            ControlInformation::SetBaudRate38400 => Direction::MasterToSlave,
            ControlInformation::OutputRAMContent => Direction::MasterToSlave,
            ControlInformation::WriteRAMContent => Direction::MasterToSlave,
            ControlInformation::StartCalibrationTestMode => Direction::MasterToSlave,
            ControlInformation::ReadEEPROM => Direction::MasterToSlave,
            ControlInformation::StartSoftwareTest => Direction::MasterToSlave,
            ControlInformation::HashProcedure(_) => Direction::MasterToSlave,
            ControlInformation::SendErrorStatus => Direction::SlaveToMaster,
            ControlInformation::SendAlarmStatus => Direction::SlaveToMaster,
            ControlInformation::ResponseWithVariableDataStructure => Direction::SlaveToMaster,
            ControlInformation::ResponseWithFixedDataStructure => Direction::SlaveToMaster,
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
    fn from(byte: u8) -> Result<ControlInformation, ApplicationLayerError> {
        match byte {
            0x50 => Ok(ControlInformation::ResetAtApplicationLevel),
            0x51 => Ok(ControlInformation::SendData),
            0x52 => Ok(ControlInformation::SelectSlave),
            0x54 => Ok(ControlInformation::SynchronizeSlave),
            0xB8 => Ok(ControlInformation::SetBaudRate300),
            0xB9 => Ok(ControlInformation::SetBaudRate600),
            0xBA => Ok(ControlInformation::SetBaudRate1200),
            0xBB => Ok(ControlInformation::SetBaudRate2400),
            0xBC => Ok(ControlInformation::SetBaudRate4800),
            0xBD => Ok(ControlInformation::SetBaudRate9600),
            0xBE => Ok(ControlInformation::SetBaudRate19200),
            0xBF => Ok(ControlInformation::SetBaudRate38400),
            0xB1 => Ok(ControlInformation::OutputRAMContent),
            0xB2 => Ok(ControlInformation::WriteRAMContent),
            0xB3 => Ok(ControlInformation::StartCalibrationTestMode),
            0xB4 => Ok(ControlInformation::ReadEEPROM),
            0xB6 => Ok(ControlInformation::StartSoftwareTest),
            0x90..=0x97 => Ok(ControlInformation::HashProcedure(byte - 0x90)),
            0x70 => Ok(ControlInformation::SendErrorStatus),
            0x71 => Ok(ControlInformation::SendAlarmStatus),
            0x72 | 0x76 => Ok(ControlInformation::ResponseWithVariableDataStructure),
            0x73 | 0x77 => Ok(ControlInformation::ResponseWithFixedDataStructure),
            _ => Err(ApplicationLayerError::InvalidControlInformation { byte }),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ApplicationLayerError {
    MissingControlInformation,
    InvalidControlInformation { byte: u8 },
    IdentificationNumberError { digits: [u8; 4], number: u32 },
    InvalidManufacturerCode { code: u16 },
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
            ApplicationLayerError::IdentificationNumberError => {
                write!(f, "Invalid identification number")
            }
            ApplicationLayerError::InvalidManufacturerCode { code } => {
                write!(f, "Invalid manufacturer code: {}", code)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ApplicationLayerError {}

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

impl ApplicationResetSubcode {
    pub fn from(value: u8) -> Self {
        match value & 0b1111 {
            // Extracting the lower 4 bits
            0b0000 => ApplicationResetSubcode::All(value),
            0b0001 => ApplicationResetSubcode::UserData(value),
            0b0010 => ApplicationResetSubcode::SimpleBilling(value),
            0b0011 => ApplicationResetSubcode::EnhancedBilling(value),
            0b0100 => ApplicationResetSubcode::MultiTariffBilling(value),
            0b0101 => ApplicationResetSubcode::InstantaneousValues(value),
            0b0110 => ApplicationResetSubcode::LoadManagementValues(value),
            0b0111 => ApplicationResetSubcode::Reserved1(value),
            0b1000 => ApplicationResetSubcode::InstallationStartup(value),
            0b1001 => ApplicationResetSubcode::Testing(value),
            0b1010 => ApplicationResetSubcode::Calibration(value),
            0b1011 => ApplicationResetSubcode::ConfigurationUpdates(value),
            0b1100 => ApplicationResetSubcode::Manufacturing(value),
            0b1101 => ApplicationResetSubcode::Development(value),
            0b1110 => ApplicationResetSubcode::Selftest(value),
            _ => ApplicationResetSubcode::Reserved2(value),
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
        number = number * 100 + (upper as u32 * 10) + lower as u32;
    }

    Ok(number)
}

#[derive(Debug, PartialEq)]
pub struct Counter {
    count: u32,
}

#[derive(Debug, PartialEq)]
pub struct IdentificationNumber {
    number: u32,
}

impl From<IdentificationNumber> for u32 {
    fn from(id: IdentificationNumber) -> Self {
        id.number
    }
}

impl IdentificationNumber {
    pub fn from_bcd_hex_digits(digits: [u8; 4]) -> Result<Self, ApplicationLayerError> {
        let number = bcd_hex_digits_to_u32(digits)?;
        Ok(IdentificationNumber { number })
    }
}

impl Counter {
    pub fn from_bcd_hex_digits(digits: [u8; 4]) -> Result<Self, ApplicationLayerError> {
        let count = bcd_hex_digits_to_u32(digits)?;
        Ok(Counter { count })
    }
}

#[derive(Debug, PartialEq)]
pub struct FixedDataHeder {
    identification_number: IdentificationNumber,
    manufacturer_code: ManufacturerCode,
    version: u8,
    medium: Medium,
    access_number: u8,
    status: StatusField,
    signature: u16,
}

#[derive(Debug, PartialEq)]
pub enum UserDataBlock {
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
        variable_data_block: ArrayVec<u8, MAXIMUM_VARIABLE_DATA_BLOCKS>,
        mdh: u8,
        manufacturer_specific_data: ArrayVec<u8, MAXIMUM_VARIABLE_DATA_BLOCKS>,
    },
}

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
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => Medium::Other,
            0x01 => Medium::Oil,
            0x02 => Medium::Electricity,
            0x03 => Medium::Gas,
            0x04 => Medium::Heat,
            0x05 => Medium::Steam,
            0x06 => Medium::HotWater,
            0x07 => Medium::Water,
            0x08 => Medium::HeatCostAllocator,
            0x09 => Medium::Reserved, // Note: Reserved for 0x09 from the first set
            0x0A => Medium::GasMode2,
            0x0B => Medium::HeatMode2,
            0x0C => Medium::HotWaterMode2,
            0x0D => Medium::WaterMode2,
            0x0E => Medium::HeatCostAllocator2,
            0x0F => Medium::ReservedMode2,
            // Unique mediums from the second set
            0x10 => Medium::Reserved, // Reserved range
            0x11 => Medium::Reserved, // Reserved range
            0x12 => Medium::Reserved, // Reserved range
            0x13 => Medium::Reserved, // Reserved range
            0x14 => Medium::Reserved, // Reserved range
            0x15 => Medium::Reserved, // Reserved range
            0x16 => Medium::ColdWater,
            0x17 => Medium::DualWater,
            0x18 => Medium::Pressure,
            0x19 => Medium::ADConverter,
            // Extended reserved range from the second set
            0x20..=0xFF => Medium::Reserved,
            _ => Medium::Unknown,
        }
    }
}

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

#[derive(Debug, PartialEq)]
pub struct ManufacturerCode {
    pub code: [char; 3],
}

impl ManufacturerCode {
    pub fn from_id(id: u16) -> Result<Self, ApplicationLayerError> {
        let first_letter = ((id / (32 * 32)) + 64) as u8 as char;
        let second_letter = (((id % (32 * 32)) / 32) + 64) as u8 as char;
        let third_letter = ((id % 32) + 64) as u8 as char;

        if first_letter.is_ascii_uppercase()
            && second_letter.is_ascii_uppercase()
            && third_letter.is_ascii_uppercase()
        {
            Ok(ManufacturerCode {
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
    pub fn new(byte: u8) -> Self {
        MeasuredMedium {
            medium: Medium::from_byte(byte),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for UserDataBlock {
    type Error = ApplicationLayerError;

    fn try_from(data: &'a [u8]) -> Result<Self, ApplicationLayerError> {
        if data.is_empty() {
            return Err(ApplicationLayerError::MissingControlInformation);
        }

        let control_information = ControlInformation::from(data[0])?;

        match control_information {
            ControlInformation::ResetAtApplicationLevel => {
                let subcode = ApplicationResetSubcode::from(data[1]);
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
                let variable_data_block = ArrayVec::<u8, MAXIMUM_VARIABLE_DATA_BLOCKS>::new();
                let manufacturer_specific_data: ArrayVec<u8, MAXIMUM_VARIABLE_DATA_BLOCKS> =
                    ArrayVec::new();
                Ok(UserDataBlock::VariableDataStructure {
                    fixed_data_header: FixedDataHeader {
                        identification_number: IdentificationNumber::from_bcd_hex_digits([
                            data[1], data[2], data[3], data[4],
                        ])?,
                        manufacturer: ManufacturerCode::from_id(u16::from_be_bytes([
                            data[6], data[5],
                        ]))?,
                        version: data[7],
                        medium: MeasuredMedium::new(data[8]).medium,
                        access_number: data[9],
                        status: StatusField::from_bits_truncate(data[10]),
                        signature: u16::from_be_bytes([data[12], data[11]]),
                    },
                    //variable_data_block: data[13..data.len() - 3],
                    variable_data_block,
                    mdh: data[data.len() - 3],
                    manufacturer_specific_data,
                })
            }
            ControlInformation::ResponseWithFixedDataStructure => {
                let identification_number = IdentificationNumber::from_bcd_hex_digits([
                    data[1], data[2], data[3], data[4],
                ])?;
                let access_number = data[5];
                let status = StatusField::from_bits_truncate(data[6]);
                let medium_and_unit = u16::from_be_bytes([data[7], data[8]]);
                let counter1 =
                    Counter::from_bcd_hex_digits([data[9], data[10], data[11], data[12]])?;
                let counter2 =
                    Counter::from_bcd_hex_digits([data[13], data[14], data[15], data[16]])?;
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
