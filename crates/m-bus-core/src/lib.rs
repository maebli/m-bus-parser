pub mod decryption;

#[cfg(feature = "std")]
use std::fmt::{self, Display};

#[cfg(not(feature = "std"))]
use core::fmt;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
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

    #[must_use]
    pub const fn to_id(&self) -> u16 {
        (self.code[0] as u16 - 64) * 32 * 32
            + (self.code[1] as u16 - 64) * 32
            + (self.code[2] as u16 - 64)
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

#[cfg(feature = "std")]
impl Display for DeviceType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DeviceType::Other => write!(f, "Other"),
            DeviceType::OilMeter => write!(f, "Oil Meter"),
            DeviceType::ElectricityMeter => write!(f, "Electricity Meter"),
            DeviceType::GasMeter => write!(f, "Gas Meter"),
            DeviceType::HeatMeterReturn => write!(f, "Heat Meter (Return)"),
            DeviceType::SteamMeter => write!(f, "Steam Meter"),
            DeviceType::WarmWaterMeter => write!(f, "Warm Water Meter (30-90°C)"),
            DeviceType::WaterMeter => write!(f, "Water Meter"),
            DeviceType::HeatCostAllocator => write!(f, "Heat Cost Allocator"),
            DeviceType::CompressedAir => write!(f, "Compressed Air"),
            DeviceType::CoolingMeterReturn => write!(f, "Cooling Meter (Return)"),
            DeviceType::CoolingMeterFlow => write!(f, "Cooling Meter (Flow)"),
            DeviceType::HeatMeterFlow => write!(f, "Heat Meter (Flow)"),
            DeviceType::CombinedHeatCoolingMeter => write!(f, "Combined Heat/Cooling Meter"),
            DeviceType::BusSystemComponent => write!(f, "Bus/System Component"),
            DeviceType::UnknownDevice => write!(f, "Unknown Device"),
            DeviceType::IrrigationWaterMeter => write!(f, "Irrigation Water Meter"),
            DeviceType::WaterDataLogger => write!(f, "Water Data Logger"),
            DeviceType::GasDataLogger => write!(f, "Gas Data Logger"),
            DeviceType::GasConverter => write!(f, "Gas Converter"),
            DeviceType::CalorificValue => write!(f, "Calorific Value"),
            DeviceType::HotWaterMeter => write!(f, "Hot Water Meter (≥90°C)"),
            DeviceType::ColdWaterMeter => write!(f, "Cold Water Meter"),
            DeviceType::DualRegisterWaterMeter => write!(f, "Dual Register Water Meter"),
            DeviceType::PressureMeter => write!(f, "Pressure Meter"),
            DeviceType::AdConverter => write!(f, "A/D Converter"),
            DeviceType::SmokeDetector => write!(f, "Smoke Detector"),
            DeviceType::RoomSensor => write!(f, "Room Sensor"),
            DeviceType::GasDetector => write!(f, "Gas Detector"),
            DeviceType::ReservedSensor(code) => write!(f, "Reserved Sensor (0x{:02X})", code),
            DeviceType::ElectricityBreaker => write!(f, "Breaker (Electricity)"),
            DeviceType::Valve => write!(f, "Valve (Gas/Water)"),
            DeviceType::ReservedSwitch(code) => write!(f, "Reserved Switch (0x{:02X})", code),
            DeviceType::CustomerUnit => write!(f, "Customer Unit (Display)"),
            DeviceType::ReservedCustomer(code) => {
                write!(f, "Reserved Customer Unit (0x{:02X})", code)
            }
            DeviceType::WasteWaterMeter => write!(f, "Waste Water Meter"),
            DeviceType::Garbage => write!(f, "Garbage"),
            DeviceType::ReservedCO2 => write!(f, "Reserved (CO₂)"),
            DeviceType::ReservedEnvironmental(code) => {
                write!(f, "Reserved Environmental (0x{:02X})", code)
            }
            DeviceType::ServiceTool => write!(f, "Service Tool"),
            DeviceType::CommunicationController => write!(f, "Communication Controller (Gateway)"),
            DeviceType::UnidirectionalRepeater => write!(f, "Unidirectional Repeater"),
            DeviceType::BidirectionalRepeater => write!(f, "Bidirectional Repeater"),
            DeviceType::ReservedSystem(code) => write!(f, "Reserved System (0x{:02X})", code),
            DeviceType::RadioConverterSystemSide => write!(f, "Radio Converter (System Side)"),
            DeviceType::RadioConverterMeterSide => write!(f, "Radio Converter (Meter Side)"),
            DeviceType::BusConverterMeterSide => write!(f, "Bus Converter (Meter Side)"),
            DeviceType::Reserved(code) => write!(f, "Reserved (0x{:02X})", code),
            DeviceType::Wildcard => write!(f, "Wildcard"),
        }
    }
}

