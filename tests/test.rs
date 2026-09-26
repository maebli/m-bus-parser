#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/rscada_units.rs"]
mod rscada_units;

use m_bus_core::DeviceType;
use m_bus_parser::{
    user_data::{
        data_information::{
            DataFieldCoding, DataInformationError, DataType, FunctionField, Month,
            SingleEveryOrInvalid, SpecialFunctions, TextUnit,
        },
        ApplicationLayerError, Counter, DataRecord, DataRecordError, DataRecords, UserDataBlock,
    },
    Address, FrameError, Function, WiredFrame,
};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MBusData {
    slave_information: SlaveInformation,
    #[serde(rename = "DataRecord", default)]
    data_records: Vec<ExpectedRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SlaveInformation {
    id: u32,
    manufacturer: Option<String>,
    version: Option<u8>,
    // ProductName is a libmbus device database lookup, not a field in our parser.
    medium: String,
    access_number: u8,
    status: String,
    signature: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ExpectedRecord {
    #[serde(rename = "@id")]
    id: usize,
    function: String,
    storage_number: Option<u64>,
    tariff: Option<u64>,
    device: Option<u64>,
    unit: Option<String>,
    value: Option<String>,
}

fn fixtures(directory: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/rscada")
        .join(directory);
    let paths: Vec<_> = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .map(|entry| entry.expect("fixture directory must be readable"))
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "hex"))
        .map(|entry| entry.into_path())
        .collect();
    assert!(!paths.is_empty(), "empty fixture directory: {directory}");
    paths
}

