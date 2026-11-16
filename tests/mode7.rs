#![cfg(all(feature = "std", feature = "decryption-mode-7"))]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{
    decode_bytes, decode_hex, decode_hex_bytes, render_hex, DecodeOptions, OutputFormat,
    RenderOptions,
};

const MASTER_KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
const OMS_WIRED: &str = "683131680803900F002C25B30A0000A08518CCB022C5FD7278563412931533037500100710D371C801D409B0D928D5659759C2EC935B16";
const OMS_WIRELESS: &str = "434493157856341233038C2075900F002C25B30A000021924D4F2FB66E017A75002007109058475F4BC91DF878B80A1B0F98B629024AAC727942BFC549233C0140829B93";
const OMS_WIRELESS_CRC: &str = "434493157856341233037AC98C2075900F002C25B30A000021924D4FBA372FB66E017A75002007109058475F4BC9D1281DF878B80A1B0F98B629024AAC7279429398BFC549233C0140829B93BAA1";

fn options() -> DecodeOptions {
    DecodeOptions {
        key: Some(MASTER_KEY),
        ..DecodeOptions::default()
    }
}

#[test]
fn oms_annex_n_2_3_and_n_2_5_authenticate_and_decode() {
    for (frame, protocol, count) in [
        (OMS_WIRED, "wired", 2),
        (OMS_WIRELESS, "wireless", 3),
        (OMS_WIRELESS_CRC, "wireless", 3),
    ] {
        let output = decode_hex(frame, &options()).unwrap();
        assert_eq!(output.protocol, protocol);
        assert_eq!(output.decode_state, "complete");
        assert_eq!(output.security.mode_code, Some(7));
        assert_eq!(output.security.key_id, Some(0));
        assert_eq!(output.security.message_counter, Some(2739));
        assert_eq!(output.security.decryption_state, "decrypted_authenticated");
        assert_eq!(output.records.len(), count);
        assert_eq!(
            serde_json::to_value(&output.records[0]).unwrap()["value"]["value"],
            "28504.27"
        );
    }
}

#[test]
fn headers_without_a_key_do_not_expose_readings() {
    for frame in [OMS_WIRED, OMS_WIRELESS] {
        let output = decode_hex(frame, &DecodeOptions::default()).unwrap();
        assert_eq!(output.security.mode_code, Some(7));
        assert_eq!(output.security.key_id, Some(0));
        assert_eq!(output.security.message_counter, Some(2739));
        assert_eq!(output.security.decryption_state, "key_missing");
        assert!(output.records.is_empty());
        assert!(output.security.decrypted_payload_hex.is_none());
    }
}

#[test]
fn wrong_keys_and_modified_authentication_data_are_rejected() {
    let wrong_key = DecodeOptions {
        key: Some([0; 16]),
        ..DecodeOptions::default()
    };
    for frame in [OMS_WIRED, OMS_WIRELESS] {
        assert_eq!(
            decode_hex(frame, &wrong_key).unwrap_err().code(),
            "security.authentication_failed"
        );
    }
    let original = decode_hex_bytes(OMS_WIRELESS).unwrap();
    for index in 18..original.len() {
        let mut frame = original.clone();
        frame[index] ^= 1;
        let result = decode_bytes(&frame, &options());
        if let Ok(output) = &result {
            assert!(output.records.is_empty(), "modified byte {index}");
            assert!(
                output.security.decrypted_payload_hex.is_none(),
                "modified byte {index}"
            );
        }
    }
}

#[test]
fn the_old_header_only_example_cannot_bypass_authentication() {
    let frame = "6820206808017278563412931533032A0010071066E94BD4EF8A2C3B884CFA59CA342B2E8A16";
    assert_eq!(
        decode_hex(frame, &options()).unwrap_err().code(),
        "security.unsupported_configuration"
    );
}

#[test]
fn authenticated_mode7_supports_every_output_format() {
    for frame in [OMS_WIRED, OMS_WIRELESS] {
        for format in [
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Csv,
            OutputFormat::Xml,
            OutputFormat::Table,
            OutputFormat::Mermaid,
            OutputFormat::Annotated,
        ] {
            let output = render_hex(
                frame,
                format,
                &RenderOptions {
                    decode: options(),
                    ..RenderOptions::default()
                },
            )
            .unwrap();
            assert!(!output.is_empty());
        }
    }
}

fn context() -> m_bus_core::decryption::KeyContext {
    m_bus_core::decryption::KeyContext {
        manufacturer: m_bus_core::ManufacturerCode::from_id(0x1593).unwrap(),
        identification_number: 12345678,
        version: 0x33,
        device_type: m_bus_core::DeviceType::GasMeter,
        security_mode: m_bus_core::SecurityMode::AesCbc128IvZero,
        access_number: 0x75,
    }
}

fn provider() -> m_bus_core::decryption::StaticKeyProvider<1> {
    let mut provider = m_bus_core::decryption::StaticKeyProvider::new();
    provider.add_key(0x1593, 12345678, MASTER_KEY).unwrap();
    provider
}

