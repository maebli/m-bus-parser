use super::data_information::{self};
use super::variable_user_data::DataRecordError;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub struct DataInformationBlock<'a> {
    pub data_information_field: DataInformationField,
    pub data_information_field_extension: Option<DataInformationFieldExtensions<'a>>,
}

impl DataInformationBlock<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.data_information_field_extension {
            size += vife.len();
        }
        size
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct DataInformationField {
    pub data: u8,
}

impl From<data_information::DataInformationError> for DataRecordError {
    fn from(error: data_information::DataInformationError) -> Self {
        Self::DataInformationError(error)
    }
}

impl From<u8> for DataInformationField {
    fn from(data: u8) -> Self {
        Self { data }
    }
}

impl From<u8> for DataInformationFieldExtension {
    fn from(data: u8) -> Self {
        Self { data }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
#[repr(transparent)]
pub struct DataInformationFieldExtension {
    pub data: u8,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DataInformationFieldExtensions<'a>(&'a [u8]);
impl<'a> DataInformationFieldExtensions<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self(data)
    }
}

impl<'a> Iterator for DataInformationFieldExtensions<'a> {
    type Item = DataInformationFieldExtension;
    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(DataInformationFieldExtension { data: *head })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}
impl<'a> ExactSizeIterator for DataInformationFieldExtensions<'a> {}
impl<'a> DoubleEndedIterator for DataInformationFieldExtensions<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (end, start) = self.0.split_last()?;
        self.0 = start;
        Some(DataInformationFieldExtension { data: *end })
    }
}

impl<'a> TryFrom<&'a [u8]> for DataInformationBlock<'a> {
    type Error = DataInformationError;

    fn try_from(data: &'a [u8]) -> Result<Self, DataInformationError> {
        let Some((dif_byte, data)) = data.split_first() else {
            return Err(DataInformationError::NoData);
        };
        let dif = DataInformationField::from(*dif_byte);

        let length = data.iter().take_while(|&&u8| u8 & 0x80 != 0).count();
        let offset = length + 1;
        match () {
            () if dif.has_extension() && offset > MAXIMUM_DATA_INFORMATION_SIZE => {
                Err(DataInformationError::DataTooLong)
            }
            () if dif.has_extension() && offset > data.len() => {
                Err(DataInformationError::DataTooShort)
            }
            () if dif.has_extension() => Ok(DataInformationBlock {
                data_information_field: dif,
                data_information_field_extension: Some(DataInformationFieldExtensions::new(
                    data.get(..offset)
                        .ok_or(DataInformationError::DataTooShort)?,
                )),
            }),
            () => Ok(DataInformationBlock {
                data_information_field: dif,
                data_information_field_extension: None,
            }),
        }
    }
}

impl DataInformationField {
    const fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

impl DataInformationFieldExtension {
    const fn special_function(&self) -> SpecialFunctions {
        match self.data {
            0x0F => SpecialFunctions::ManufacturerSpecific,
            0x1F => SpecialFunctions::MoreRecordsFollow,
            0x2F => SpecialFunctions::IdleFiller,
            0x7F => SpecialFunctions::GlobalReadoutRequest,
            _ => SpecialFunctions::Reserved,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInformation {
    pub storage_number: u64,
    pub function_field: FunctionField,
    pub data_field_coding: DataFieldCoding,
    pub data_information_extension: Option<DataInformationExtensionField>,
    pub size: usize,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DataInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{}",
            self.storage_number, self.function_field, self.data_field_coding
        )
    }
}

const MAXIMUM_DATA_INFORMATION_SIZE: usize = 11;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInformationExtensionField {}

#[derive(Debug, PartialEq)]
pub enum DataInformationError {
    NoData,
    DataTooLong,
    DataTooShort,
    InvalidValueInformation,
}

impl TryFrom<&DataInformationBlock<'_>> for DataInformation {
    type Error = DataInformationError;

