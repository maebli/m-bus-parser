//! # User Data
//! User data is part of the application layer

use std::fmt;

#[derive(Debug, PartialEq)]
pub struct StatusField {
    pub counter_binary_signed: bool,
    pub counter_fixed_date: bool,     
    pub power_low: bool,             
    pub permanent_error: bool,       
    pub temporary_error: bool,       
    pub manufacturer_specific_1: bool, 
    pub manufacturer_specific_2: bool,
    pub manufacturer_specific_3: bool,
}

impl StatusField {
    pub fn from(byte: u8) -> Self {
        StatusField {
            counter_binary_signed: byte & 0b00000001 != 0,
            counter_fixed_date: byte & 0b00000010 != 0,
            power_low: byte & 0b00000100 != 0,
            permanent_error: byte & 0b00001000 != 0,
            temporary_error: byte & 0b00010000 != 0,
            manufacturer_specific_1: byte & 0b00100000 != 0,
            manufacturer_specific_2: byte & 0b01000000 != 0,
            manufacturer_specific_3: byte & 0b10000000 != 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    SlaveToMaster,
    MasterToSlave,
}

#[derive(Debug, PartialEq)]
pub enum ControlInformation {
    SendData(Direction),
    SelectSlave(Direction),
    ResetAtApplicationLevel(Direction),
    SynchronizeSlave(Direction),
    SetBaudRate300(Direction),
    SetBaudRate600(Direction),
    SetBaudRate1200(Direction),
    SetBaudRate2400(Direction),
    SetBaudRate4800(Direction),
    SetBaudRate9600(Direction),
    SetBaudRate19200(Direction),
    SetBaudRate38400(Direction),
    OutputRAMContent(Direction),
    WriteRAMContent(Direction),
    StartCalibrationTestMode(Direction),
    ReadEEPROM(Direction),
    StartSoftwareTest(Direction),
    HashProcedure(u8, Direction),
    SendErrorStatus(Direction),
    SendAlarmStatus(Direction),
    ResponseWithVariableDataStructure(Direction),
    ResponseWithFixedDataStructure(Direction),
}

impl ControlInformation {
    fn from(byte: u8) -> Result<ControlInformation,ApplicationLayerError> {
        match byte {
            0x50 => Ok(ControlInformation::ResetAtApplicationLevel(Direction::MasterToSlave)),
            0x51 => Ok(ControlInformation::SendData(Direction::MasterToSlave)),
            0x52 => Ok(ControlInformation::SelectSlave(Direction::MasterToSlave)),
            0x54 => Ok(ControlInformation::SynchronizeSlave(Direction::MasterToSlave)),
            0xB8 => Ok(ControlInformation::SetBaudRate300(Direction::MasterToSlave)),
            0xB9 => Ok(ControlInformation::SetBaudRate600(Direction::MasterToSlave)),
            0xBA => Ok(ControlInformation::SetBaudRate1200(Direction::MasterToSlave)),
            0xBB => Ok(ControlInformation::SetBaudRate2400(Direction::MasterToSlave)),
            0xBC => Ok(ControlInformation::SetBaudRate4800(Direction::MasterToSlave)),
            0xBD => Ok(ControlInformation::SetBaudRate9600(Direction::MasterToSlave)),
            0xBE => Ok(ControlInformation::SetBaudRate19200(Direction::MasterToSlave)),
            0xBF => Ok(ControlInformation::SetBaudRate38400(Direction::MasterToSlave)),
            0xB1 => Ok(ControlInformation::OutputRAMContent(Direction::MasterToSlave)),
            0xB2 => Ok(ControlInformation::WriteRAMContent(Direction::MasterToSlave)),
            0xB3 => Ok(ControlInformation::StartCalibrationTestMode(Direction::MasterToSlave)),
            0xB4 => Ok(ControlInformation::ReadEEPROM(Direction::MasterToSlave)),
            0xB6 => Ok(ControlInformation::StartSoftwareTest(Direction::MasterToSlave)),
            0x90..=0x97 => Ok(ControlInformation::HashProcedure(byte - 0x90, Direction::MasterToSlave)),
            0x70 => Ok(ControlInformation::SendErrorStatus(Direction::SlaveToMaster)),
            0x71 => Ok(ControlInformation::SendAlarmStatus(Direction::SlaveToMaster)),
            0x72 | 0x76 => Ok(ControlInformation::ResponseWithVariableDataStructure(Direction::SlaveToMaster)),
            0x73 | 0x77 => Ok(ControlInformation::ResponseWithFixedDataStructure(Direction::SlaveToMaster)),
            _ => Err(ApplicationLayerError::InvalidControlInformation{byte}),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ApplicationLayerError{
    MissingControlInformation,
    InvalidControlInformation{byte:u8},
}

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
        match value & 0b1111 { // Extracting the lower 4 bits
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


#[derive(Debug, Clone)]
pub enum IdentificationNumberError {
    InvalidDigit,
    // You can add more error types here if needed
}

impl std::fmt::Display for IdentificationNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentificationNumberError::InvalidDigit => write!(f, "Invalid digit for BCD"),
            // Handle other errors here
        }
    }
}

impl std::error::Error for IdentificationNumberError {}


#[derive(Debug, PartialEq)]
pub struct IdentificationNumber {
    number: u32,
}

impl IdentificationNumber {

    pub fn from_bcd_digits(digits: [u8; 8]) -> Result<Self, IdentificationNumberError> {
        let mut number = 0u32;
        for &digit in &digits {
            if digit > 9 {
                return Err(IdentificationNumberError::InvalidDigit);
            }
            number = number * 10 + digit as u32;
        }
        Ok(IdentificationNumber { number })
    }

}

#[derive(Debug, PartialEq)]
pub struct FixedDataHeder{
    IdentificationNumber: IdentificationNumber,
    ManufacturerCode: ManufacturerCode,
    Version: u8,
    Medium: Medium,
    AccessNumber: u8,
    Status: StatusField,
    Signature: u16,
}

#[derive(Debug, PartialEq)]
pub enum UserDataBlock {
    ResetAtApplicationLevel{subcode: ApplicationResetSubcode},
    FixedDataStructure{
        IdentificationNumber: IdentificationNumber,
        AccessNumber: u8,
        Status: StatusField,
        MediumAdUnit: u16,
        Counter1: u32,
        Counter2: u32,
    },
    VariableDataStructure{
        FixedDataHeder: u8,
        VariableDataBlock: Vec<u8>,
        MDH: u8,
        ManufacturerSpecificData: Vec<u8>,
    },
}

#[derive(Debug,PartialEq)]
pub enum Medium {
    Other,
    Oil,
    Electricity,
    Gas,
    Heat,
    Steam,
    HotWater,
    Water,
    HCA,
    Reserved,
    GasMode2,
    HeatMode2,
    HotWaterMode2,
    WaterMode2,
    HCAMode2,
    ReservedMode2,
    HeatVolumeReturn,
    HeatCostAllocator,
    CompressedAir,
    CoolingLoadMeterReturn,
    CoolingLoadMeterFlow,
    HeatVolumeFlow,
    HeatCoolingLoadMeter,
    BusSystem,
    UnknownMedium,
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
            0x08 => Medium::HCA,
            0x09 => Medium::Reserved, // Note: Reserved for 0x09 from the first set
            0x0A => Medium::GasMode2,
            0x0B => Medium::HeatMode2,
            0x0C => Medium::HotWaterMode2,
            0x0D => Medium::WaterMode2,
            0x0E => Medium::HCAMode2,
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
            _ => Medium::UnknownMedium,
        }
    }
}


#[derive(Debug, PartialEq)]
pub struct ManufacturerCode {
    code: [char; 3],
}

impl ManufacturerCode {

