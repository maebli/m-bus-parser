#![cfg(feature = "std")]

use m_bus_parser::{decode_hex, DecodeOptions};

#[test]
fn hydroclima_both_layouts_report_the_published_readings() {
    for (version, tail, average, total) in [
        (0x53, "100043106A7D2C4A078F12202CB1242A06D3062100210000","18.66","33"),
        (0x85, "1100009924293400003713DF08BC163633960E8208D401D90879076006DA080809DA08E537000A3500DF1201830900","22.71","2435"),
    ] {
        let body = format!("44B00912345678{version:02X}087A000000000F{tail}");
        let telegram = format!("{:02X}{body}", body.len()/2);
        let output = decode_hex(&telegram,&DecodeOptions::default()).unwrap();
        let fields = &output.records[0].manufacturer_fields;
        for (name,value) in [("average ambient temperature", average), ("total consumption", total)] {
            assert!(fields.iter().any(|field| field.name == name && field.value == value));
        }
    }
}

#[test]
fn hydrodigit_decodes_battery_backflow_and_monthly_volume() {
    let telegram = "4644B4092143658713077A9C0000000C1364390400046D212F16350F152A0F000000440F00C00F00511000D51000B20B00180C007C0C00E60C00560D00D10D00400E00C60E0000";
    let parsed = decode_hex(telegram, &DecodeOptions::default()).unwrap();
    let fields = &parsed.records.last().unwrap().manufacturer_fields;
    for (name, value) in [
        ("battery voltage", "3.05"),
        ("reverse volume", "0.015"),
        ("January total", "39.08"),
        ("December total", "37.82"),
    ] {
        assert!(fields.iter().any(|f| f.name == name && f.value == value));
    }
}

#[test]
fn hydrodigit_handles_event_dates_and_incomplete_history() {
    let prefix = "44B4092243658713077A7F0000000F";
    for (tail, expected_date) in [
        ("80240425", Some("2024-04-25")),
        ("80000000", None),
        ("9024042500", None),
    ] {
        let body = format!("{prefix}{tail}");
        let frame = format!("{:02X}{body}", body.len() / 2);
        let parsed = decode_hex(&frame, &DecodeOptions::default()).unwrap();
        let record = &parsed.records[0];
        let date = record
            .manufacturer_fields
            .iter()
            .find(|f| f.name == "leak date");
        assert_eq!(date.map(|f| f.value.as_str()), expected_date);
        assert_eq!(record.data_hex.replace(' ', ""), tail);
    }
}
