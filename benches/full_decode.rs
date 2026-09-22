//! Shared full-decode workload for native timings and embedded footprint builds.
use core::hint::black_box;
use m_bus_parser::{mbus_data::MbusData, user_data::UserDataBlock, WiredFrame};

#[derive(Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Decoded {
    pub records: u32,
    pub errors: u32,
}

#[inline(never)]
pub fn decode(data: &[u8]) -> Decoded {
    let mut result = Decoded::default();
    let parsed = match MbusData::<WiredFrame>::try_from(data) {
        Ok(parsed) => parsed,
        Err(error) => {
            black_box(error);
            result.errors += 1;
            return result;
        }
    };
    if let Some(error) = parsed.application_error {
        black_box(error);
        result.errors += 1;
    }
    if matches!(
        parsed.user_data,
        Some(UserDataBlock::FixedDataStructure { .. })
    ) {
        result.records += 2;
    }
    black_box(parsed.user_data);
    if let Some(records) = parsed.data_records {
        for record in records {
            match record {
                Ok(record) => {
                    if let Some(info) = record.value_information() {
                        for label in info.labels() {
                            black_box(label);
                        }
                        for unit in info.units() {
                            black_box(unit);
                        }
                        black_box((info.decimal_scale_exponent, info.decimal_offset_exponent));
                    }
                    black_box(record);
                    result.records += 1;
                }
                Err(error) => {
                    black_box(error);
                    result.errors += 1;
                }
            }
        }
    }
    result
}
