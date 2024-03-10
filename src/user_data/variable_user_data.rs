use super::data_information::{self, DataInformation};
use super::data_information::{FunctionField, Unit};
use super::value_information::ValueInformation;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DataRecord {
    function: FunctionField,
    storage_number: u64,
    unit: Unit,
    quantity: Quantity,
    value: f64,
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

impl DataRecord {
    pub fn new(data: &[u8]) -> Result<DataRecord, DataRecordError> {
        let data_information = DataInformation::new(data)?;
        let _value_information = ValueInformation::new(data);

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

// Maximum 234 bytes for variable data blocks, each block consists of a minimum of 2 bytes
// therefore the maximum number of blocks is 117, see https://m-bus.com/documentation-wired/06-application-layer
const MAXIMUM_VARIABLE_DATA_BLOCKS: usize = 117;

pub fn parse_variable_data(
    data: &[u8],
    records: &mut [Option<DataRecord>; MAXIMUM_VARIABLE_DATA_BLOCKS],
) -> Result<usize, VariableUserDataError> {
    let mut count = 0;
    let mut offset = 0;
    let mut more_records_follow = false;

    while offset < data.len() && count < records.len() {
        match data[offset] {
            0x0F => {
                offset = data.len();
            }
            0x1F => {
                more_records_follow = true;
                offset = data.len();
            }
            0x2F => {
                offset += 1;
            }
            _ => {
                // As we don't allocate memory dynamically, ensure we do not exceed the array.
                if count < MAXIMUM_VARIABLE_DATA_BLOCKS {
                    records[count] = Some(DataRecord::new(&data[offset..offset + 1])?);
                    count += 1;
                }
                offset += 1; // Adjust this based on how you process records.
            }
        }
    }

    Ok(count)
}

mod tests {

    use super::*;
    fn test_parse_vafriable_data() {
        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        let data = &[0x03, 0x13, 0x15, 0x31, 0x00];
        let mut records = [None; MAXIMUM_VARIABLE_DATA_BLOCKS];

        let result = parse_variable_data(data, &mut records);
        assert_eq!(result, Ok(5));
    }

    fn test_parse_variable_data2() {
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let data = &[0xDA, 0x02, 0x3B, 0x13, 0x01];
        let mut records = [None; MAXIMUM_VARIABLE_DATA_BLOCKS];
        let result = parse_variable_data(data, &mut records);
        assert_eq!(result, Ok(5));
    }

    fn test_parse_variable_data3() {
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = &[0x8B, 0x60, 0x04, 0x37, 0x18, 0x02];
        let mut records = [None; MAXIMUM_VARIABLE_DATA_BLOCKS];

        let result = parse_variable_data(data, &mut records);
        assert_eq!(result, Ok(5));
    }
}
