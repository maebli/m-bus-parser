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