impl DeviceType {
    pub fn to_byte(&self) -> u8 {
        match self {
            DeviceType::Other => 0x00,
            DeviceType::OilMeter => 0x01,
            DeviceType::ElectricityMeter => 0x02,
            DeviceType::GasMeter => 0x03,
            DeviceType::HeatMeterReturn => 0x04,
            DeviceType::SteamMeter => 0x05,
            DeviceType::WarmWaterMeter => 0x06,
            DeviceType::WaterMeter => 0x07,
            DeviceType::HeatCostAllocator => 0x08,
            DeviceType::CompressedAir => 0x09,
            DeviceType::CoolingMeterReturn => 0x0A,
            DeviceType::CoolingMeterFlow => 0x0B,
            DeviceType::HeatMeterFlow => 0x0C,
            DeviceType::CombinedHeatCoolingMeter => 0x0D,
            DeviceType::BusSystemComponent => 0x0E,
            DeviceType::UnknownDevice => 0x0F,
            DeviceType::IrrigationWaterMeter => 0x10,
            DeviceType::WaterDataLogger => 0x11,
            DeviceType::GasDataLogger => 0x12,
            DeviceType::GasConverter => 0x13,
            DeviceType::CalorificValue => 0x14,
            DeviceType::HotWaterMeter => 0x15,
            DeviceType::ColdWaterMeter => 0x16,
            DeviceType::DualRegisterWaterMeter => 0x17,
            DeviceType::PressureMeter => 0x18,
            DeviceType::AdConverter => 0x19,
            DeviceType::SmokeDetector => 0x1A,
            DeviceType::RoomSensor => 0x1B,
            DeviceType::GasDetector => 0x1C,
            DeviceType::ReservedSensor(code) => *code,
            DeviceType::ElectricityBreaker => 0x20,
            DeviceType::Valve => 0x21,
            DeviceType::ReservedSwitch(code) => *code,
            DeviceType::CustomerUnit => 0x25,
            DeviceType::ReservedCustomer(code) => *code,
            DeviceType::WasteWaterMeter => 0x28,
            DeviceType::Garbage => 0x29,
            DeviceType::ReservedCO2 => 0x2A,
            DeviceType::ReservedEnvironmental(code) => *code,
            DeviceType::ServiceTool => 0x30,
            DeviceType::CommunicationController => 0x31,
            DeviceType::UnidirectionalRepeater => 0x32,
            DeviceType::BidirectionalRepeater => 0x33,
            DeviceType::ReservedSystem(code) => *code,
            DeviceType::RadioConverterSystemSide => 0x36,
            DeviceType::RadioConverterMeterSide => 0x37,
            DeviceType::BusConverterMeterSide => 0x38,
            DeviceType::Reserved(code) => *code,
            DeviceType::Wildcard => 0xFF,
        }
    }
}