fn read_frame(path: &Path) -> Vec<u8> {
    // manual_frame1 contains the single-digit token `D`; retain that byte so
    // this exercises InvalidStartByte instead of failing in a hex-string codec.
    fs::read_to_string(path)
        .unwrap()
        .split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

fn medium(name: &str) -> DeviceType {
    match name {
        "Other" => DeviceType::Other,
        "Oil" => DeviceType::OilMeter,
        "Electricity" => DeviceType::ElectricityMeter,
        "Gas" => DeviceType::GasMeter,
        "Heat: Outlet" => DeviceType::HeatMeterReturn,
        "Heat: Inlet" => DeviceType::HeatMeterFlow,
        "Warm water (30-90°C)" | "Warm water (30-90Â°C)" => DeviceType::WarmWaterMeter,
        "Water" => DeviceType::WaterMeter,
        "Cold water" => DeviceType::ColdWaterMeter,
        "Heat Cost Allocator" => DeviceType::HeatCostAllocator,
        "Heat / Cooling load meter" => DeviceType::CombinedHeatCoolingMeter,
        "Breaker: Electricity" => DeviceType::ElectricityBreaker,
        "Bus/System" => DeviceType::BusSystemComponent,
        _ => panic!("unmapped medium: {name}"),
    }
}

#[test]
fn valid_frames_match_xml_headers_and_records() {
    let paths = fixtures("test-frames");
    assert_eq!(
        paths.len(),
        73,
        "update the corpus count when adding fixtures"
    );
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        println!("fixture: {name}");
        let expected: MBusData =
            serde_xml_rs::from_str(&fs::read_to_string(path.with_extension("xml")).unwrap())
                .unwrap();
        let bytes = read_frame(&path);
        let WiredFrame::LongFrame { data, .. } = WiredFrame::try_from(bytes.as_slice()).unwrap()
        else {
            panic!("{name}: expected a long frame");
        };
        let header = &expected.slave_information;
        match UserDataBlock::try_from(data).unwrap() {
            UserDataBlock::VariableDataStructureWithLongTplHeader {
                long_tpl_header,
                variable_data_block,
                extended_link_layer,
            } => {
                assert!(extended_link_layer.is_none(), "{name}");
                assert!(!long_tpl_header.lsb_order, "{name}");
                assert_eq!(
                    u32::from(long_tpl_header.identification_number),
                    header.id,
                    "{name}: id"
                );
                assert_eq!(
                    long_tpl_header
                        .manufacturer
                        .unwrap()
                        .code
                        .iter()
                        .collect::<String>(),
                    *header.manufacturer.as_ref().unwrap(),
                    "{name}: manufacturer"
                );
                assert_eq!(
                    Some(long_tpl_header.version),
                    header.version,
                    "{name}: version"
                );
                assert_eq!(
                    long_tpl_header.device_type,
                    medium(&header.medium),
                    "{name}: medium"
                );
                assert_eq!(
                    long_tpl_header.short_tpl_header.access_number, header.access_number,
                    "{name}: access number"
                );
                assert_eq!(
                    long_tpl_header.short_tpl_header.status.bits(),
                    u8::from_str_radix(&header.status, 16).unwrap(),
                    "{name}: status"
                );
                assert_eq!(
                    long_tpl_header.short_tpl_header.configuration_field.raw(),
                    u16::from_str_radix(header.signature.as_ref().unwrap(), 16).unwrap(),
                    "{name}: signature"
                );
                let records: Vec<_> =
                    DataRecords::from((variable_data_block, &long_tpl_header)).collect();
                let legacy_plaintext =
                    matches!(name, "ELV-Elvaco-CMa10" | "THI_cma10" | "elv_temp_humid")
                        && !cfg!(feature = "plaintext-before-extension");
                assert_eq!(
                    records.len(),
                    if legacy_plaintext {
                        2
                    } else {
                        expected.data_records.len()
                    },
                    "{name}: record count"
                );
                for (index, (actual, expected)) in
                    records.iter().zip(&expected.data_records).enumerate()
                {
                    assert_eq!(expected.id, index, "{name}: XML record order");
                    // These VIFs are currently unsupported; assert the precise
                    // error and still check every subsequent readable record.
                    let unsupported_vif = matches!(
                        (name, index),
                        ("ELS_Elster-F96-Plus", 4 | 5) | ("abb_f95", 2 | 3)
                    );
                    if unsupported_vif || (legacy_plaintext && index == 1) {
                        assert_eq!(
                            actual,
                            &Err(DataRecordError::DataInformationError(
                                DataInformationError::InvalidValueInformation
                            )),
                            "{name}: record {index}"
                        );
                    } else {
                        assert_record(name, actual.as_ref().unwrap(), expected);
                    }
                }
            }
            UserDataBlock::FixedDataStructure {
                identification_number,
                access_number,
                status,
                device_type_and_unit,
                counter1,
                counter2,
            } => {
                assert_eq!(u32::from(identification_number), header.id, "{name}: id");
                assert_eq!(access_number, header.access_number, "{name}: access number");
                assert_eq!(
                    status.bits(),
                    u8::from_str_radix(&header.status, 16).unwrap(),
                    "{name}: status"
                );
                let (packed, medium, units) = match name {
                    "manual_frame2" => (0xE97E, "Water", ["l", "reserved but historic"]),
                    "sen_pollusonic_2" => (0x0569, "Heat", ["kWh", "l"]),
                    _ => panic!("{name}: unexpected fixed-data fixture"),
                };
                assert_eq!(device_type_and_unit, packed, "{name}: medium and units");
                assert_eq!(header.medium, medium);
                assert_eq!(expected.data_records.len(), 2);
                for (index, (record, counter)) in expected
                    .data_records
                    .iter()
                    .zip([counter1, counter2])
                    .enumerate()
                {
                    assert_eq!(record.id, index);
                    assert_eq!(record.function, "Actual value");
                    assert_eq!(record.unit.as_deref(), Some(units[index]));
                    let digits = format!(
                        "{:08}",
                        record.value.as_ref().unwrap().parse::<u32>().unwrap()
                    );
                    let mut bcd = hex::decode(digits).unwrap();
                    bcd.reverse();
                    assert_eq!(
                        counter,
                        Counter::from_bcd_hex_digits(bcd.try_into().unwrap()).unwrap(),
                        "{name}: counter {index}"
                    );
                }
            }
            other => panic!("{name}: unexpected application block: {other:?}"),
        }
    }
}