    fn try_from(
        data_information_block: &DataInformationBlock,
    ) -> Result<Self, DataInformationError> {
        let dif = data_information_block.data_information_field.data;
        let possible_difes = &data_information_block.data_information_field_extension;
        let mut storage_number = u64::from((dif & 0b0100_0000) >> 6);

        let mut extension_bit = dif & 0x80 != 0;
        let mut extension_index = 1;
        let mut _tariff = 0;
        let mut _sub_unit = 0;
        let mut first_dife = None;

        if let Some(difes) = possible_difes {
            first_dife = difes.clone().next();
            for dife in difes.clone() {
                if extension_index > MAXIMUM_DATA_INFORMATION_SIZE {
                    return Err(DataInformationError::DataTooLong);
                }
                let dife = dife.data;
                storage_number += u64::from(dife & 0x0f) << ((extension_index * 4) + 1);
                _sub_unit += u32::from((dife & 0x40) >> 6) << extension_index;
                _tariff += u64::from((dife & 0x30) >> 4) << (extension_index * 2);
                extension_bit = dife & 0x80 != 0;
                extension_index += 1;
            }
        }

        let function_field = match (dif & 0b0011_0000) >> 4 {
            0b00 => FunctionField::InstantaneousValue,
            0b01 => FunctionField::MaximumValue,
            0b10 => FunctionField::MinimumValue,
            _ => FunctionField::ValueDuringErrorState,
        };
        let data_field_coding = match dif & 0b0000_1111 {
            0b0000 => DataFieldCoding::NoData,
            0b0001 => DataFieldCoding::Integer8Bit,
            0b0010 => DataFieldCoding::Integer16Bit,
            0b0011 => DataFieldCoding::Integer24Bit,
            0b0100 => DataFieldCoding::Integer32Bit,
            0b0101 => DataFieldCoding::Real32Bit,
            0b0110 => DataFieldCoding::Integer48Bit,
            0b0111 => DataFieldCoding::Integer64Bit,
            0b1000 => DataFieldCoding::SelectionForReadout,
            0b1001 => DataFieldCoding::BCD2Digit,
            0b1010 => DataFieldCoding::BCD4Digit,
            0b1011 => DataFieldCoding::BCD6Digit,
            0b1100 => DataFieldCoding::BCD8Digit,
            0b1101 => DataFieldCoding::VariableLength,
            0b1110 => DataFieldCoding::BCDDigit12,
            0b1111 => DataFieldCoding::SpecialFunctions(
                first_dife
                    .ok_or(DataInformationError::DataTooShort)?
                    .special_function(),
            ),
            _ => unreachable!(), // This case should never occur due to the 4-bit width
        };

        Ok(Self {
            storage_number,
            function_field,
            data_field_coding,
            data_information_extension: if extension_bit {
                Some(DataInformationExtensionField {})
            } else {
                None
            },
            size: extension_index,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "String"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextUnit<'a>(&'a [u8]);
impl<'a> TextUnit<'a> {
    pub const fn new(input: &'a [u8]) -> Self {
        Self(input)
    }
}

impl PartialEq<str> for TextUnit<'_> {
    fn eq(&self, other: &str) -> bool {
        self.0.iter().eq(other.as_bytes().iter().rev())
    }
}
#[cfg(any(feature = "serde", feature = "std"))]
impl From<TextUnit<'_>> for String {
    fn from(value: TextUnit<'_>) -> Self {
        let value: Vec<u8> = value.0.iter().copied().rev().collect();
        String::from_utf8(value).unwrap_or_default()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[cfg(feature = "std")]
impl std::fmt::Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Month::January => write!(f, "Jan"),
            Month::February => write!(f, "Feb"),
            Month::March => write!(f, "Mar"),
            Month::April => write!(f, "Apr"),
            Month::May => write!(f, "May"),
            Month::June => write!(f, "Jun"),
            Month::July => write!(f, "Jul"),
            Month::August => write!(f, "Aug"),
            Month::September => write!(f, "Sep"),
            Month::October => write!(f, "Oct"),
            Month::November => write!(f, "Nov"),
            Month::December => write!(f, "Dec"),
        }
    }
}

pub type Year = u16;
pub type DayOfMonth = u8;
pub type Hour = u8;
pub type Minute = u8;
pub type Second = u8;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum SingleEveryOrInvalid<T> {
    Single(T),
    Every(),
    Invalid(),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub enum DataType<'a> {
    Text(TextUnit<'a>),
    Number(f64),
    Date(
        SingleEveryOrInvalid<DayOfMonth>,
        SingleEveryOrInvalid<Month>,
        SingleEveryOrInvalid<Year>,
    ),
    Time(
        SingleEveryOrInvalid<Second>,
        SingleEveryOrInvalid<Minute>,
        SingleEveryOrInvalid<Hour>,
    ),
    DateTime(
        SingleEveryOrInvalid<DayOfMonth>,
        SingleEveryOrInvalid<Month>,
        SingleEveryOrInvalid<Year>,
        SingleEveryOrInvalid<Hour>,
        SingleEveryOrInvalid<Minute>,
    ),
    DateTimeWithSeconds(
        SingleEveryOrInvalid<DayOfMonth>,
        SingleEveryOrInvalid<Month>,
        SingleEveryOrInvalid<Year>,
        SingleEveryOrInvalid<Hour>,
        SingleEveryOrInvalid<Minute>,
        SingleEveryOrInvalid<Second>,
    ),
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(PartialEq, Debug)]
pub struct Data<'a> {
    value: Option<DataType<'a>>,
    size: usize,
}

#[cfg(feature = "std")]
impl<T: std::fmt::Display> std::fmt::Display for SingleEveryOrInvalid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingleEveryOrInvalid::Single(value) => write!(f, "{}", value),
            SingleEveryOrInvalid::Every() => write!(f, "Every"),
            SingleEveryOrInvalid::Invalid() => write!(f, "Invalid"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for Data<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(value) => match value {
                &DataType::Text(text) => {
                    let text: String = text.into();
                    write!(f, "{}", text)
                }
                DataType::Number(value) => write!(f, "{}", value),
                DataType::Date(day, month, year) => write!(f, "{}/{}/{}", day, month, year),
                DataType::DateTime(day, month, year, hour, minute) => {
                    write!(f, "{}/{}/{} {}:{}:00", day, month, year, hour, minute)
                }
                DataType::DateTimeWithSeconds(day, month, year, hour, minute, second) => {
                    write!(
                        f,
                        "{}/{}/{} {}:{}:{}",
                        day, month, year, hour, minute, second
                    )
                }
                DataType::Time(seconds, minutes, hours) => {
                    write!(f, "{}:{}:{}", hours, minutes, seconds)
                }
            },
            None => write!(f, "No Data"),
        }
    }
}

