use super::value_information::ValueInformation;
use super::data_information::{self, DataInformation};
use super::data_information::{FunctionField, Unit};

#[derive(Debug, Clone,PartialEq)]
pub struct DataRecord {
    function: FunctionField,
    storage_number: u64,
    unit: Unit,
    quantity: String,
    value: f64,
}

enum DataRecordError {
    DataInformationError(data_information::DataInformationError),
}

impl From<data_information::DataInformationError> for DataRecordError {
    fn from(error: data_information::DataInformationError) -> Self {
        DataRecordError::DataInformationError(error)
    }
}

impl DataRecord {
    pub fn new(data: &[u8]) -> Result<DataRecord,DataRecordError> {

        let data_information = DataInformation::new(data)?;
        let value_information = ValueInformation::new(data);

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
            quantity: "Volume".to_string(),
            value: 0.0,
        })

    }
}

#[derive(Debug, PartialEq)]
pub enum VariableUserDataError{}


pub fn parse_variable_data(data: &[u8]) -> Result<Vec<DataRecord>,VariableUserDataError> {

    let mut records = Vec::new();

    let mut offset = 0;
    let mut more_records_follow= false ;

    while offset < data.len() {
        match data[offset] {
            0x0F  => {
                // manufacturer specific
                offset = data.len();
            },
            0x1F => {
                // manufacturer specific
                more_records_follow = true;
                offset = data.len();
            },
            0x2F => {
                // filler byte, can be skipped
                offset += 1;
            },

            _ => {}
        }

        let next_data_record = DataRecord::new(&data[offset..]);


        offset += 1;
    }

    Ok(records)
}


mod tests {

    use super::*;
    #[test]
    fn test_parse_vafriable_data() {
        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        let data = vec![
            0x03, 0x13, 0x15, 0x31, 0x00
        ];
    
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));
    }

    fn test_parse_variable_data2(){
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let data = vec![
            0xDA, 0x02, 0x3B, 0x13, 0x01
        ];
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));
    }

    fn test_parse_variable_data3(){
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = vec![
            0x8B, 0x60, 0x04, 0x37, 0x18, 0x02
        ];
        let result = parse_variable_data(&data);
        assert_eq!(result, Ok(vec![]));

    }
}