fn assert_record(name: &str, actual: &DataRecord<'_>, expected: &ExpectedRecord) {
    let context = format!("{name}: record {}", expected.id);
    let info = actual.data_information().expect(&context);
    let special = match expected.function.as_str() {
        "Manufacturer specific" => Some(SpecialFunctions::ManufacturerSpecific),
        "More records follow" => Some(SpecialFunctions::MoreRecordsFollow),
        _ => None,
    };
    if let Some(special) = special {
        assert_eq!(
            info.data_field_coding,
            DataFieldCoding::SpecialFunctions(special),
            "{context}"
        );
    } else {
        let function = match expected.function.as_str() {
            "Instantaneous value" => FunctionField::InstantaneousValue,
            "Maximum value" => FunctionField::MaximumValue,
            "Minimum value" => FunctionField::MinimumValue,
            "Value during error state" => FunctionField::ValueDuringErrorState,
            other => panic!("{context}: unknown function {other}"),
        };
        assert_eq!(info.function_field, function, "{context}: function");
        assert_eq!(
            info.storage_number,
            expected.storage_number.unwrap_or(0),
            "{context}: storage"
        );
        assert_eq!(
            info.tariff,
            expected.tariff.unwrap_or(0),
            "{context}: tariff"
        );
        assert_eq!(
            info.device,
            expected.device.unwrap_or(0),
            "{context}: device"
        );
    }
    if let Some(unit) = &expected.unit {
        let (scale, label, units) = rscada_units::unit(unit.trim());
        let value_info = actual.value_information().expect(&context);
        assert_eq!(value_info.decimal_scale_exponent, scale, "{context}: scale");
        assert_eq!(value_info.decimal_offset_exponent, 0, "{context}: offset");
        assert_eq!(value_info.labels().next(), label, "{context}: quantity");
        assert!(
            value_info.units().collect::<Vec<_>>().starts_with(&units),
            "{context}: units"
        );
        if matches!(
            unit.as_str(),
            "C" | "c" | "cust. ID" | "bat. time" | "1e-2  %RH"
        ) {
            let raw = actual
                .data_record_header
                .raw_data_record_header
                .value_information_block
                .as_ref()
                .unwrap();
            let text: String = raw
                .plaintext_vife
                .as_ref()
                .unwrap()
                .as_ascii_str()
                .unwrap()
                .chars()
                .rev()
                .collect();
            assert_eq!(
                text,
                if unit == "1e-2  %RH" { "%RH" } else { unit },
                "{context}: plaintext unit"
            );
        }
    } else {
        assert!(
            actual.value_information().is_none(),
            "{context}: unexpected unit"
        );
    }
    let text = expected.value.as_deref().unwrap_or("");
    match actual.value() {
        Some(DataType::Number(value)) => {
            // libmbus renders signed BCD as Fxxxxx; our API returns a number.
            let number = if let Some(digits) = text.strip_prefix('F') {
                -digits.parse::<f64>().unwrap()
            } else {
                text.parse::<f64>().unwrap()
            };
            if info.data_field_coding == DataFieldCoding::Real32Bit {
                // XML rounds floating-point values; integer and BCD values are exact.
                assert!(
                    (value - number).abs() <= 1e-6 * number.abs().max(1.0),
                    "{context}: value {value} != {text}"
                );
            } else {
                assert_eq!(*value, number, "{context}: value");
            }
        }
        Some(DataType::Text(value)) => {
            // serde-xml-rs trims whitespace-only XML values.
            if text.is_empty() {
                assert_eq!(
                    value,
                    &TextUnit::new(&[b' '; 10]),
                    "{context}: blank customer ID"
                );
            } else {
                assert!(value == text, "{context}: text value");
            }
        }
        Some(DataType::ManufacturerSpecific(value)) => {
            let bytes: Vec<_> = text
                .split_whitespace()
                .map(|b| u8::from_str_radix(b, 16).unwrap())
                .collect();
            assert_eq!(*value, bytes, "{context}: manufacturer payload");
        }
        Some(
            value @ (DataType::Date(..)
            | DataType::DateTime(..)
            | DataType::DateTimeWithSeconds(..)),
        ) => {
            assert_date(name, expected.id, value, text);
        }
        other => panic!("{context}: unexpected value {other:?}"),
    }
}