impl Data<'_> {
    #[must_use]
    pub const fn get_size(&self) -> usize {
        self.size
    }
}

macro_rules! parse_single_or_every {
    ($input:expr, $mask:expr, $all_value:expr, $shift:expr) => {
        if $input & $mask == $all_value {
            SingleEveryOrInvalid::Every()
        } else {
            SingleEveryOrInvalid::Single(($input & $mask) >> $shift)
        }
    };
}

macro_rules! parse_month {
    ($input:expr) => {
        match $input & 0xF {
            0x1 => SingleEveryOrInvalid::Single(Month::January),
            0x2 => SingleEveryOrInvalid::Single(Month::February),
            0x3 => SingleEveryOrInvalid::Single(Month::March),
            0x4 => SingleEveryOrInvalid::Single(Month::April),
            0x5 => SingleEveryOrInvalid::Single(Month::May),
            0x6 => SingleEveryOrInvalid::Single(Month::June),
            0x7 => SingleEveryOrInvalid::Single(Month::July),
            0x8 => SingleEveryOrInvalid::Single(Month::August),
            0x9 => SingleEveryOrInvalid::Single(Month::September),
            0xA => SingleEveryOrInvalid::Single(Month::October),
            0xB => SingleEveryOrInvalid::Single(Month::November),
            0xC => SingleEveryOrInvalid::Single(Month::December),
            _ => SingleEveryOrInvalid::Invalid(),
        }
    };
}