impl From<DeviceType> for u8 {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Other => 0x00,
            DeviceType::OilMeter => 0x01,
            DeviceType::ElectricityMeter => 0x02,
            DeviceType::GasMeter => 0x03,
            DeviceType::HeatMeterReturn => 0x04,
            DeviceType::SteamMeter => 0x05,
            DeviceType::WarmWaterMeter => 0x06,
            DeviceType::WaterMeter => 0x07,
            DeviceType::HeatCostAllocator => 0x08,
            DeviceType::CompressedAir => 0x09,
            DeviceType::CoolingMeterReturn => 0x0A,
            DeviceType::CoolingMeterFlow => 0x0B,
            DeviceType::HeatMeterFlow => 0x0C,
            DeviceType::CombinedHeatCoolingMeter => 0x0D,
            DeviceType::BusSystemComponent => 0x0E,
            DeviceType::UnknownDevice => 0x0F,
            DeviceType::IrrigationWaterMeter => 0x10,
            DeviceType::WaterDataLogger => 0x11,
            DeviceType::GasDataLogger => 0x12,
            DeviceType::GasConverter => 0x13,
            DeviceType::CalorificValue => 0x14,
            DeviceType::HotWaterMeter => 0x15,
            DeviceType::ColdWaterMeter => 0x16,
            DeviceType::DualRegisterWaterMeter => 0x17,
            DeviceType::PressureMeter => 0x18,
            DeviceType::AdConverter => 0x19,
            DeviceType::SmokeDetector => 0x1A,
            DeviceType::RoomSensor => 0x1B,
            DeviceType::GasDetector => 0x1C,
            DeviceType::ReservedSensor(code) => code,
            DeviceType::ElectricityBreaker => 0x20,
            DeviceType::Valve => 0x21,
            DeviceType::ReservedSwitch(code) => code,
            DeviceType::CustomerUnit => 0x25,
            DeviceType::ReservedCustomer(code) => code,
            DeviceType::WasteWaterMeter => 0x28,
            DeviceType::Garbage => 0x29,
            DeviceType::ReservedCO2 => 0x2A,
            DeviceType::ReservedEnvironmental(code) => code,
            DeviceType::ServiceTool => 0x30,
            DeviceType::CommunicationController => 0x31,
            DeviceType::UnidirectionalRepeater => 0x32,
            DeviceType::BidirectionalRepeater => 0x33,
            DeviceType::ReservedSystem(code) => code,
            DeviceType::RadioConverterSystemSide => 0x36,
            DeviceType::RadioConverterMeterSide => 0x37,
            DeviceType::BusConverterMeterSide => 0x38,
            DeviceType::Reserved(code) => code,
            DeviceType::Wildcard => 0xFF,
        }
    }
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
    fn test_manufacturer_code_roundtrip() -> Result<(), ApplicationLayerError> {
        // Test that to_id is the inverse of from_id
        let original_id = 0x1ee6;
        let code = ManufacturerCode::from_id(original_id)?;
        let converted_id = code.to_id();
        assert_eq!(original_id, converted_id);

        // Test a few more cases
        for id in [0x0000, 0x0421, 0x1234, 0x7FFF] {
            if let Ok(code) = ManufacturerCode::from_id(id) {
                assert_eq!(id, code.to_id());
            }
        }
        Ok(())
    }

    #[test]
    fn test_identification_number() -> Result<(), ApplicationLayerError> {
        let data = [0x78, 0x56, 0x34, 0x12];
        let result = IdentificationNumber::from_bcd_hex_digits(data)?;
        assert_eq!(result, IdentificationNumber { number: 12345678 });
        Ok(())
    }

    #[test]
    fn test_configuration_field_debug() {
        // Test raw value 1360 (0x0550) - corresponds to mode 5 (AES-CBC-128; IV ≠ 0)
        let cf = ConfigurationField::from(1360);
        let debug_output = format!("{:?}", cf);
        assert_eq!(
            debug_output,
            "ConfigurationField { mode: AesCbc128IvNonZero }"
        );

        // Test mode 0 (No encryption)
        let cf_no_enc = ConfigurationField::from(0);
        let debug_output = format!("{:?}", cf_no_enc);
        assert_eq!(debug_output, "ConfigurationField { mode: NoEncryption }");
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Function {
    SndNk { prm: bool },
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
            Function::SndNk { prm: _prm } => write!(f, "SndNk"),
            Function::SndUd { fcb } => write!(f, "SndUd (FCB: {fcb})"),
            Function::ReqUd2 { fcb } => write!(f, "ReqUd2 (FCB: {fcb})"),
            Function::ReqUd1 { fcb } => write!(f, "ReqUd1 (FCB: {fcb})"),
            Function::RspUd { acd, dfc } => write!(f, "RspUd (ACD: {acd}, DFC: {dfc})"),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FrameError {
    EmptyData,
    InvalidStartByte,
    InvalidStopByte,
    WrongLengthIndication,
    LengthShort,
    LengthShorterThanSix { length: usize },
    WrongLength { expected: usize, actual: usize },
    WrongCrc { expected: u16, actual: u16 },
    WrongChecksum { expected: u8, actual: u8 },
    InvalidControlInformation { byte: u8 },
    InvalidFunction { byte: u8 },
}

#[cfg(feature = "std")]
impl std::error::Error for FrameError {}

#[cfg(feature = "std")]
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::EmptyData => write!(f, "Data is empty"),
            FrameError::InvalidStartByte => write!(f, "Invalid start byte"),
            FrameError::InvalidStopByte => write!(f, "Invalid stop byte"),
            FrameError::LengthShort => write!(f, "Length mismatch"),
            FrameError::LengthShorterThanSix { length } => {
                write!(f, "Length is shorter than six: {}", length)
            }
            FrameError::WrongChecksum { expected, actual } => write!(
                f,
                "Wrong checksum, expected: {}, actual: {}",
                expected, actual
            ),
            FrameError::InvalidControlInformation { byte } => {
                write!(f, "Invalid control information: {}", byte)
            }
            FrameError::InvalidFunction { byte } => write!(f, "Invalid function: {}", byte),
            FrameError::WrongLengthIndication => write!(f, "Wrong length indication"),
            FrameError::WrongLength { expected, actual } => write!(
                f,
                "Wrong length, expected: {}, actual: {}",
                expected, actual
            ),
            FrameError::WrongCrc { expected, actual } => {
                write!(f, "Wrong CRC, expected: {}, actual: {}", expected, actual)
            }
        }
    }
}
impl TryFrom<u8> for Function {
    type Error = FrameError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x40 => Ok(Self::SndNk { prm: false }),
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

/// Security Mode as defined in EN 13757-7:2018 Table 19
///
/// The Security mode defines the applied set of security mechanisms
/// and is encoded in bits 12-8 (5 bits) of the Configuration Field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum SecurityMode {
    NoEncryption,
    ManufacturerSpecific,
    DesIvZero,
    DesIvNonZero,
    SpecificUsage4,
    AesCbc128IvNonZero,
    Reserved6,
    AesCbc128IvZero,
    AesCtr128Cmac,
    AesGcm128,
    AesCcm128,
    Reserved11,
    Reserved12,
    SpecificUsage13,
    Reserved14,
    SpecificUsage15,
    ReservedHigher(u8),
}