fn date_part<T: Copy + Into<u16>>(value: &SingleEveryOrInvalid<T>, every: u16) -> u16 {
    match value {
        SingleEveryOrInvalid::Single(value) => (*value).into(),
        SingleEveryOrInvalid::Every() => every,
        SingleEveryOrInvalid::Invalid() => 0,
        _ => panic!("unexpected date component"),
    }
}

fn month(value: &SingleEveryOrInvalid<Month>) -> u16 {
    match value {
        SingleEveryOrInvalid::Single(value) => *value as u16 + 1,
        SingleEveryOrInvalid::Invalid() => 0,
        other => panic!("unexpected month {other:?}"),
    }
}

fn assert_date(name: &str, index: usize, value: &DataType<'_>, expected: &str) {
    let (day, mon, year, time) = match value {
        DataType::Date(day, mon, year) => (day, mon, year, None),
        DataType::DateTime(day, mon, year, hour, minute) => (
            day,
            mon,
            year,
            Some((date_part(hour, 0), date_part(minute, 0), 0)),
        ),
        DataType::DateTimeWithSeconds(day, mon, year, hour, minute, second) => (
            day,
            mon,
            year,
            Some((
                date_part(hour, 0),
                date_part(minute, 0),
                date_part(second, 0),
            )),
        ),
        _ => unreachable!(),
    };
    // libmbus prints wildcard day/year encodings as their numeric bit patterns.
    let day = date_part(day, if time.is_some() { 31 } else { 0 });
    let mut actual = format!("{:04}-{:02}-{day:02}", date_part(year, 2127), month(mon));
    if let Some((h, m, s)) = time {
        actual.push_str(&format!("T{h:02}:{m:02}:{s:02}"));
    }
    // This XML has a stale timestamp; the capture contains 21 15 E9 17.
    let expected = if (name, index) == ("REL-Relay-Padpuls2", 1) {
        "2015-07-09T21:33:00"
    } else {
        expected
    };
    assert_eq!(actual, expected, "{name}: record {index} date");
}

#[test]
fn error_frames_have_explicit_outcomes() {
    let paths = fixtures("error-frames");
    assert_eq!(paths.len(), 20);
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        println!("fixture: {name}");
        let bytes = read_frame(&path);
        let WiredFrame::LongFrame { data, .. } = WiredFrame::try_from(bytes.as_slice()).unwrap()
        else {
            panic!("{name}: expected long frame");
        };
        let block = UserDataBlock::try_from(data);
        match name {
            "application_busy"
            | "buffer_too_long"
            | "error"
            | "premature_end_of_record"
            | "too_many_difes"
            | "too_many_readouts"
            | "too_many_records"
            | "too_many_vifes"
            | "unimplemented_ci"
            | "unspecified_error" => {
                // These are valid CI=70 error-status telegrams, not malformed
                // DIF/VIF streams. Decoding the reported status is unsupported.
                assert_eq!(
                    block,
                    Err(ApplicationLayerError::Unimplemented {
                        feature: "SendErrorStatus control information"
                    }),
                    "{name}"
                );
            }
            "too_short_header" => assert_eq!(
                block,
                Err(ApplicationLayerError::InsufficientData),
                "{name}"
            ),
            _ => {
                let block = block.unwrap();
                let mut records = block.data_records().unwrap();
                let (count, error) = match name {
                    "premature_end_of_data1" | "premature_end_of_data2" => {
                        (2, Some(DataRecordError::InsufficientData))
                    }
                    "premature_end_of_dif1" | "premature_end_of_dif2" | "premature_end_of_vif1" => {
                        (
                            2,
                            Some(DataRecordError::DataInformationError(
                                DataInformationError::DataTooShort,
                            )),
                        )
                    }
                    "premature_end_of_var_vif1" | "too_long_var_vif" => (
                        if cfg!(feature = "plaintext-before-extension") {
                            3
                        } else {
                            1
                        },
                        Some(DataRecordError::DataInformationError(
                            DataInformationError::InvalidValueInformation,
                        )),
                    ),
                    "too_many_vife" => (
                        2,
                        Some(DataRecordError::DataInformationError(
                            DataInformationError::InvalidValueInformation,
                        )),
                    ),
                    // Unlike libmbus, the borrowed DIFE representation accepts
                    // this extended chain. Pin its decoded result below.
                    "too_many_dife" => (3, None),
                    _ => panic!("{name}: missing error-fixture expectation"),
                };
                for index in 0..count {
                    let record = records.next().unwrap().unwrap();
                    assert!(!record.raw_bytes().is_empty(), "{name}: record {index}");
                    let values: &[f64] =
                        if matches!(name, "premature_end_of_var_vif1" | "too_long_var_vif") {
                            &[0.0, 4564.0, 4552.0]
                        } else {
                            &[12565.0, 113.0, 21837.0]
                        };
                    assert_eq!(
                        record.value(),
                        Some(&DataType::Number(values[index])),
                        "{name}: record {index}"
                    );
                    if name == "too_many_dife" && index == 2 {
                        let info = record.data_information().unwrap();
                        assert_eq!(info.storage_number, 1_612_617_054_070);
                        assert_eq!(info.tariff, 2_097_152);
                        assert_eq!(info.device, 1024);
                    }
                }
                if let Some(error) = error {
                    assert_eq!(records.next(), Some(Err(error)), "{name}");
                }
                assert_eq!(records.next(), None, "{name}: unexpected extra record");
                assert_eq!(records.next(), None, "{name}: iterator must stay exhausted");
            }
        }
    }
}