macro_rules! parse_year {
    ($input:expr, $mask_byte1:expr, $mask_byte2:expr, $all_value:expr) => {{
        let byte1 = u16::from($input.get(1).copied().unwrap_or(0) & $mask_byte1);
        let byte2 = u16::from($input.get(0).copied().unwrap_or(0) & $mask_byte2);
        let year = byte1.wrapping_shr(1) | byte2.wrapping_shr(5);
        if year == $all_value {
            SingleEveryOrInvalid::Every()
        } else {
            SingleEveryOrInvalid::Single(year)
        }
    }};
}
fn bcd_to_value_internal(
    data: &[u8],
    num_digits: usize,
    sign: i32,
) -> Result<Data, DataRecordError> {
    if data.len() < (num_digits + 1) / 2 {
        return Err(DataRecordError::InsufficientData);
    }

    let mut data_value = 0.0;
    let mut current_weight = 1.0;

    for i in 0..num_digits {
        let byte = data.get(i / 2).ok_or(DataRecordError::InsufficientData)?;

        let digit = if i % 2 == 0 {
            byte & 0x0F
        } else {
            (byte >> 4) & 0x0F
        };

        data_value += f64::from(digit) * current_weight;
        current_weight *= 10.0;
    }

    let signed_value = data_value * sign as f64;

    Ok(Data {
        value: Some(DataType::Number(signed_value)),
        size: (num_digits + 1) / 2,
    })
}

