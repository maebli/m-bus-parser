#![cfg(feature = "decryption-mode-10")]

use m_bus_core::{
    ConfigurationField, DeviceType, ManufacturerCode, SecurityMode,
    decryption::{DecryptionError, KeyContext, KeyProvider, Mode10Payload, StaticKeyProvider},
};

const MASTER_KEY: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const OMS_PAYLOAD: &str = "1001b30a0000c882038e223bbc1091ceb8c0a1009d388a5af7883a5b5a7fd6";
const PLAINTEXT: &str = "0c1427048502046d32371f1502fd170000";

struct FixedKey<'a>(&'a [u8]);

impl KeyProvider for FixedKey<'_> {
    fn get_key(&self, _: &KeyContext) -> Result<&[u8], DecryptionError> {
        Ok(self.0)
    }
}

fn bytes<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(hex.len(), N * 2);
    std::array::from_fn(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap())
}

fn payload(data: &[u8]) -> Mode10Payload<'_> {
    Mode10Payload {
        data,
        context: KeyContext {
            manufacturer: ManufacturerCode::from_id(0x1593).unwrap(),
            identification_number: 12345678,
            version: 0x33,
            device_type: DeviceType::from(0x03),
            security_mode: SecurityMode::AesCcm128,
            access_number: 0x75,
        },
        control_information: 0x7a,
        status: 0,
        configuration: ConfigurationField::from(0x2a11),
        from_meter: true,
    }
}

#[test]
fn oms_annex_n_2_6_decrypts_with_the_persistent_key() {
    let data = bytes::<31>(OMS_PAYLOAD);
    let mut provider = StaticKeyProvider::<1>::new();
    provider.add_key(0x1593, 12345678, MASTER_KEY).unwrap();
    let mut output = [0xa5; 32];

    assert_eq!(payload(&data).decrypt_into(&provider, &mut output), Ok(17));
    assert_eq!(output[..17], bytes::<17>(PLAINTEXT));
    assert_eq!(output[17..], [0xa5; 15]);
}

#[test]
fn modified_ciphertext_and_tags_clear_the_entire_output_payload() {
    for index in 6..31 {
        let mut data = bytes::<31>(OMS_PAYLOAD);
        data[index] ^= 1;
        let mut output = [0xa5; 32];

        assert_eq!(
            payload(&data).decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::AuthenticationFailed),
            "modified byte {index}"
        );
        assert_eq!(output[..17], [0; 17]);
        assert_eq!(output[17..], [0xa5; 15]);
    }
}

#[test]
fn the_wrong_key_does_not_release_plaintext() {
    let data = bytes::<31>(OMS_PAYLOAD);
    let mut key = MASTER_KEY;
    key[0] ^= 1;
    let mut output = [0xa5; 17];

    assert_eq!(
        payload(&data).decrypt_into(&FixedKey(&key), &mut output),
        Err(DecryptionError::AuthenticationFailed)
    );
    assert_eq!(output, [0; 17]);
}

#[test]
fn authenticated_transport_fields_cannot_be_changed() {
    let data = bytes::<31>(OMS_PAYLOAD);
    for field in 0..4 {
        let mut encrypted = payload(&data);
        match field {
            0 => encrypted.control_information ^= 1,
            1 => encrypted.context.access_number ^= 1,
            2 => encrypted.status ^= 1,
            _ => encrypted.configuration = ConfigurationField::from(0x6a11),
        }
        let mut output = [0xa5; 17];

        assert_eq!(
            encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::AuthenticationFailed),
            "modified transport field {field}"
        );
        assert_eq!(output, [0; 17]);
    }
}

#[test]
fn the_configuration_extension_and_message_counter_are_authenticated() {
    for (index, mask) in [(0, 1), (1, 4), (2, 1), (3, 1), (4, 1), (5, 1)] {
        let mut data = bytes::<31>(OMS_PAYLOAD);
        data[index] ^= mask;
        let mut output = [0xa5; 17];

        assert_eq!(
            payload(&data).decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::AuthenticationFailed),
            "modified header byte {index}"
        );
        assert_eq!(output, [0; 17]);
    }
}

#[test]
fn the_nonce_authenticates_every_meter_address_field() {
    let data = bytes::<31>(OMS_PAYLOAD);
    for field in 0..4 {
        let mut encrypted = payload(&data);
        match field {
            0 => encrypted.context.manufacturer = ManufacturerCode::from_id(0x1594).unwrap(),
            1 => encrypted.context.identification_number += 1,
            2 => encrypted.context.version ^= 1,
            _ => encrypted.context.device_type = DeviceType::from(0x07),
        }
        let mut output = [0xa5; 17];

        assert_eq!(
            encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::AuthenticationFailed),
            "modified address field {field}"
        );
        assert_eq!(output, [0; 17]);
    }
}

#[test]
fn every_truncated_oms_payload_is_rejected_without_changing_output() {
    let data = bytes::<31>(OMS_PAYLOAD);
    for length in 0..data.len() {
        let mut output = [0xa5; 32];

        assert_eq!(
            payload(&data[..length]).decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::InvalidDataLength),
            "truncated length {length}"
        );
        assert_eq!(output, [0xa5; 32]);
    }
}

#[test]
fn every_short_output_buffer_remains_unchanged() {
    let data = bytes::<31>(OMS_PAYLOAD);
    for length in 0..17 {
        let mut output = [0xa5; 17];

        assert_eq!(
            payload(&data).decrypt_into(&FixedKey(&MASTER_KEY), &mut output[..length]),
            Err(DecryptionError::InvalidDataLength),
            "output length {length}"
        );
        assert_eq!(output, [0xa5; 17]);
    }
}

