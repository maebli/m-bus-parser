use super::data_information::{self};
use super::{DataRecords, FixedDataHeader};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DataRecordError {
    DataInformationError(data_information::DataInformationError),
    InsufficientData,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum VariableUserDataError {
    DataInformationError(DataRecordError),
}

impl From<DataRecordError> for VariableUserDataError {
    fn from(error: DataRecordError) -> Self {
        Self::DataInformationError(error)
    }
}

impl<'a> From<&'a [u8]> for DataRecords<'a> {
    fn from(data: &'a [u8]) -> Self {
        DataRecords::new(data, None)
    }
}

impl<'a> From<(&'a [u8], &'a FixedDataHeader)> for DataRecords<'a> {
    fn from((data, fixed_data_header): (&'a [u8], &'a FixedDataHeader)) -> Self {
        DataRecords::new(data, Some(fixed_data_header))
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use crate::user_data::{data_information::DataFieldCoding, data_record::DataRecord};

    #[test]
    fn test_parse_variable_data_length() {
        use crate::user_data::data_information::DataFieldCoding;
        use crate::user_data::data_information::DataType;
        use crate::user_data::data_information::TextUnit;
        use crate::user_data::DataRecords;

        let data: &[u8] = &[
            0x0D, 0x06, 0xC1, 0x12, 0x0D, 0x06, 0xD3, 0x12, 0x34, 0x56, 0x0D, 0x06, 0x02, 0x31,
            0x32, 0x0D, 0x06, 0xE1, 0xFF, 0x0D, 0x06, 0x00,
        ];

        let records: Vec<DataRecord<'_>> = DataRecords::from(data).flatten().collect();

        assert_eq!(records.len(), 5);
        {
            let record = records.get(0).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Number(12.0))
        }
        {
            let record = records.get(1).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Number(-563412.0))
        }
        {
            let record = records.get(2).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[0x31, 0x32])))
        }
        {
            let record = records.get(3).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Number(-1.0))
        }
        {
            let record = records.get(4).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
    }

    #[test]
    fn test_parse_variable_lossy_data_length() {
        use crate::user_data::data_information::DataFieldCoding;
        use crate::user_data::data_information::DataType;
        use crate::user_data::data_information::TextUnit;
        use crate::user_data::DataRecords;

        let data: &[u8] = &[
            0x0D, 0x06, 0xE9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0D, 0x06,
            0x00, 0x0D, 0x06, 0xEF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0D, 0x06, 0x00, 0x0D, 0x06, 0xF4, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0x0D, 0x06, 0x00, 0x0D, 0x06, 0xF5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0D,
            0x06, 0x00, 0x0D, 0x06, 0xF6, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0D,
            0x06, 0x00,
        ];

        let records: Vec<DataRecord<'_>> = DataRecords::from(data).flatten().collect();

        assert_eq!(records.len(), 10);
        {
            let record = records.get(0).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::LossyNumber(-1.0))
        }
        {
            let record = records.get(1).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
        {
            let record = records.get(2).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::LossyNumber(-1.0))
        }
        {
            let record = records.get(3).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
        {
            let record = records.get(4).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::LossyNumber(-1.0))
        }
        {
            let record = records.get(5).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
        {
            let record = records.get(6).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::LossyNumber(-1.0))
        }
        {
            let record = records.get(7).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
        {
            let record = records.get(8).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::LossyNumber(-1.0))
        }
        {
            let record = records.get(9).unwrap();
            let code = get_data_field_coding(record);
            assert_eq!(code, DataFieldCoding::VariableLength);
            let value = record.data.value.clone().unwrap();
            assert_eq!(value, DataType::Text(TextUnit::new(&[])))
        }
    }

    #[test]
    fn test_parse_variable_data() {
        use crate::user_data::DataRecords;

        /* Data block 1: unit 0, storage No 0, no tariff, instantaneous volume, 12565 l (24 bit integer) */
        /* DIF = 0x03, VIF = 0x13, Value = 0x153100 */
        let data = &[0x03, 0x13, 0x15, 0x31, 0x00];

        let _result = DataRecords::try_from(data.as_slice());
    }

    #[test]
    fn test_parse_variable_data2() {
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let _data = &[0x01, 0xFD, 0x1B, 0x00];
    }

    /*  Out: PlainText : Unit "%RH"  Value:   33.96
    In: 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D*/
    #[cfg(feature = "plaintext-before-extension")]
    #[test]
    fn test_parse_variable_data3() {
        use crate::user_data::DataRecords;
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = &[0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D];
        let _data = DataRecords::try_from(data.as_slice());
    }

    /*  Out: PlainText : Unit "%RH"  Value:   33.96
    In: 0x02, 0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x44, 0x0D*/
    #[cfg(not(feature = "plaintext-before-extension"))]
    #[test]
    fn test_parse_variable_data3() {
        use crate::user_data::DataRecords;
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let data = &[0x02, 0xFC, 0x74, 0x03, 0x48, 0x52, 0x25, 0x44, 0x0D];
        let _data = DataRecords::try_from(data.as_slice());
    }

    const fn _test_parse_variable_data2() {
        /* Data block 2: unit 0, storage No 5, no tariff, maximum volume flow, 113 l/h (4 digit BCD) */
        let _data = &[0xDA, 0x02, 0x3B, 0x13, 0x01];
    }

    const fn _test_parse_variable_data3() {
        /* Data block 3: unit 1, storage No 0, tariff 2, instantaneous energy, 218,37 kWh (6 digit BCD) */
        let _data = &[0x8B, 0x60, 0x04, 0x37, 0x18, 0x02];
    }

    fn get_data_field_coding(record: &DataRecord) -> DataFieldCoding {
        record
            .data_record_header
            .processed_data_record_header
            .data_information
            .clone()
            .unwrap()
            .data_field_coding
    }
}