impl DataFieldCoding {
    pub fn parse<'a>(&self, input: &'a [u8]) -> Result<Data<'a>, DataRecordError> {
        macro_rules! bcd_to_value {
            ($data:expr, $num_digits:expr) => {{
                bcd_to_value_internal($data, $num_digits, 1)
            }};

            ($data:expr, $num_digits:expr, $sign:expr) => {{
                bcd_to_value_internal($data, $num_digits, $sign)
            }};
        }

        macro_rules! integer_to_value {
            ($data:expr, $byte_size:expr) => {{
                if $data.len() < $byte_size {
                    return Err(DataRecordError::InsufficientData);
                }
                let mut data_value = 0u32;
                let mut shift = 0;
                for byte in $data.iter().take($byte_size) {
                    data_value |= (*byte as u32) << shift;
                    shift += 8;
                }
                Ok(Data {
                    value: Some(DataType::Number(f64::from(data_value))),
                    size: $byte_size,
                })
            }};
        }
        match self {
            Self::NoData => Ok(Data {
                value: None,
                size: 0,
            }),
            Self::Integer8Bit => integer_to_value!(input, 1),
            Self::Integer16Bit => integer_to_value!(input, 2),
            Self::Integer24Bit => integer_to_value!(input, 3),
            Self::Integer32Bit => integer_to_value!(input, 4),
            Self::Integer48Bit => integer_to_value!(input, 6),
            Self::Integer64Bit => integer_to_value!(input, 8),

            Self::Real32Bit => {
                if input.len() < 4 {
                    return Err(DataRecordError::InsufficientData);
                }
                if let Ok(x) = input
                    .get(0..4)
                    .ok_or(DataRecordError::InsufficientData)?
                    .try_into()
                {
                    let x: [u8; 4] = x;
                    Ok(Data {
                        value: Some(DataType::Number(f64::from(f32::from_be_bytes(x)))),
                        size: 4,
                    })
                } else {
                    Err(DataRecordError::InsufficientData)
                }
            }

            Self::SelectionForReadout => Ok(Data {
                value: None,
                size: 0,
            }),

            Self::BCD2Digit => bcd_to_value!(input, 2),
            Self::BCD4Digit => bcd_to_value!(input, 4),
            Self::BCD6Digit => bcd_to_value!(input, 6),
            Self::BCD8Digit => bcd_to_value!(input, 8),
            Self::BCDDigit12 => bcd_to_value!(input, 12),

            Self::VariableLength => {
                let mut length = *input.first().ok_or(DataRecordError::InsufficientData)?;
                match length {
                    0x00..=0xBF => Ok(Data {
                        value: Some(DataType::Text(TextUnit::new(
                            input
                                .get(1..(length as usize))
                                .ok_or(DataRecordError::InsufficientData)?,
                        ))),
                        size: length as usize + 1,
                    }),
                    0xC0..=0xD9 => {
                        length -= 0xC0;
                        let is_negative =
                            *input.first().ok_or(DataRecordError::InsufficientData)? > 0xC9;
                        let sign = if is_negative { -1 } else { 1 };
                        bcd_to_value!(input, length as usize, sign)
                    }
                    0xE0..=0xE9 => {
                        length -= 0xE0;
                        todo!("0xE0-0xE9 not implemented for length {}", length);
                    }
                    0xF0..=0xF4 => {
                        length -= 0xF0;
                        todo!("0xF0-0xF4 not implemented for length {}", length);
                    }
                    0xF5 => {
                        length = 6;
                        todo!("0xF5 not implemented for length {}", length);
                    }
                    0xF6 => {
                        length = 8;
                        todo!("0xF6 not implemented for length {}", length);
                    }
                    _ => {
                        todo!(
                            "Variable length parsing for length: {} is a resreved value",
                            length
                        );
                    }
                }
            }

            Self::SpecialFunctions(_code) => {
                // Special functions parsing based on the code
                todo!()
            }

            Self::DateTypeG => {
                let day = parse_single_or_every!(
                    input.first().ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0,
                    0
                );
                let month = parse_month!(input.get(1).ok_or(DataRecordError::InsufficientData)?);
                let year = parse_year!(input, 0xF0, 0xE0, 0x7F);

                Ok(Data {
                    value: Some(DataType::Date(day, month, year)),
                    size: 2,
                })
            }
            Self::DateTimeTypeF => {
                let minutes = parse_single_or_every!(
                    input.first().ok_or(DataRecordError::InsufficientData)?,
                    0x3F,
                    0x3F,
                    0
                );
                let hour = parse_single_or_every!(
                    input.get(1).ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0x1F,
                    0
                );
                let day = parse_single_or_every!(
                    input.get(2).ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0x1F,
                    0
                );
                let month = parse_month!(input.get(3).ok_or(DataRecordError::InsufficientData)?);
                let year = parse_year!(input, 0xF0, 0xE0, 0x7F);

                Ok(Data {
                    value: Some(DataType::DateTime(day, month, year, hour, minutes)),
                    size: 4,
                })
            }
            Self::DateTimeTypeJ => {
                let seconds = parse_single_or_every!(
                    input.first().ok_or(DataRecordError::InsufficientData)?,
                    0x3F,
                    0x3F,
                    0
                );
                let minutes = parse_single_or_every!(
                    input.get(1).ok_or(DataRecordError::InsufficientData)?,
                    0x3F,
                    0x3F,
                    0
                );
                let hours = parse_single_or_every!(
                    input.get(2).ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0x1F,
                    0
                );

                Ok(Data {
                    value: Some(DataType::Time(seconds, minutes, hours)),
                    size: 4,
                })
            }
            Self::DateTimeTypeI => {
                // note: more information can be extracted from the data,
                // however, because this data can be derived from the other data that is
                // that is extracted, it is not necessary to extract it.

                let seconds = parse_single_or_every!(
                    input.first().ok_or(DataRecordError::InsufficientData)?,
                    0x3F,
                    0x3F,
                    0
                );

                let minutes = parse_single_or_every!(
                    input.get(1).ok_or(DataRecordError::InsufficientData)?,
                    0x3F,
                    0x3F,
                    0
                );

                let hours = parse_single_or_every!(
                    input.get(2).ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0x1F,
                    0
                );
                let days = parse_single_or_every!(
                    input.get(3).ok_or(DataRecordError::InsufficientData)?,
                    0x1F,
                    0x1F,
                    0
                );
                let months = parse_month!(input.get(4).ok_or(DataRecordError::InsufficientData)?);
                let year = parse_year!(input, 0xF0, 0xE0, 0x7F);

                Ok(Data {
                    value: Some(DataType::DateTimeWithSeconds(
                        days, months, year, hours, minutes, seconds,
                    )),
                    size: 6,
                })
            }
        }
    }
}

