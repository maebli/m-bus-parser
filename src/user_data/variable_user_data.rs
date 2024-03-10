use arrayvec::ArrayVec;

use super::data_information::{self, DataInformation};
use super::data_information::{FunctionField, Unit};
use super::value_information::{self, ValueInformation};
use super::MAXIMUM_VARIABLE_DATA_BLOCKS;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DataRecord {
    function: FunctionField,
    storage_number: u64,
    unit: Unit,
    quantity: Quantity,
    value: f64,
    size: usize,
}

#[derive(Debug, Copy, PartialEq, Clone)]
enum Quantity {
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
    fn from(error: value_information::ValueInformationError) -> Self {
        DataRecordError::DataInformationError(data_information::DataInformationError::NoData)
    }
}

impl TryFrom<&[u8]> for DataRecord {
    type Error = DataRecordError;
    fn try_from(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let data_information = DataInformation::try_from(data)?;
        let value_information = ValueInformation::try_from(data)?;
        let size = data_information.size + value_information.get_size();

        let storage_number = data_information.storage_number;

        let function = match data_information.function_field {
            FunctionField::InstantaneousValue => FunctionField::InstantaneousValue,
            FunctionField::MaximumValue => FunctionField::MaximumValue,
            FunctionField::MinimumValue => FunctionField::MinimumValue,
            FunctionField::ValueDuringErrorState => FunctionField::ValueDuringErrorState,
        };

        /* returning some dummy */
        Ok(DataRecord {
            function,
            storage_number,
            unit: Unit::WithoutUnits,
            quantity: Quantity::Some,
            value: 0.0,
            size,
        })
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

pub fn parse_variable_data(
    data: &[u8],
) -> Result<ArrayVec<DataRecord, MAXIMUM_VARIABLE_DATA_BLOCKS>, VariableUserDataError> {
    let mut records = ArrayVec::new();
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
                records.push(DataRecord::try_from(&data[offset..])?);
                offset += records.last().unwrap().size;
            }
        }
    }

    Ok(records)
}

mod tests {

    use super::*;

    #[test]
    fn test_parse_vafriable_data() {
        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        let data = &[0x03, 0x13];

        let result = parse_variable_data(data);
        assert_eq!(result, Ok(ArrayVec::new()));
        assert_eq!(
            result.unwrap()[0],
            DataRecord {
                function: FunctionField::InstantaneousValue,
                storage_number: 0,
                unit: Unit::WithoutUnits,
                quantity: Quantity::Some,
                value: 0.0,
                size: 2,
            }
        );
    }

    fn test_parse_variable_data2() {
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let data = &[0xDA, 0x02, 0x3B, 0x13, 0x01];
        let result = parse_variable_data(data);
        assert_eq!(result, Ok(ArrayVec::new()));
    }

    fn test_parse_variable_data3() {
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = &[0x8B, 0x60, 0x04, 0x37, 0x18, 0x02];

        let result = parse_variable_data(data);
        assert_eq!(result, Ok(ArrayVec::new()));
    }
}
