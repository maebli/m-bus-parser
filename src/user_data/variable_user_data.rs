use super::value_information::ValueInformation;
use super::data_information::{FunctionField, Unit};

#[derive(Debug, Clone,PartialEq)]
pub struct DataRecord {
    function: FunctionField,
    storage_number: u32,
    unit: Unit,
    quantity: String,
    value: f64,
}

#[derive(Debug, PartialEq)]
pub enum VariableUserDataError{
}


pub fn parse_variable_data(data: &[u8]) -> Result<Vec<DataRecord>,VariableUserDataError> {
    let mut records = Vec::new();
    let mut data = data;

    let vif = ValueInformation::new(data);
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