impl DataInformation {
    #[must_use]
    pub const fn get_size(&self) -> usize {
        self.size
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionField {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FunctionField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionField::InstantaneousValue => write!(f, "Inst"),
            FunctionField::MaximumValue => write!(f, "Max"),
            FunctionField::MinimumValue => write!(f, "Min"),
            FunctionField::ValueDuringErrorState => write!(f, "Value During Error State"),
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialFunctions {
    ManufacturerSpecific,
    MoreRecordsFollow,
    IdleFiller,
    Reserved,
    GlobalReadoutRequest,
}

pub struct Value {
    pub data: f64,
    pub byte_size: usize,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataFieldCoding {
    NoData,
    Integer8Bit,
    Integer16Bit,
    Integer24Bit,
    Integer32Bit,
    Real32Bit,
    Integer48Bit,
    Integer64Bit,
    SelectionForReadout,
    BCD2Digit,
    BCD4Digit,
    BCD6Digit,
    BCD8Digit,
    VariableLength,
    BCDDigit12,
    SpecialFunctions(SpecialFunctions),
    DateTypeG,
    DateTimeTypeF,
    DateTimeTypeJ,
    DateTimeTypeI,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DataFieldCoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataFieldCoding::NoData => write!(f, "No Data"),
            DataFieldCoding::Integer8Bit => write!(f, "8-bit Integer"),
            DataFieldCoding::Integer16Bit => write!(f, "16-bit Integer"),
            DataFieldCoding::Integer24Bit => write!(f, "24-bit Integer"),
            DataFieldCoding::Integer32Bit => write!(f, "32-bit Integer"),
            DataFieldCoding::Real32Bit => write!(f, "32-bit Real"),
            DataFieldCoding::Integer48Bit => write!(f, "48-bit Integer"),
            DataFieldCoding::Integer64Bit => write!(f, "64-bit Integer"),
            DataFieldCoding::SelectionForReadout => write!(f, "Selection for Readout"),
            DataFieldCoding::BCD2Digit => write!(f, "BCD 2-digit"),
            DataFieldCoding::BCD4Digit => write!(f, "BCD 4-digit"),
            DataFieldCoding::BCD6Digit => write!(f, "BCD 6-digit"),
            DataFieldCoding::BCD8Digit => write!(f, "BCD 8-digit"),
            DataFieldCoding::VariableLength => write!(f, "Variable Length"),
            DataFieldCoding::BCDDigit12 => write!(f, "BCD 12-digit"),
            DataFieldCoding::DateTypeG => write!(f, "Date Type G"),
            DataFieldCoding::DateTimeTypeF => write!(f, "Date Time Type F"),
            DataFieldCoding::DateTimeTypeJ => write!(f, "Date Time Type J"),
            DataFieldCoding::DateTimeTypeI => write!(f, "Date Time Type I"),
            DataFieldCoding::SpecialFunctions(code) => write!(f, "Special Functions ({:?})", code),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_data_information() {
        let data = [0x13_u8];
        let result = DataInformationBlock::try_from(data.as_slice());
        let result = DataInformation::try_from(&result.unwrap());
        assert_eq!(
            result,
            Ok(DataInformation {
                storage_number: 0,
                function_field: FunctionField::MaximumValue,
                data_field_coding: DataFieldCoding::Integer24Bit,
                data_information_extension: None,
                size: 1,
            })
        );
    }

    #[test]
    fn reverse_text_unit() {
        let original_value = [0x6c, 0x61, 0x67, 0x69];
        let parsed = TextUnit::new(&original_value);
        assert_eq!(&parsed, "igal");
    }

    #[test]
    fn test_invalid_data_information() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_longest_data_information_not_too_long() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        ];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_ne!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_short_data_information() {
        let data = [0xFF];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooShort));
    }

    #[test]
    fn test_data_inforamtion1() {
        let data = [178, 1];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_size(), 2);
    }

    #[test]
    fn test_bcd_to_value_unsigned() {
        let data = [0x54, 0x76, 0x98];
        let result = bcd_to_value_internal(&data, 6, 1);
        assert_eq!(
            result.unwrap(),
            Data {
                value: Some(DataType::Number(987654.0)),
                size: 3
            }
        );
    }
}
