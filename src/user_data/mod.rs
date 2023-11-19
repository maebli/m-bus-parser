//! # User Data
//! User data is part of the application layer

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


#[derive(Debug, Clone, Copy)]
pub struct IdentificationNumber {
    number: u32,
}

impl IdentificationNumber {
    pub fn new(number: u32) -> Self {
        IdentificationNumber { number }
    }

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


pub enum UserDataBlock {
    ResetAtApplicationLevel{subcode: ApplicationResetSubcode},
    FixedDataStructure{
        IdentificationNumber: IdentificationNumber,
        AccessNumber: u8,
        Status: u8,
        Medium: u8,
        Counter1: u32,
        Counter2: u32,
    }
}

pub enum UserDataError {
    InvalidControlInformation,
    InvalidUserData,
}

fn parse_user_data(data: &[u8]) -> Result<UserDataBlock, ApplicationLayerError> {
    
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
}