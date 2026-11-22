#![cfg(feature = "std")]

use m_bus_parser::{decode_hex, DecodeOptions};

#[test]
fn zenner_preserves_sub_litre_precision() {
    let frame = "1B44496A677308500B167AD80000000F0000000080BF1B0000A64200";
    let parsed = decode_hex(frame, &DecodeOptions::default()).unwrap();
    let fields = &parsed.records[0].manufacturer_fields;
    assert_eq!(fields[0].labels, ["OK"]);
    assert_eq!(fields[1].value, "7.1035");
    assert_eq!(fields[2].value, "17.062");
    assert_eq!(fields[2].unit, "m3");
}
