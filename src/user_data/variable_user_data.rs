use super::data_information::{self};
use super::DataRecords;

#[derive(Debug, PartialEq)]
pub enum DataRecordError {
    DataInformationError(data_information::DataInformationError),
    InsufficientData,
}

#[derive(Debug, PartialEq)]
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
        DataRecords::new(data)
    }
}

mod tests {

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
}
