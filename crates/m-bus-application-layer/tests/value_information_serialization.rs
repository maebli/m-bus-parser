#![cfg(feature = "serde")]

use m_bus_application_layer::value_information::{
    Unit, ValueInformation, ValueInformationBlock, ValueLabel,
};
use serde::Serialize;

// The original owned representation defines the binary field order and lengths.
#[derive(Serialize)]
struct OwnedValueInformation {
    decimal_offset_exponent: isize,
    labels: Vec<ValueLabel>,
    decimal_scale_exponent: isize,
    units: Vec<Unit>,
}

#[test]
fn binary_serialization_preserves_owned_layout_and_remaining_items() {
    for raw in [
        &[0x13][..],
        &[0x7f],
        &[0xfd, 0xd9, 0xfc, 0x01],
        &[
            0x93, 0xa0, 0xa0, 0xa0, 0xa0, 0xa0, 0xa0, 0xa0, 0xa0, 0xa0, 0x20,
        ],
    ] {
        let block = ValueInformationBlock::try_from(raw).unwrap();
        let info = ValueInformation::try_from(&block).unwrap();
        let owned = OwnedValueInformation {
            decimal_offset_exponent: info.decimal_offset_exponent,
            labels: info.labels().collect(),
            decimal_scale_exponent: info.decimal_scale_exponent,
            units: info.units().collect(),
        };
        assert_eq!(
            bincode::serialize(&info).unwrap(),
            bincode::serialize(&owned).unwrap()
        );

        let mut labels = info.labels();
        for offset in 0..=owned.labels.len() {
            assert_eq!(
                bincode::serialize(&labels).unwrap(),
                bincode::serialize(&owned.labels[offset..]).unwrap()
            );
            assert_eq!(labels.next(), owned.labels.get(offset).copied());
        }
        let mut units = info.units();
        for offset in 0..=owned.units.len() {
            assert_eq!(
                bincode::serialize(&units).unwrap(),
                bincode::serialize(&owned.units[offset..]).unwrap()
            );
            assert_eq!(units.next(), owned.units.get(offset).copied());
        }
    }
}
