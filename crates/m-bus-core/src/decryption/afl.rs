use super::{DecryptionError, decimal_to_bcd};
use crate::authentication::AuthenticationHeader;
use aes::{Aes128, cipher::KeyInit};
use cmac::{Cmac, Mac};

pub(super) fn ephemeral_key(
    key: &[u8],
    constant: u8,
    counter: u32,
    identification_number: u32,
) -> Result<[u8; 16], DecryptionError> {
    let mut derivation = [7; 16];
    derivation[0] = constant;
    derivation[1..5].copy_from_slice(&counter.to_le_bytes());
    derivation[5..9].copy_from_slice(&decimal_to_bcd(identification_number));
    let mut kdf =
        Cmac::<Aes128>::new_from_slice(key).map_err(|_| DecryptionError::InvalidKeyLength)?;
    kdf.update(&derivation);
    Ok(kdf.finalize().into_bytes().into())
}

pub fn verify_afl_cmac(
    authentication: &AuthenticationHeader<'_>,
    key: &[u8],
    identification_number: u32,
    from_meter: bool,
) -> Result<(), DecryptionError> {
    let required_length = match authentication.message_control & 15 {
        5 => 8,
        6 => 12,
        7 => 16,
        _ => return Err(DecryptionError::UnsupportedConfiguration),
    };
    if authentication.mac.len() != required_length {
        return Err(DecryptionError::InvalidDataLength);
    }
    let derived = ephemeral_key(
        key,
        if from_meter { 1 } else { 0x11 },
        authentication.message_counter,
        identification_number,
    )?;
    let mut mac =
        Cmac::<Aes128>::new_from_slice(&derived).map_err(|_| DecryptionError::InvalidKeyLength)?;
    mac.update(&[authentication.message_control]);
    mac.update(&authentication.message_counter.to_le_bytes());
    if let Some(length) = authentication.message_length {
        mac.update(&length.to_le_bytes());
    }
    mac.update(authentication.transport);
    mac.verify_truncated_left(authentication.mac)
        .map_err(|_| DecryptionError::AuthenticationFailed)
}
