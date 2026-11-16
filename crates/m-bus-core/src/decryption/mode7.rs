use super::{DecryptionError, KeyContext, KeyProvider, decimal_to_bcd, decrypt_aes_cbc_into};
use crate::{ConfigurationField, SecurityMode, authentication::AuthenticationHeader};
use aes::Aes128;
use cipher::KeyInit;
use cmac::{Cmac, Mac};

pub struct Mode7Payload<'a> {
    pub data: &'a [u8],
    pub context: KeyContext,
    pub configuration: ConfigurationField,
    pub from_meter: bool,
    pub authentication: &'a AuthenticationHeader<'a>,
}

impl Mode7Payload<'_> {
    pub fn decrypt_into<K: KeyProvider>(
        &self,
        provider: &K,
        output: &mut [u8],
    ) -> Result<usize, DecryptionError> {
        let mac_length = match self.authentication.message_control & 0x0f {
            5 => 8,
            6 => 12,
            7 => 16,
            _ => return Err(DecryptionError::UnsupportedConfiguration),
        };
        if self.authentication.mac.len() != mac_length
            || self.authentication.message_control & 0xa0 != 0x20
        {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        if self.context.security_mode != SecurityMode::AesCbc128IvZero {
            return Err(DecryptionError::UnsupportedMode(self.context.security_mode));
        }
        if self.configuration.security_mode() != SecurityMode::AesCbc128IvZero
            || self.configuration.raw() & 0x2000 != 0
            || self.context.identification_number > 99_999_999
            || !self.authentication.transport.ends_with(self.data)
        {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        let [extension, rest @ ..] = self.data else {
            return Err(DecryptionError::InvalidDataLength);
        };
        if extension & 0xb0 != 0x10 {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        let (version, data) = if extension & 0x40 != 0 {
            let [version, rest @ ..] = rest else {
                return Err(DecryptionError::InvalidDataLength);
            };
            (*version, rest)
        } else {
            (0, rest)
        };
        if self
            .authentication
            .key_information
            .is_some_and(|information| {
                information as u8 != extension & 0x3f
                    || (extension & 0x40 != 0 && information >> 8 != u16::from(version))
            })
        {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        let encrypted_length = match (self.configuration.raw() >> 4) & 0x0f {
            15 => data.len(),
            blocks => usize::from(blocks) * 16,
        };
        if encrypted_length > data.len() || encrypted_length % 16 != 0 {
            return Err(DecryptionError::InvalidDataLength);
        }
        let target = output
            .get_mut(..data.len())
            .ok_or(DecryptionError::InvalidDataLength)?;
        let key = provider.get_key(&self.context)?;
        let counter = self.authentication.message_counter.to_le_bytes();
        let mut derivation = [7; 16];
        derivation[0] = if self.from_meter { 0x01 } else { 0x11 };
        derivation[1..5].copy_from_slice(&counter);
        derivation[5..9].copy_from_slice(&decimal_to_bcd(self.context.identification_number));
        let mut kdf =
            Cmac::<Aes128>::new_from_slice(key).map_err(|_| DecryptionError::InvalidKeyLength)?;
        kdf.update(&derivation);
        let authentication_key = kdf.finalize().into_bytes();
        let mut mac = Cmac::<Aes128>::new_from_slice(&authentication_key)
            .map_err(|_| DecryptionError::InvalidKeyLength)?;
        mac.update(&[self.authentication.message_control]);
        mac.update(&counter);
        if let Some(length) = self.authentication.message_length {
            mac.update(&length.to_le_bytes());
        }
        mac.update(self.authentication.transport);
        if mac.verify_truncated_left(self.authentication.mac).is_err() {
            target.fill(0);
            return Err(DecryptionError::AuthenticationFailed);
        }
        if encrypted_length == 0 {
            if self.configuration.raw() & 0x08 != 0 {
                return Err(DecryptionError::UnsupportedConfiguration);
            }
            target.copy_from_slice(data);
            return Ok(data.len());
        }
        derivation[0] = if self.from_meter { 0x00 } else { 0x10 };
        let mut kdf =
            Cmac::<Aes128>::new_from_slice(key).map_err(|_| DecryptionError::InvalidKeyLength)?;
        kdf.update(&derivation);
        let encryption_key = kdf.finalize().into_bytes();
        target.copy_from_slice(data);
        decrypt_aes_cbc_into(
            &data[..encrypted_length],
            &encryption_key,
            &[0; 16],
            &mut target[..encrypted_length],
        )?;
        if !target.starts_with(&[0x2f, 0x2f]) {
            target.fill(0);
            return Err(DecryptionError::DecryptionFailed);
        }
        let padding_length = if self.configuration.raw() & 0x08 != 0 {
            let padding = usize::from(target[encrypted_length - 1]);
            if !(1..=16).contains(&padding)
                || padding > encrypted_length - 2
                || !target[encrypted_length - padding..encrypted_length]
                    .iter()
                    .all(|byte| usize::from(*byte) == padding)
            {
                target.fill(0);
                return Err(DecryptionError::DecryptionFailed);
            }
            padding
        } else {
            0
        };
        target.copy_within(2..encrypted_length - padding_length, 0);
        target.copy_within(
            encrypted_length..data.len(),
            encrypted_length - padding_length - 2,
        );
        let length = data.len() - padding_length - 2;
        target[length..].fill(0);
        Ok(length)
    }
}
