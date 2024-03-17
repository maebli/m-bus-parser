use super::data_information::FunctionField;
use super::data_information::{self, DataInformation};
use super::value_information::{self, Unit, ValueInformation};
use super::DataRecords;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DataRecord {
    pub function: FunctionField,
    pub storage_number: u64,
    pub unit: Unit,
    pub exponent: Exponent,
    pub quantity: Quantity,
    pub value: f64,
    pub size: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Exponent {
    pub inner: Option<isize>,
}
impl From<isize> for Exponent {
    fn from(value: isize) -> Self {
        Exponent { inner: Some(value) }
    }
}

impl From<&ValueInformation> for Exponent {
    fn from(value_information: &ValueInformation) -> Exponent {
        match value_information {
            ValueInformation::Primary(x) => match x & 0x7F {
                0..=7 | 0x18..=0x1F | 0x28..=0x2F | 0x50..=0x57 => {
                    Exponent::from((x & 0b111) as isize - 3)
                }
                8..=15 | 0x30..=0x37 => Exponent::from((x & 0b111) as isize),
                0x10..=0x17 | 0x38..=0x3F | 0x6C..=0x6D => Exponent::from((x & 0b111) as isize - 6),
                0x20..=0x27 | 0x74..=0x77 => Exponent::from(1),
                0x40..=0x47 => Exponent::from((x & 0b111) as isize - 7),
                0x48..=0x4F => Exponent::from((x & 0b111) as isize - 9),
                0x58..=0x6B => Exponent::from((x & 0b11) as isize - 3),
                0x6E..=0x6F | 0x78 => Exponent { inner: None },
                _ => todo!("Implement the rest of the units: {:?}", x),
            },
            ValueInformation::PlainText => todo!(),
            ValueInformation::Extended(_) => todo!(),
            ValueInformation::Any => todo!(),
            ValueInformation::ManufacturerSpecific => todo!(),
        }
    }
}

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum Quantity {
    /* TODO */
    Some,
}

#[derive(Debug, PartialEq)]
pub enum DataRecordError {
    DataInformationError(data_information::DataInformationError),
}

impl From<data_information::DataInformationError> for DataRecordError {
    fn from(error: data_information::DataInformationError) -> Self {
        DataRecordError::DataInformationError(error)
    }
}

impl From<value_information::ValueInformationError> for DataRecordError {
    fn from(_error: value_information::ValueInformationError) -> Self {
        DataRecordError::DataInformationError(data_information::DataInformationError::NoData)
    }
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let data_information = DataInformation::try_from(data)?;
        let value_information = ValueInformation::try_from(&data[1..])?;
        let mut total_size = data_information.get_size() + value_information.get_size();
        let current_index = total_size - 1;
        match value_information {
            ValueInformation::Primary(_) => {
                let value = match data_information.data_field_coding {
                    data_information::DataFieldCoding::Integer8Bit => {
                        total_size += 1;
                        data[current_index] as f64
                    }
                    data_information::DataFieldCoding::Integer16Bit => {
                        total_size += 2;
                        ((data[current_index + 1] as u16) << 8 | data[current_index] as u16) as f64
                    }
                    data_information::DataFieldCoding::Integer24Bit => {
                        total_size += 3;
                        ((data[current_index + 3] as u32) << 16
                            | (data[current_index + 2] as u32) << 8
                            | data[current_index + 1] as u32) as f64
                    }
                    _ => 0.0,
                };
                Ok(DataRecord {
                    function: data_information.function_field,
                    storage_number: data_information.storage_number,
                    unit: Unit::try_from(&value_information)?,
                    exponent: Exponent::from(&value_information),
                    quantity: Quantity::Some,
                    value,
                    size: total_size,
                })
            }
            _ => Err(DataRecordError::DataInformationError(
                data_information::DataInformationError::NoData,
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum VariableUserDataError {
    DataInformationError(DataRecordError),
}

impl From<DataRecordError> for VariableUserDataError {
    fn from(error: DataRecordError) -> Self {
        VariableUserDataError::DataInformationError(error)
    }
}

impl TryFrom<&[u8]> for DataRecords {
    type Error = VariableUserDataError;
    fn try_from(data: &[u8]) -> Result<DataRecords, VariableUserDataError> {
        let mut records = DataRecords::new();
        let mut offset = 0;
        let mut _more_records_follow = false;

        while offset < data.len() {
            match data[offset] {
                0x0F => {
                    /* TODO: parse manufacturer specific */
                    offset = data.len();
                }
                0x1F => {
                    /* TODO: parse manufacturer specific */
                    _more_records_follow = true;
                    offset = data.len();
                }
                0x2F => {
                    offset += 1;
                }
                _ => {
                    let _ = records.add_record(DataRecord::try_from(&data[offset..])?);
                    offset += records.last().unwrap().size;
                }
            }
        }

        Ok(records)
    }
}

mod tests {

    #[test]
    fn test_parse_variable_data() {
        use crate::user_data::variable_user_data::Exponent;
        use crate::user_data::{
            data_information::FunctionField, value_information::Unit, variable_user_data::Quantity,
            DataRecord, DataRecords,
        };
        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        /* DIF = 0x03, VIF = 0x13, Value = 0x153100 */
        let data = &[0x03, 0x13, 0x15, 0x31, 0x00];

        let result = DataRecords::try_from(data.as_slice());
        assert_eq!(
            result.unwrap().get(0),
            Some(&DataRecord {
                function: FunctionField::InstantaneousValue,
                storage_number: 0,
                unit: Unit::CubicMeter,
                exponent: Exponent::from(-3),
                quantity: Quantity::Some,
                value: 12565.0,
                size: 5,
            })
        );
    }

    fn _test_parse_variable_data2() {
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let _data = &[0xDA, 0x02, 0x3B, 0x13, 0x01];
    }

    fn _test_parse_variable_data3() {
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let _data = &[0x8B, 0x60, 0x04, 0x37, 0x18, 0x02];
    }
}