impl SecurityMode {
    /// Create SecurityMode from 5-bit value (bits 12-8)
    pub const fn from_bits(mode: u8) -> Self {
        match mode & 0b0001_1111 {
            0 => Self::NoEncryption,
            1 => Self::ManufacturerSpecific,
            2 => Self::DesIvZero,
            3 => Self::DesIvNonZero,
            4 => Self::SpecificUsage4,
            5 => Self::AesCbc128IvNonZero,
            6 => Self::Reserved6,
            7 => Self::AesCbc128IvZero,
            8 => Self::AesCtr128Cmac,
            9 => Self::AesGcm128,
            10 => Self::AesCcm128,
            11 => Self::Reserved11,
            12 => Self::Reserved12,
            13 => Self::SpecificUsage13,
            14 => Self::Reserved14,
            15 => Self::SpecificUsage15,
            other => Self::ReservedHigher(other),
        }
    }

    /// Get the 5-bit mode value
    pub const fn to_bits(&self) -> u8 {
        match self {
            Self::NoEncryption => 0,
            Self::ManufacturerSpecific => 1,
            Self::DesIvZero => 2,
            Self::DesIvNonZero => 3,
            Self::SpecificUsage4 => 4,
            Self::AesCbc128IvNonZero => 5,
            Self::Reserved6 => 6,
            Self::AesCbc128IvZero => 7,
            Self::AesCtr128Cmac => 8,
            Self::AesGcm128 => 9,
            Self::AesCcm128 => 10,
            Self::Reserved11 => 11,
            Self::Reserved12 => 12,
            Self::SpecificUsage13 => 13,
            Self::Reserved14 => 14,
            Self::SpecificUsage15 => 15,
            Self::ReservedHigher(mode) => *mode & 0b0001_1111,
        }
    }
}

