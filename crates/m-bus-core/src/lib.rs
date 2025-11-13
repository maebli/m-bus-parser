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
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum DeviceType {
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

impl From<u8> for DeviceType {
    fn from(byte: u8) -> Self {
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