#[test]
fn openssl_vectors_cover_encryption_boundaries_and_optional_fields() {
    use m_bus_core::{
        authentication::AuthenticationHeader, decryption::Mode7Payload, ConfigurationField,
    };
    let vectors = [
("partial", 0x0710, true, "900f002c25b30a0000596d330a224ce0287a7500100710d371c801d409b0d928d5659759c2ec9302fd170000", "0c1427048502046d32371f152f2f02fd170000"),
("all_blocks", 0x07f0, true, "900f002c25b30a0000ae26fa7cf542d8577a7500f00710d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("padding", 0x0718, true, "900f002c25b30a0000a982bbba6ad5781f7a750018071069d2bc156bfe9b95fea6bf34efcdaef6", "0c1427048502046d32371f15"),
("padding_partial", 0x0718, true, "900f002c25b30a0000a134eebcf4c49ea67a750018071069d2bc156bfe9b95fea6bf34efcdaef602fd170000", "0c1427048502046d32371f1502fd170000"),
("key_version", 0x0710, true, "900f002c25b30a0000eb241a3f08322bb17a750010075003d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("key_information", 0x0710, true, "9011002e351003b30a00004fab6318bb3b1ac67a750010075003d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("tag12", 0x0710, true, "9013002c26b30a0000ba8e7a423f335f70faaf18e57a7500100710d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("tag16", 0x0710, true, "9017002c27b30a0000ccbe54e447fc689cc07e45163dadb3bb7a7500100710d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("length", 0x0710, true, "9011003c65b30a0000b9e605751565293816007a7500100710d371c801d409b0d928d5659759c2ec93", "0c1427048502046d32371f152f2f"),
("gateway", 0x0710, false, "900f002c25b30a00005124df0f97bb40435a7500100710d5028b23893ad87f3c2d2da471360d2f", "0c1427048502046d32371f152f2f"),
("authenticated_cleartext", 0x0700, true, "900f002c25b30a0000ad085ee9e72d22667a75000007100c1427048502046d32371f15", "0c1427048502046d32371f15"),
("bad_padding", 0x0718, true, "900f002c25b30a00004b98f541eb6b55747a750018071035d2a9583a2d95736925dc6a861aaee9", "0c1427048502046d32371f15"),
    ];
    for (name, configuration, from_meter, frame, expected) in vectors {
        let bytes = decode_hex_bytes(frame).unwrap();
        let authentication = AuthenticationHeader::parse(&bytes).unwrap();
        let payload = Mode7Payload {
            data: &authentication.transport[5..],
            context: context(),
            configuration: ConfigurationField::from_bytes(
                configuration as u8,
                (configuration >> 8) as u8,
            ),
            from_meter,
            authentication: &authentication,
        };
        let mut output = [0xaa; 64];
        let result = payload.decrypt_into(&provider(), &mut output);
        if name == "bad_padding" {
            assert_eq!(
                result,
                Err(m_bus_core::decryption::DecryptionError::DecryptionFailed)
            );
            assert!(output[..16].iter().all(|byte| *byte == 0));
        } else {
            assert!(result.is_ok(), "{name}: {result:?}");
            let length = result.unwrap();
            assert_eq!(
                &output[..length],
                decode_hex_bytes(expected).unwrap(),
                "{name}"
            );
        }
    }
}

#[test]
fn incomplete_and_inconsistent_authentication_headers_are_rejected() {
    use m_bus_core::authentication::AuthenticationHeader;
    let frame = decode_hex_bytes(OMS_WIRELESS).unwrap();
    let afl = &frame[13..];
    for length in 0..=17 {
        assert!(
            AuthenticationHeader::parse(&afl[..length]).is_err(),
            "length {length}"
        );
    }
    for (offset, value) in [
        (1, 14),
        (1, 16),
        (1, 255),
        (2, 1),
        (3, 0x6c),
        (3, 0x2d),
        (3, 0x28),
        (3, 0x0c),
        (4, 0xa5),
        (4, 0x35),
        (4, 0x65),
        (4, 0x24),
    ] {
        let mut modified = afl.to_vec();
        modified[offset] = value;
        assert!(
            AuthenticationHeader::parse(&modified).is_err(),
            "byte {offset} = {value}"
        );
    }
}

#[test]
fn authentication_failure_clears_output_and_never_decrypts() {
    use m_bus_core::{
        authentication::AuthenticationHeader,
        decryption::{DecryptionError, Mode7Payload, StaticKeyProvider},
        ConfigurationField,
    };
    let mut bytes = decode_hex_bytes(OMS_WIRELESS).unwrap();
    bytes[22] ^= 1;
    let authentication = AuthenticationHeader::parse(&bytes[13..]).unwrap();
    let payload = Mode7Payload {
        data: &authentication.transport[5..],
        context: context(),
        configuration: ConfigurationField::from_bytes(0x20, 7),
        from_meter: true,
        authentication: &authentication,
    };
    let mut output = [0xaa; 32];
    assert_eq!(
        payload.decrypt_into(&provider(), &mut output),
        Err(DecryptionError::AuthenticationFailed)
    );
    assert_eq!(output, [0; 32]);
    assert_eq!(
        payload.decrypt_into(&provider(), &mut output[..31]),
        Err(DecryptionError::InvalidDataLength)
    );
    assert_eq!(
        payload.decrypt_into(&StaticKeyProvider::<1>::new(), &mut output),
        Err(DecryptionError::KeyNotFound)
    );
}

#[test]
fn annotated_examples_show_authenticated_readings() {
    for frame in [OMS_WIRED, OMS_WIRELESS, OMS_WIRELESS_CRC] {
        let rendered = render_hex(
            frame,
            OutputFormat::Annotated,
            &RenderOptions {
                decode: options(),
                ..RenderOptions::default()
            },
        )
        .unwrap();
        let output: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(output["decrypted"], true);
        let segments = output["segments"].as_array().unwrap();
        assert!(segments
            .iter()
            .any(|segment| segment["kind"] == "authentication_fragmentation_layer"));
        assert!(segments.iter().any(|segment| segment["kind"] == "dif"));
        assert!(!segments
            .iter()
            .any(|segment| segment["kind"] == "encrypted_payload"));
    }
}
