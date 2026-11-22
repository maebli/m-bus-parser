#![cfg(feature = "std")]

use m_bus_parser::{decode_hex, DecodeOptions};

#[test]
fn multical_e8_and_e9_use_unsigned_registers_and_keep_storage() {
    for (manufacturer, version, matched) in [
        ("2D2C", "35", true),
        ("E61E", "35", false),
        ("2D2C", "FF", false),
    ] {
        let body =
            format!("44{manufacturer}12345678{version}047A0100000004FF072B01000044FF08FFFFFFFF");
        let frame = format!("{:02X}{body}", body.len() / 2);
        let output = decode_hex(&frame, &DecodeOptions::default()).unwrap();
        let first = &output.records[0];
        assert_eq!(
            first
                .quantities
                .iter()
                .any(|quantity| quantity.contains("E8 register count")),
            matched
        );
        if matched {
            assert_eq!(
                first.quantities,
                ["Flow temperature integral (E8 register count)"]
            );
            assert_eq!(first.value.value.as_ref().unwrap(), "299");
            let stored = &output.records[1];
            assert_eq!(
                stored.quantities,
                ["Return temperature integral (E9 register count)"]
            );
            assert_eq!(stored.storage_number, 1);
            assert_eq!(stored.value.value.as_ref().unwrap(), "4294967295");
        }
    }
}