#[cfg(feature = "std")]
impl fmt::Display for SecurityMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEncryption => write!(f, "No encryption used"),
            Self::ManufacturerSpecific => write!(f, "Manufacturer specific usage"),
            Self::DesIvZero => write!(f, "DES; IV = 0 (deprecated)"),
            Self::DesIvNonZero => write!(f, "DES; IV ≠ 0 (deprecated)"),
            Self::SpecificUsage4 => write!(f, "Specific usage (Bibliographical Entry [6])"),
            Self::AesCbc128IvNonZero => write!(f, "AES-CBC-128; IV ≠ 0"),
            Self::Reserved6 => write!(f, "Reserved for future use"),
            Self::AesCbc128IvZero => write!(f, "AES-CBC-128; IV = 0"),
            Self::AesCtr128Cmac => write!(f, "AES-CTR-128; CMAC"),
            Self::AesGcm128 => write!(f, "AES-GCM-128"),
            Self::AesCcm128 => write!(f, "AES-CCM-128"),
            Self::Reserved11 => write!(f, "Reserved for future use"),
            Self::Reserved12 => write!(f, "Reserved for future use"),
            Self::SpecificUsage13 => write!(f, "Specific usage (Bibliographical Entry [8])"),
            Self::Reserved14 => write!(f, "Reserved for future use"),
            Self::SpecificUsage15 => write!(f, "Specific usage (Bibliographical Entry [7])"),
            Self::ReservedHigher(mode) => write!(f, "Reserved for future use (mode {})", mode),
        }
    }
}

/// Configuration Field (CF) - EN 13757-7:2018 Clause 7.5.8, Table 18
///
/// The configuration field consists of two bytes containing information about the applied
/// Security mode. The Security mode defines:
/// - applied set of security mechanisms
/// - content of other bits in the configuration field
/// - presence, length and content of configuration field extension (CFE)
/// - number, length and content of optional TPL-header/trailer fields
///
/// # Bit Layout (Table 18)
/// - Bits 15-13: Security mode specific (X)
/// - Bits 12-8: Security mode bits (M) - 5 bits defining the security mode
/// - Bits 7-0: Security mode specific (X)
///
/// The decoding of bits marked "X" depends on the selected Security mode.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConfigurationField {
    raw: u16,
}

impl ConfigurationField {
    /// Create a Configuration Field from two bytes
    ///
    /// # Arguments
    /// * `lsb` - Lower byte (bits 7-0)
    /// * `msb` - Upper byte (bits 15-8)
    pub const fn from_bytes(lsb: u8, msb: u8) -> Self {
        Self {
            raw: u16::from_le_bytes([lsb, msb]),
        }
    }

    /// Get the raw 16-bit value
    pub const fn raw(&self) -> u16 {
        self.raw
    }

    /// Get the Security mode (bits 12-8, 5 bits)
    ///
    /// The Security mode defines the applied set of security mechanisms.
    /// See EN 13757-7:2018 Table 19 for Security mode definitions.
    pub const fn security_mode(&self) -> SecurityMode {
        let mode_bits = ((self.raw >> 8) & 0b0001_1111) as u8;
        SecurityMode::from_bits(mode_bits)
    }

    /// Get the lower mode-specific byte (bits 7-0)
    ///
    /// The meaning of these bits depends on the selected Security mode.
    pub const fn mode_specific_lower(&self) -> u8 {
        (self.raw & 0xFF) as u8
    }

    /// Get the upper mode-specific bits (bits 15-13)
    ///
    /// The meaning of these bits depends on the selected Security mode.
    pub const fn mode_specific_upper(&self) -> u8 {
        ((self.raw >> 13) & 0b0000_0111) as u8
    }
}

impl From<u16> for ConfigurationField {
    fn from(value: u16) -> Self {
        Self { raw: value }
    }
}

impl From<ConfigurationField> for u16 {
    fn from(cf: ConfigurationField) -> Self {
        cf.raw
    }
}

#[cfg(feature = "std")]
impl fmt::Display for ConfigurationField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Configuration Field: 0x{:04X} (Security mode: {})",
            self.raw,
            self.security_mode()
        )
    }
}

impl fmt::Debug for ConfigurationField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigurationField")
            .field("mode", &self.security_mode())
            .finish()
    }
}
