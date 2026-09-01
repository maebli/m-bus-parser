#![cfg(feature = "std")]
#![allow(clippy::unwrap_used)]

use m_bus_parser::{serialize_mbus_data, user_data::value_information::ValueInformationBlock};
use serde_json::{json, Value};

#[cfg(not(feature = "plaintext-before-extension"))]
const PLAINTEXT_VIF: &[u8] = &[0xFC, 0x74, 0x02, b'A', b'B'];
#[cfg(feature = "plaintext-before-extension")]
const PLAINTEXT_VIF: &[u8] = &[0xFC, 0x02, b'A', b'B', 0x74];

#[cfg(not(feature = "plaintext-before-extension"))]
const LEGACY_FRAME: &str = concat!(
    "68 4D 4D 68 08 01 72 01 00 00 00 96 15 01 00 18 00 00 00 ",
    "0C 78 56 00 00 00 01 FD 1B 00 02 FC 74 03 48 52 25 44 0D ",
    "22 FC 74 03 48 52 25 F1 0C 12 FC 74 03 48 52 25 63 11 02 ",
    "65 B4 09 22 65 86 09 12 65 B7 09 01 72 00 72 65 00 00 B2 ",
    "01 65 00 00 1F B3 16"
);
#[cfg(feature = "plaintext-before-extension")]
const LEGACY_FRAME: &str = concat!(
    "68 4D 4D 68 08 01 72 01 00 00 00 96 15 01 00 18 00 00 00 ",
    "0C 78 56 00 00 00 01 FD 1B 00 02 FC 03 48 52 25 74 44 0D ",
    "22 FC 03 48 52 25 74 F1 0C 12 FC 03 48 52 25 74 63 11 02 ",
    "65 B4 09 22 65 86 09 12 65 B7 09 01 72 00 72 65 00 00 B2 ",
    "01 65 00 00 1F B3 16"
);

#[test]
fn borrowed_vif_fields_preserve_their_logical_serde_shape() {
    let extension_bytes = [0xFD, 0xD9, 0xFC, 0x01];
    let extensions = ValueInformationBlock::try_from(extension_bytes.as_slice()).unwrap();

    assert_eq!(
        serde_json::to_value(extensions).unwrap(),
        json!({
            "value_information": { "data": 253 },
            "value_information_extension": [
                { "data": 217 },
                { "data": 252 },
                { "data": 1 }
            ],
            "plaintext_vife": null
        })
    );

    let plaintext = ValueInformationBlock::try_from(PLAINTEXT_VIF).unwrap();

    assert_eq!(
        serde_json::to_value(plaintext).unwrap(),
        json!({
            "value_information": { "data": 252 },
            "value_information_extension": [{ "data": 116 }],
            "plaintext_vife": ["A", "B"]
        })
    );
}

#[test]
fn legacy_json_and_yaml_preserve_borrowed_vif_field_shapes() {
    let json_output: Value =
        serde_json::from_str(&serialize_mbus_data(LEGACY_FRAME, "json-legacy", None)).unwrap();
    let yaml_output: serde_yaml::Value =
        serde_yaml::from_str(&serialize_mbus_data(LEGACY_FRAME, "yaml-legacy", None)).unwrap();

    let expected_extensions = json!([{ "data": 116 }]);
    let encoded_extensions = json!([116]);
    let expected_plaintext = json!(["H", "R", "%"]);
    let encoded_plaintext = json!([3, 72, 82, 37]);

    assert!(json_contains_property(
        &json_output,
        "value_information_extension",
        &expected_extensions
    ));
    assert!(!json_contains_property(
        &json_output,
        "value_information_extension",
        &encoded_extensions
    ));
    assert!(json_contains_property(
        &json_output,
        "plaintext_vife",
        &expected_plaintext
    ));
    assert!(!json_contains_property(
        &json_output,
        "plaintext_vife",
        &encoded_plaintext
    ));

    let expected_extensions = serde_yaml::from_str("[{data: 116}]").unwrap();
    let encoded_extensions = serde_yaml::from_str("[116]").unwrap();
    let expected_plaintext = serde_yaml::from_str("[H, R, '%']").unwrap();
    let encoded_plaintext = serde_yaml::from_str("[3, 72, 82, 37]").unwrap();

    assert!(yaml_contains_property(
        &yaml_output,
        "value_information_extension",
        &expected_extensions
    ));
    assert!(!yaml_contains_property(
        &yaml_output,
        "value_information_extension",
        &encoded_extensions
    ));
    assert!(yaml_contains_property(
        &yaml_output,
        "plaintext_vife",
        &expected_plaintext
    ));
    assert!(!yaml_contains_property(
        &yaml_output,
        "plaintext_vife",
        &encoded_plaintext
    ));
}

fn json_contains_property(value: &Value, key: &str, expected: &Value) -> bool {
    match value {
        Value::Object(properties) => {
            properties.get(key) == Some(expected)
                || properties
                    .values()
                    .any(|value| json_contains_property(value, key, expected))
        }
        Value::Array(values) => values
            .iter()
            .any(|value| json_contains_property(value, key, expected)),
        _ => false,
    }
}

fn yaml_contains_property(
    value: &serde_yaml::Value,
    key: &str,
    expected: &serde_yaml::Value,
) -> bool {
    match value {
        serde_yaml::Value::Mapping(properties) => {
            properties.get(key) == Some(expected)
                || properties
                    .values()
                    .any(|value| yaml_contains_property(value, key, expected))
        }
        serde_yaml::Value::Sequence(values) => values
            .iter()
            .any(|value| yaml_contains_property(value, key, expected)),
        serde_yaml::Value::Tagged(tagged) => yaml_contains_property(&tagged.value, key, expected),
        _ => false,
    }
}
