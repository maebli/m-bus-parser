use std::collections::HashMap;

use super::{exact_significand, hex_string, record_offset_power10, scaled_decimal, ValueOutput};
use crate::user_data::{data_information::DataType, DataRecord};

#[derive(Default)]
pub(crate) struct ProfileHistory<'a> {
    bases: HashMap<(u8, &'a [u8]), i128>,
}

impl<'a> ProfileHistory<'a> {
    pub(crate) fn observe(&mut self, record: &DataRecord<'a>) -> Option<ValueOutput> {
        let header = record
            .raw_bytes
            .get(..record.data_record_header.get_size())?;
        let (&dif, fields) = header.split_first()?;
        let register = dif & 0xf0;
        match record.value()? {
            DataType::Number(number) if record.data_information()?.storage_number != 0 => {
                if let Some(value) = exact_significand(record, *number).and_then(|n| n.parse().ok())
                {
                    self.bases.insert((register, fields), value);
                } else {
                    self.bases.remove(&(register, fields));
                }
                None
            }
            DataType::InverseCompactProfile(profile) => {
                let Some(differences) = profile.monthly_bcd_differences() else {
                    return Some(ValueOutput {
                        kind: "inverse_compact_profile".into(),
                        value: Some(serde_json::json!({ "raw_hex": hex_string(profile.bytes) })),
                    });
                };
                let mut base_fields = fields.strip_suffix(&[0x13])?.to_vec();
                *base_fields.last_mut()? &= 0x7f;
                let mut reading = self.bases.get(&(register, base_fields.as_slice())).copied();
                let has_base = reading.is_some();
                let scale = record.value_information()?.decimal_scale_exponent;
                let offset = record_offset_power10(record);
                let values: Vec<_> = differences
                    .map(|difference| {
                        let difference = difference.map(i128::from);
                        if has_base {
                            reading = reading
                                .zip(difference)
                                .and_then(|(value, change)| value.checked_sub(change));
                            Some(scaled_decimal(&reading?.to_string(), scale, offset))
                        } else {
                            Some(scaled_decimal(&difference?.to_string(), scale, None))
                        }
                    })
                    .collect();
                let field = if has_base { "readings" } else { "differences" };
                Some(ValueOutput {
                    kind: "inverse_compact_profile".into(),
                    value: Some(serde_json::json!({ "interval": "1 month", field: values })),
                })
            }
            _ => None,
        }
    }
}
