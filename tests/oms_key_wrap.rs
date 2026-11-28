#![cfg(all(feature = "std", feature = "oms-gen5"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::output::{render_application_oms_with_evidence, OmsFormat};
use serde_json::Value;

const TRANSFER: &str =
    "C6010000002600000000011201FC62E3909B37AC2166DAB372D3C7E9386B3AE15EEF228946F5E5E7CAFDA4D15D";
const ACTIVATE: &str = "C6010000001E0000040003120178AF64C661837162C83ED7C408D9F72200D4A6810F04F75C";
const KEY: [u8; 16] = [
    0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255,
];
fn verify(input: &str, key: Option<&[u8; 16]>) -> Value {
    let output = render_application_oms_with_evidence(
        input,
        OmsFormat::Json,
        &Default::default(),
        Default::default(),
        None,
        None,
        |frame| {
            if let Some(key) = key {
                frame.verify_key_wrap(key)?;
                let normalized = serde_json::to_string(&frame).unwrap();
                assert!(!normalized.contains("000102030405060708090A0B0C0D0E0F"));
            }
            Ok(())
        },
    )
    .unwrap();
    serde_json::from_str(&output).unwrap()
}
fn status<'a>(result: &'a Value, rule: &str) -> &'a str {
    result["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == rule)
        .unwrap()["status"]
        .as_str()
        .unwrap()
}
#[test]
fn openssl_wrapped_keys_and_counter_use_independent_evidence() {
    for input in [TRANSFER, ACTIVATE] {
        let verified = verify(input, Some(&KEY));
        for rule in [
            "AES-KWP container length",
            "SITP AES-KWP integrity",
            "SITP unwrapped structure",
            "SITP key update parameters",
            "SITP wrapper key scope",
        ] {
            assert_eq!(status(&verified, rule), "passed", "{rule}");
        }
        assert_eq!(
            status(&verify(input, None), "SITP AES-KWP integrity"),
            "could_not_check"
        );
        assert_eq!(
            status(&verify(input, Some(&[0; 16])), "SITP AES-KWP integrity"),
            "failed"
        );
    }
    let application =
        "C6010000002200002000301001010000001E83DC01C53040F2C9BC18523C9CBA403E4DC19A61900202";
    assert_eq!(
        status(&verify(application, Some(&KEY)), "SITP wrapped key counter"),
        "passed"
    );
    assert_eq!(
        status(
            &verify(
                &application.replace("100101000000", "100102000000"),
                Some(&KEY)
            ),
            "SITP wrapped key counter"
        ),
        "failed"
    );
}

#[test]
fn tls_clear_key_containers_check_mli_padding_and_key_fields() {
    let clear = "C6010000001E0000040003FFFFA65959A60000000A00000000300001000001000000000000";
    let verified = verify(clear, None);
    assert_eq!(
        status(&verified, "SITP clear key container fields"),
        "passed"
    );
    assert_eq!(status(&verified, "SITP key update parameters"), "passed");
    assert_eq!(
        status(
            &verify(&clear.replace("0000000A", "00000009"), None),
            "SITP clear key container fields"
        ),
        "failed"
    );
    assert_eq!(
        status(
            &verify(&clear.replace("300001000001", "300001000000"), None),
            "SITP key update parameters"
        ),
        "failed"
    );
}
