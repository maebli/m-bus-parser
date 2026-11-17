#![cfg(feature = "decryption-mode-13")]

use m_bus_core::{ConfigurationField, decryption::mode13::*};

mod fixtures {
    include!("fixtures/mode13.rs");
}

fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn payload(record: &str) -> Vec<u8> {
    let mut data = vec![1];
    data.extend(bytes(record));
    data
}

fn parse(data: &[u8]) -> Result<Mode13Payload<'_>, Mode13Error> {
    Mode13Payload::parse(ConfigurationField::from(0x0dff), data)
}

fn session(key: &[u8]) -> TlsSession<'_> {
    TlsSession {
        cipher_suite: if key.len() == 32 {
            TlsCipherSuite::EcdheEcdsaAes256GcmSha384
        } else {
            TlsCipherSuite::EcdheEcdsaAes128GcmSha256
        },
        direction: TlsDirection::ClientToServer,
        key,
        fixed_iv: [1, 2, 3, 4],
        sequence_number: 5,
    }
}

#[test]
fn independent_openssl_records_authenticate_for_both_key_sizes() {
    for (size, vector) in [
        (16, fixtures::OPENSSL_AES128_RECORD),
        (32, fixtures::OPENSSL_AES256_RECORD),
    ] {
        let key: Vec<u8> = (0..size).collect();
        let data = payload(vector);
        let mut output = [0xaa; 4];
        let session = session(&key);
        assert_eq!(
            parse(&data).unwrap().decrypt_into(&session, &mut output),
            Ok(3)
        );
        assert_eq!(output, [1, 0x13, 0x2a, 0xaa]);
        assert_eq!(session.sequence_number, 5);
    }
}

#[test]
fn concatenated_records_advance_the_local_counter_and_fail_atomically() {
    let key: Vec<u8> = (0..16).collect();
    let mut data = payload(fixtures::OPENSSL_AES128_RECORD);
    data.extend(bytes(fixtures::OPENSSL_AES128_SECOND_RECORD));
    let mut output = [0xaa; 7];
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.records().count(), 2);
    assert_eq!(parsed.decrypt_into(&session(&key), &mut output), Ok(6));
    assert_eq!(output, [1, 0x13, 0x2a, 1, 0x13, 0x2b, 0xaa]);
    *data.last_mut().unwrap() ^= 1;
    assert_eq!(
        parse(&data)
            .unwrap()
            .decrypt_into(&session(&key), &mut output),
        Err(Mode13Error::AuthenticationFailed)
    );
    assert_eq!(output, [0, 0, 0, 0, 0, 0, 0xaa]);
}

#[test]
fn incorrect_key_salt_counter_and_modified_ciphertext_release_no_plaintext() {
    let mut key: Vec<u8> = (0..16).collect();
    let data = payload(fixtures::OPENSSL_AES128_RECORD);
    for change in 0..3 {
        let mut session = session(&key);
        match change {
            0 => session.sequence_number += 1,
            1 => session.fixed_iv[0] ^= 1,
            _ => session.key = &[0; 16],
        }
        let mut output = [0xaa; 3];
        assert_eq!(
            parse(&data).unwrap().decrypt_into(&session, &mut output),
            Err(Mode13Error::AuthenticationFailed)
        );
        assert_eq!(output, [0; 3]);
    }
    for index in 6..data.len() {
        let mut altered = data.clone();
        altered[index] ^= 1;
        let mut output = [0xaa; 3];
        assert_eq!(
            parse(&altered)
                .unwrap()
                .decrypt_into(&session(&key), &mut output),
            Err(Mode13Error::AuthenticationFailed)
        );
        assert_eq!(output, [0; 3]);
    }
    key.pop();
    assert_eq!(
        parse(&data)
            .unwrap()
            .decrypt_into(&session(&key), &mut [0; 3]),
        Err(Mode13Error::InvalidKeyLength)
    );
}

#[test]
fn incomplete_records_and_invalid_configuration_are_rejected() {
    let data = payload(fixtures::OPENSSL_AES128_RECORD);
    for length in 0..data.len() {
        assert!(parse(&data[..length]).is_err(), "prefix {length}");
    }
    for configuration in [0x0aff, 0x2dff, 0x0d00] {
        assert_eq!(
            Mode13Payload::parse(ConfigurationField::from(configuration), &data).err(),
            Some(Mode13Error::InvalidConfiguration)
        );
    }
    let mut altered = data.clone();
    altered[0] = 2;
    assert_eq!(
        parse(&altered).err(),
        Some(Mode13Error::InvalidConfiguration)
    );
    altered = data.clone();
    altered[3] = 4;
    assert_eq!(parse(&altered).err(), Some(Mode13Error::UnsupportedVersion));
    altered = data;
    altered[1] = 24;
    assert_eq!(
        parse(&altered).err(),
        Some(Mode13Error::UnsupportedContentType)
    );
}

#[test]
fn channel_and_handshake_records_are_inspected_without_decryption() {
    let channel = parse(&[0, 0, 0, 0, 0, 0]).unwrap();
    assert!(channel.is_channel_request());
    assert_eq!(
        channel.records().next().unwrap().content_type,
        TlsContentType::ChannelRequest
    );
    let handshake = parse(&[1, 22, 3, 3, 0, 4, 14, 0, 0, 0]).unwrap();
    assert_eq!(handshake.records().next().unwrap().payload, &[14, 0, 0, 0]);
    let mut output = [0xaa; 3];
    assert_eq!(
        handshake.decrypt_into(&session(&[0; 16]), &mut output),
        Err(Mode13Error::NotApplicationData)
    );
    assert_eq!(output, [0xaa; 3]);
}

#[test]
fn insufficient_output_and_counter_overflow_preserve_the_buffer() {
    let key: Vec<u8> = (0..16).collect();
    let mut data = payload(fixtures::OPENSSL_AES128_RECORD);
    let mut short = [0xaa; 2];
    assert_eq!(
        parse(&data)
            .unwrap()
            .decrypt_into(&session(&key), &mut short),
        Err(Mode13Error::InvalidDataLength)
    );
    assert_eq!(short, [0xaa; 2]);
    data.extend(bytes(fixtures::OPENSSL_AES128_SECOND_RECORD));
    let mut session = session(&key);
    session.sequence_number = u64::MAX;
    let mut output = [0xaa; 6];
    assert_eq!(
        parse(&data).unwrap().decrypt_into(&session, &mut output),
        Err(Mode13Error::SequenceNumberOverflow)
    );
    assert_eq!(output, [0xaa; 6]);
}
