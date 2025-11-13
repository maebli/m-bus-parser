#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
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
pub struct IdentificationNumber {
    pub number: u32,
}

#[cfg(feature = "std")]
impl fmt::Display for IdentificationNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08}", self.number)
    }
}

/// This used to be called "Medium"
/// Defined in EN 13757-7
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Other,
    OilMeter,
    ElectricityMeter,
    GasMeter,
    HeatMeterReturn,
    SteamMeter,
    WarmWaterMeter,
    WaterMeter,
    HeatCostAllocator,
    CompressedAir,
    CoolingMeterReturn,
    CoolingMeterFlow,
    HeatMeterFlow,
    CombinedHeatCoolingMeter,
    BusSystemComponent,
    UnknownDevice,
    IrrigationWaterMeter,
    WaterDataLogger,
    GasDataLogger,
    GasConverter,
    CalorificValue,
    HotWaterMeter,
    ColdWaterMeter,
    DualRegisterWaterMeter,
    PressureMeter,
    AdConverter,
    SmokeDetector,
    RoomSensor,
    GasDetector,
    ReservedSensor(u8),
    ElectricityBreaker,
    Valve,
    ReservedSwitch(u8),
    CustomerUnit,
    ReservedCustomer(u8),
    WasteWaterMeter,
    Garbage,
    ReservedCO2,
    ReservedEnvironmental(u8),
    ServiceTool,
    CommunicationController,
    UnidirectionalRepeater,
    BidirectionalRepeater,
    ReservedSystem(u8),
    RadioConverterSystemSide,
    RadioConverterMeterSide,
    BusConverterMeterSide,
    Reserved(u8),
    Wildcard,
}

impl From<u8> for DeviceType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DeviceType::Other,
            0x01 => DeviceType::OilMeter,
            0x02 => DeviceType::ElectricityMeter,
            0x03 => DeviceType::GasMeter,
            0x04 => DeviceType::HeatMeterReturn,
            0x05 => DeviceType::SteamMeter,
            0x06 => DeviceType::WarmWaterMeter,
            0x07 => DeviceType::WaterMeter,
            0x08 => DeviceType::HeatCostAllocator,
            0x09 => DeviceType::CompressedAir,
            0x0A => DeviceType::CoolingMeterReturn,
            0x0B => DeviceType::CoolingMeterFlow,
            0x0C => DeviceType::HeatMeterFlow,
            0x0D => DeviceType::CombinedHeatCoolingMeter,
            0x0E => DeviceType::BusSystemComponent,
            0x0F => DeviceType::UnknownDevice,
            0x10 => DeviceType::IrrigationWaterMeter,
            0x11 => DeviceType::WaterDataLogger,
            0x12 => DeviceType::GasDataLogger,
            0x13 => DeviceType::GasConverter,
            0x14 => DeviceType::CalorificValue,
            0x15 => DeviceType::HotWaterMeter,
            0x16 => DeviceType::ColdWaterMeter,
            0x17 => DeviceType::DualRegisterWaterMeter,
            0x18 => DeviceType::PressureMeter,
            0x19 => DeviceType::AdConverter,
            0x1A => DeviceType::SmokeDetector,
            0x1B => DeviceType::RoomSensor,
            0x1C => DeviceType::GasDetector,
            0x1D..=0x1F => DeviceType::ReservedSensor(value),
            0x20 => DeviceType::ElectricityBreaker,
            0x21 => DeviceType::Valve,
            0x22..=0x24 => DeviceType::ReservedSwitch(value),
            0x25 => DeviceType::CustomerUnit,
            0x26..=0x27 => DeviceType::ReservedCustomer(value),
            0x28 => DeviceType::WasteWaterMeter,
            0x29 => DeviceType::Garbage,
            0x2A => DeviceType::ReservedCO2,
            0x2B..=0x2F => DeviceType::ReservedEnvironmental(value),
            0x30 => DeviceType::ServiceTool,
            0x31 => DeviceType::CommunicationController,
            0x32 => DeviceType::UnidirectionalRepeater,
            0x33 => DeviceType::BidirectionalRepeater,
            0x34..=0x35 => DeviceType::ReservedSystem(value),
            0x36 => DeviceType::RadioConverterSystemSide,
            0x37 => DeviceType::RadioConverterMeterSide,
            0x38 => DeviceType::BusConverterMeterSide,
            0x39..=0x3F => DeviceType::ReservedSystem(value),
            0x40..=0xFE => DeviceType::Reserved(value),
            0xFF => DeviceType::Wildcard,
        }
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

#[cfg(test)]
mod test {
    use super::*;

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
    fn test_identification_number() -> Result<(), ApplicationLayerError> {
        let data = [0x78, 0x56, 0x34, 0x12];
        let result = IdentificationNumber::from_bcd_hex_digits(data)?;
        assert_eq!(result, IdentificationNumber { number: 12345678 });
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Function {
    SndNk,
    SndUd { fcb: bool },
    SndUd2,
    SndUd3,
    SndNr,
    SendIr,
    AccNr,
    AccDmd,
    ReqUd1 { fcb: bool },
    ReqUd2 { fcb: bool },
    RspUd { acd: bool, dfc: bool },
    Ack,
    Nack,
    CnfIr,
}

#[cfg(feature = "std")]
impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::SndNk => write!(f, "SndNk"),
            Function::SndUd { fcb } => write!(f, "SndUd (FCB: {fcb})"),
            Function::ReqUd2 { fcb } => write!(f, "ReqUd2 (FCB: {fcb})"),
            Function::ReqUd1 { fcb } => write!(f, "ReqUd1 (FCB: {fcb})"),
            Function::RspUd { acd, dfc } => write!(f, "RspUd (ACD: {acd}, DFC: {dfc})"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FrameError {
    EmptyData,
    TooShort,
    WrongLength { expected: usize, actual: usize },
    InvalidFunction { byte: u8 },
}

impl TryFrom<u8> for Function {
    type Error = FrameError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x40 => Ok(Self::SndNk),
            0x44 => Ok(Self::SndNr),
            0x53 => Ok(Self::SndUd { fcb: false }),
            0x73 => Ok(Self::SndUd { fcb: true }),
            0x5B => Ok(Self::ReqUd2 { fcb: false }),
            0x7B => Ok(Self::ReqUd2 { fcb: true }),
            0x5A => Ok(Self::ReqUd1 { fcb: false }),
            0x7A => Ok(Self::ReqUd1 { fcb: true }),
            0x08 => Ok(Self::RspUd {
                acd: false,
                dfc: false,
            }),
            0x18 => Ok(Self::RspUd {
                acd: false,
                dfc: true,
            }),
            0x28 => Ok(Self::RspUd {
                acd: true,
                dfc: false,
            }),
            0x38 => Ok(Self::RspUd {
                acd: true,
                dfc: true,
            }),
            _ => Err(FrameError::InvalidFunction { byte }),
        }
    }
}