    pub fn from_id(id: u16) -> Result<Self, &'static str> {
        let first_letter = ((id / (32 * 32)) + 64) as u8 as char;
        let second_letter = (((id % (32 * 32)) / 32) + 64) as u8 as char;
        let third_letter = ((id % 32) + 64) as u8 as char;

        if first_letter.is_ascii_uppercase() && second_letter.is_ascii_uppercase() && third_letter.is_ascii_uppercase() {
            Ok(ManufacturerCode { code: [first_letter, second_letter, third_letter] })
        } else {
            Err("ID does not correspond to valid ASCII uppercase letters")
        }
    }

    pub fn calculate_id(&self) -> Result<u16, &'static str> {
        let id = self.code.iter().enumerate().fold(0, |acc, (index, &char)| {
            acc + ((char as u16 - 64) * 32u16.pow(2 - index as u32))
        });

        if id <= u16::MAX {
            Ok(id)
        } else {
            Err("Calculated ID exceeds the 2-byte limit")
        }
    }

}

impl fmt::Display for ManufacturerCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.code[0], self.code[1], self.code[2])
    }
}

#[derive(Debug,PartialEq)]
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

pub fn parse_user_data(data: &[u8]) -> Result<UserDataBlock, ApplicationLayerError> {
    
    if data.is_empty() {
        return Err(ApplicationLayerError::MissingControlInformation);
    }

    let control_information = ControlInformation::from(data[0])?;

    match control_information {
        ControlInformation::ResetAtApplicationLevel(_) => {
            let subcode = ApplicationResetSubcode::from(data[1]);
            Ok(UserDataBlock::ResetAtApplicationLevel{subcode})
        },
        ControlInformation::SendData(_) => todo!(),
        ControlInformation::SelectSlave(_) => todo!(),
        ControlInformation::SynchronizeSlave(_) => todo!(),
        ControlInformation::SetBaudRate300(_) => todo!(),
        ControlInformation::SetBaudRate600(_) => todo!(),
        ControlInformation::SetBaudRate1200(_) => todo!(),
        ControlInformation::SetBaudRate2400(_) => todo!(),
        ControlInformation::SetBaudRate4800(_) => todo!(),
        ControlInformation::SetBaudRate9600(_) => todo!(),
        ControlInformation::SetBaudRate19200(_) => todo!(),
        ControlInformation::SetBaudRate38400(_) => todo!(),
        ControlInformation::OutputRAMContent(_) => todo!(),
        ControlInformation::WriteRAMContent(_) => todo!(),
        ControlInformation::StartCalibrationTestMode(_) => todo!(),
        ControlInformation::ReadEEPROM(_) => todo!(),
        ControlInformation::StartSoftwareTest(_) => todo!(),
        ControlInformation::HashProcedure(_, _) => todo!(),
        ControlInformation::SendErrorStatus(_) => todo!(),
        ControlInformation::SendAlarmStatus(_) => todo!(),
        ControlInformation::ResponseWithVariableDataStructure(_) => todo!(),
        ControlInformation::ResponseWithFixedDataStructure(_) => todo!(),
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_control_information() {
        assert_eq!(ControlInformation::from(0x50), Ok(ControlInformation::ResetAtApplicationLevel(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0x51), Ok(ControlInformation::SendData(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0x52), Ok(ControlInformation::SelectSlave(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0x54), Ok(ControlInformation::SynchronizeSlave(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB8), Ok(ControlInformation::SetBaudRate300(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB9), Ok(ControlInformation::SetBaudRate600(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBA), Ok(ControlInformation::SetBaudRate1200(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBB), Ok(ControlInformation::SetBaudRate2400(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBC), Ok(ControlInformation::SetBaudRate4800(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBD), Ok(ControlInformation::SetBaudRate9600(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBE), Ok(ControlInformation::SetBaudRate19200(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xBF), Ok(ControlInformation::SetBaudRate38400(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB1), Ok(ControlInformation::OutputRAMContent(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB2), Ok(ControlInformation::WriteRAMContent(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB3), Ok(ControlInformation::StartCalibrationTestMode(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB4), Ok(ControlInformation::ReadEEPROM(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0xB6), Ok(ControlInformation::StartSoftwareTest(Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0x90), Ok(ControlInformation::HashProcedure(0, Direction::MasterToSlave)));
        assert_eq!(ControlInformation::from(0x91), Ok(ControlInformation::HashProcedure(1, Direction::MasterToSlave)));
    }

    #[test]
    fn test_reset_subcode(){
        // Application layer of frame | 68 04 04 68 | 53 FE 50 | 10 | B1 16
        let data = [ 0x50, 0x10];
        let result = parse_user_data(&data);
        assert_eq!(result, Ok(UserDataBlock::ResetAtApplicationLevel{subcode: ApplicationResetSubcode::All(0x10)}));
    }
}