#[test]
fn unsupported_frames_have_explicit_outcomes() {
    let paths = fixtures("unsupported-frames");
    assert_eq!(paths.len(), 7);
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let bytes = read_frame(&path);
        let frame = WiredFrame::try_from(bytes.as_slice());
        match name {
            "invalid_length" => assert_eq!(
                frame,
                Err(FrameError::WrongChecksum {
                    expected: 8,
                    actual: 0
                }),
                "{name}"
            ),
            "manual_frame1" => assert_eq!(frame, Err(FrameError::InvalidStartByte), "{name}"),
            "manual_frame4" | "manual_frame5" | "manual_frame6" => {
                assert_eq!(
                    frame,
                    Ok(WiredFrame::ControlFrame {
                        function: Function::SndUd { fcb: false },
                        address: Address::Broadcast {
                            reply_required: true
                        },
                        data: &bytes[6..bytes.len() - 2]
                    }),
                    "{name}"
                );
            }
            "invalid_length2" | "svm_f22_telegram2" => {
                let WiredFrame::LongFrame { data, .. } = frame.unwrap() else {
                    panic!("{name}: expected long frame");
                };
                let block = UserDataBlock::try_from(data);
                if name == "invalid_length2" {
                    assert_eq!(
                        block,
                        Err(ApplicationLayerError::InsufficientData),
                        "{name}"
                    );
                } else {
                    // This continuation telegram is now supported as an opaque
                    // MoreRecordsFollow record; no payload bytes may be lost.
                    let UserDataBlock::VariableDataStructureWithLongTplHeader {
                        variable_data_block,
                        ..
                    } = block.as_ref().unwrap()
                    else {
                        panic!("{name}: expected variable data");
                    };
                    let block = block.as_ref().unwrap();
                    let mut records = block.data_records().unwrap();
                    let record = records.next().unwrap().unwrap();
                    assert_eq!(
                        record.data_information().unwrap().data_field_coding,
                        DataFieldCoding::SpecialFunctions(SpecialFunctions::MoreRecordsFollow)
                    );
                    assert_eq!(record.raw_bytes(), *variable_data_block);
                    assert_eq!(
                        record.value(),
                        Some(&DataType::ManufacturerSpecific(&variable_data_block[1..]))
                    );
                    assert_eq!(records.next(), None);
                }
            }
            _ => panic!("{name}: missing unsupported-fixture expectation"),
        }
    }
}
