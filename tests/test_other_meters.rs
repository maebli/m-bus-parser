#![cfg(all(feature = "std", feature = "serde"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use m_bus_parser::mbus_data::MbusData;
use m_bus_parser::WiredFrame;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_electricity_meter() {
        let input = "68 65 65 68 08 00 72 78 56 34 12 74 52 C7 02 2A 00 00 00 04 05 73 00 00 00 04 FB 82 75 00 00 00 00 04 2A 8F 00 00 00 04 FB 97 72 AF FF FF FF 04 FB B7 72 A5 00 00 00 02 FD BA 73 64 03 84 80 80 40 FD 48 00 00 00 00 04 FD 48 F7 02 00 00 84 40 FD 59 77 00 00 00 84 80 40 FD 59 00 00 00 00 84 C0 40 FD 59 00 00 00 00 1F F9 16";
        let bytes: Vec<u8> = input
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();

        let mbus_data = MbusData::<WiredFrame>::try_from(bytes.as_slice()).unwrap();
        let json = serde_json::to_value(&mbus_data).unwrap();

        let header =
            &json["user_data"]["VariableDataStructureWithLongTplHeader"]["long_tpl_header"];
        assert_eq!(header["device_type"], "ElectricityMeter");
        assert_eq!(header["identification_number"]["number"], 12345678);
        assert_eq!(header["version"], 199);
        assert_eq!(header["manufacturer"]["Ok"]["code"][0], "T");
        assert_eq!(header["manufacturer"]["Ok"]["code"][1], "S");
        assert_eq!(header["manufacturer"]["Ok"]["code"][2], "T");
        assert_eq!(header["short_tpl_header"]["access_number"], 42);

        let records = json["data_records"].as_array().unwrap();
        assert_eq!(records.len(), 12);

        // (label, units, scale_exp, value, storage, tariff, device)
        let expected: &[(&str, &[(&str, i64)], i64, f64, u64, u64, u64)] = &[
            ("Energy", &[("Watt", 1), ("Hour", 1)], 2, 115.0, 0, 0, 0),
            (
                "ReactiveEnergy",
                &[("ReactiveWatt", 1), ("Hour", 1)],
                2,
                0.0,
                0,
                0,
                0,
            ),
            ("Power", &[("Watt", 1)], -1, 143.0, 0, 0, 0),
            ("ReactivePower", &[("ReactiveWatt", 1)], -1, -81.0, 0, 0, 0),
            ("ApparentPower", &[("ApparentWatt", 1)], -1, 165.0, 0, 0, 0),
            ("Dimensionless", &[], -3, 868.0, 0, 0, 0),
            ("Voltage", &[("Volt", 1)], -1, 0.0, 0, 0, 4),
            ("Voltage", &[("Volt", 1)], -1, 759.0, 0, 0, 0),
            ("Current", &[("Ampere", 1)], -3, 119.0, 0, 0, 1),
            ("Current", &[("Ampere", 1)], -3, 0.0, 0, 0, 2),
            ("Current", &[("Ampere", 1)], -3, 0.0, 0, 0, 3),
        ];

        for (i, (label, units, exponent, value, storage, tariff, device)) in
            expected.iter().enumerate()
        {
            let rec = &records[i];
            let hdr = &rec["data_record_header"]["processed_data_record_header"];
            let vi = &hdr["value_information"];
            let di = &hdr["data_information"];

            assert_eq!(
                vi["labels"][0].as_str().unwrap(),
                *label,
                "record {i} label"
            );
            assert_eq!(
                vi["decimal_scale_exponent"].as_i64().unwrap(),
                *exponent,
                "record {i} exponent"
            );
            assert_eq!(
                rec["data"]["value"]["Number"].as_f64().unwrap(),
                *value,
                "record {i} value"
            );
            assert_eq!(
                di["function_field"], "InstantaneousValue",
                "record {i} function"
            );
            assert_eq!(
                di["storage_number"].as_u64().unwrap(),
                *storage,
                "record {i} storage"
            );
            assert_eq!(di["tariff"].as_u64().unwrap(), *tariff, "record {i} tariff");
            assert_eq!(di["device"].as_u64().unwrap(), *device, "record {i} device");

            let json_units = vi["units"].as_array().unwrap();
            assert_eq!(json_units.len(), units.len(), "record {i} unit count");
            for (j, (name, exp)) in units.iter().enumerate() {
                assert_eq!(
                    json_units[j]["name"].as_str().unwrap(),
                    *name,
                    "record {i} unit {j} name"
                );
                assert_eq!(
                    json_units[j]["exponent"].as_i64().unwrap(),
                    *exp,
                    "record {i} unit {j} exponent"
                );
            }
        }

        // Record 11: 0x1F = MoreRecordsFollow (manufacturer specific, more records follow)
        let rec = &records[11];
        let di = &rec["data_record_header"]["processed_data_record_header"]["data_information"];
        assert_eq!(
            di["data_field_coding"]["SpecialFunctions"]
                .as_str()
                .unwrap(),
            "MoreRecordsFollow"
        );
        assert!(
            rec["data_record_header"]["processed_data_record_header"]["value_information"]
                .is_null()
        );
        assert_eq!(
            rec["data_record_header"]["raw_data_record_header"]["data_information_block"]
                ["data_information_field"]["data"],
            0x1F
        );
        assert!(rec["data"]["value"]["ManufacturerSpecific"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}