#[test]
fn unsupported_configuration_extensions_are_rejected() {
    for extension in [
        0x0190_u16, 0x4110, 0x8110, 0x0100, 0x0120, 0x0130, 0x0010, 0x0210, 0x0310,
    ] {
        let mut data = bytes::<31>(OMS_PAYLOAD);
        data[..2].copy_from_slice(&extension.to_le_bytes());
        let mut output = [0xa5; 17];

        assert_eq!(
            payload(&data).decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::UnsupportedConfiguration),
            "configuration extension {extension:04x}"
        );
        assert_eq!(output, [0xa5; 17]);
    }
}

#[test]
fn missing_counter_and_inconsistent_security_modes_are_rejected() {
    let data = bytes::<31>(OMS_PAYLOAD);
    for configuration in [0x0a11, 0x2511] {
        let mut encrypted = payload(&data);
        encrypted.configuration = ConfigurationField::from(configuration);
        let mut output = [0xa5; 17];

        assert_eq!(
            encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::UnsupportedConfiguration)
        );
        assert_eq!(output, [0xa5; 17]);
    }
    let mut encrypted = payload(&data);
    encrypted.context.security_mode = SecurityMode::NoEncryption;
    assert_eq!(
        encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut [0; 17]),
        Err(DecryptionError::UnsupportedMode(SecurityMode::NoEncryption))
    );
}

#[test]
fn missing_and_invalid_keys_do_not_change_output() {
    let data = bytes::<31>(OMS_PAYLOAD);
    let mut output = [0xa5; 17];

    assert_eq!(
        payload(&data).decrypt_into(&StaticKeyProvider::<0>::new(), &mut output),
        Err(DecryptionError::KeyNotFound)
    );
    assert_eq!(output, [0xa5; 17]);
    for key in [&[][..], &[0; 15][..], &[0; 17][..], &[0; 32][..]] {
        assert_eq!(
            payload(&data).decrypt_into(&FixedKey(key), &mut output),
            Err(DecryptionError::InvalidKeyLength)
        );
        assert_eq!(output, [0xa5; 17]);
    }
}

#[test]
fn python_cryptography_vectors_cover_key_version_partial_and_full_encryption() {
    for (configuration, encoded) in [
        (
            0x2a11,
            "500103b30a0000c882038e223bbc1091ceb8c0a1009d388afbba39f83ae8360e",
        ),
        (
            0x2a06,
            "1001b30a0000c882038e223b046d32371f1502fd170000117e3aa10f5ca88f",
        ),
        (
            0x2a00,
            "1001b30a00000c1427048502046d32371f1502fd170000d31f970bc5d79e93",
        ),
        (
            0x2aff,
            "1001b30a0000c882038e223bbc1091ceb8c0a1009d388a79263ba256b9b57c",
        ),
    ] {
        let data: Vec<u8> = (0..encoded.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&encoded[index..index + 2], 16).unwrap())
            .collect();
        let mut encrypted = payload(&data);
        encrypted.configuration = ConfigurationField::from(configuration);
        let mut output = [0xa5; 17];

        assert_eq!(
            encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Ok(17),
            "configuration {configuration:04x}"
        );
        assert_eq!(output, bytes::<17>(PLAINTEXT));
    }
}

#[test]
fn changed_key_versions_and_cleartext_suffixes_fail_authentication() {
    let mut version_data =
        bytes::<32>("500103b30a0000c882038e223bbc1091ceb8c0a1009d388afbba39f83ae8360e");
    version_data[2] ^= 1;
    let mut partial_data =
        bytes::<31>("1001b30a0000c882038e223b046d32371f1502fd170000117e3aa10f5ca88f");
    partial_data[12] ^= 1;
    for (data, configuration) in [(&version_data[..], 0x2a11), (&partial_data[..], 0x2a06)] {
        let mut encrypted = payload(data);
        encrypted.configuration = ConfigurationField::from(configuration);
        let mut output = [0xa5; 17];

        assert_eq!(
            encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
            Err(DecryptionError::AuthenticationFailed)
        );
        assert_eq!(output, [0; 17]);
    }
}

#[test]
fn gateway_messages_require_the_gateway_derivation_constant() {
    let data = bytes::<31>("1001b30a0000127bd096157e100814166ce03ed827ea07d1014fe479f20aba");
    let mut encrypted = payload(&data);
    encrypted.control_information = 0x53;
    encrypted.from_meter = false;
    let mut output = [0xa5; 17];

    assert_eq!(
        encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
        Ok(17)
    );
    assert_eq!(output, bytes::<17>(PLAINTEXT));
    encrypted.from_meter = true;
    assert_eq!(
        encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
        Err(DecryptionError::AuthenticationFailed)
    );
    assert_eq!(output, [0; 17]);
}

#[test]
fn unsupported_lengths_and_identifiers_do_not_change_output() {
    let mut data = vec![0; 2 + 4 + 256 + 8];
    data[..2].copy_from_slice(&[0x10, 0x01]);
    let mut encrypted = payload(&data);
    encrypted.configuration = ConfigurationField::from(0x2a00);
    let mut output = [0xa5; 256];

    assert_eq!(
        encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
        Err(DecryptionError::InvalidDataLength)
    );
    assert_eq!(output, [0xa5; 256]);

    let data = bytes::<31>(OMS_PAYLOAD);
    let mut encrypted = payload(&data);
    encrypted.context.identification_number = 100_000_000;
    assert_eq!(
        encrypted.decrypt_into(&FixedKey(&MASTER_KEY), &mut output),
        Err(DecryptionError::InvalidDataLength)
    );
    assert_eq!(output, [0xa5; 256]);
}
