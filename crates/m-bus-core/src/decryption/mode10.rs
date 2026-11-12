use super::{DecryptionError, KeyContext, KeyProvider, decimal_to_bcd};
use crate::{ConfigurationField, SecurityMode};
use aes::Aes128;
use ccm::{
    AeadInOut, Ccm, KeyInit, Tag,
    consts::{U8, U13},
};
use cmac::{Cmac, Mac};

type Aes128Ccm = Ccm<Aes128, U8, U13>;

#[derive(Debug)]
pub struct Mode10Payload<'a> {
    pub data: &'a [u8],
    pub context: KeyContext,
    pub control_information: u8,
    pub status: u8,
    pub configuration: ConfigurationField,
    pub from_meter: bool,
}

impl Mode10Payload<'_> {
    pub fn decrypt_into<K: KeyProvider>(
        &self,
        provider: &K,
        output: &mut [u8],
    ) -> Result<usize, DecryptionError> {
        if self.context.security_mode != SecurityMode::AesCcm128 {
            return Err(DecryptionError::UnsupportedMode(self.context.security_mode));
        }
        if self.configuration.security_mode() != SecurityMode::AesCcm128
            || self.configuration.raw() & 0x2000 == 0
        {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        let [extension_low, extension_high, ..] = self.data else {
            return Err(DecryptionError::InvalidDataLength);
        };
        let extension = u16::from_le_bytes([*extension_low, *extension_high]);
        if extension & 0xc080 != 0 || extension & 0x0330 != 0x0110 {
            return Err(DecryptionError::UnsupportedConfiguration);
        }
        let extension_len = if extension & 0x0040 != 0 { 3 } else { 2 };
        let (authenticated_extension, remaining) = self
            .data
            .split_at_checked(extension_len)
            .ok_or(DecryptionError::InvalidDataLength)?;
        let [counter0, counter1, counter2, counter3, remaining @ ..] = remaining else {
            return Err(DecryptionError::InvalidDataLength);
        };
        let counter = [*counter0, *counter1, *counter2, *counter3];
        let payload_len = remaining
            .len()
            .checked_sub(8)
            .ok_or(DecryptionError::InvalidDataLength)?;
        let (data, tag) = remaining.split_at(payload_len);
        let encrypted_len = match self.configuration.mode_specific_lower() {
            0xff => payload_len,
            length => usize::from(length),
        };
        let (ciphertext, cleartext) = data
            .split_at_checked(encrypted_len)
            .ok_or(DecryptionError::InvalidDataLength)?;
        if cleartext.len() > 255 || ciphertext.len() > usize::from(u16::MAX) {
            return Err(DecryptionError::InvalidDataLength);
        }
        if self.context.identification_number > 99_999_999 {
            return Err(DecryptionError::InvalidDataLength);
        }
        let identification = decimal_to_bcd(self.context.identification_number);
        let key = provider.get_key(&self.context)?;
        let mut derivation = [7u8; 16];
        derivation[0] = if self.from_meter { 0x00 } else { 0x10 };
        derivation[1..5].copy_from_slice(&counter);
        derivation[5..9].copy_from_slice(&identification);
        let mut mac =
            Cmac::<Aes128>::new_from_slice(key).map_err(|_| DecryptionError::InvalidKeyLength)?;
        mac.update(&derivation);
        let session_key = mac.finalize().into_bytes();
        let cipher = Aes128Ccm::new(&session_key);
        let mut nonce = [0u8; 13];
        nonce[..2].copy_from_slice(&self.context.manufacturer.to_id().to_le_bytes());
        nonce[2..6].copy_from_slice(&identification);
        nonce[6] = self.context.version;
        nonce[7] = self.context.device_type.into();
        nonce[9..].copy_from_slice(&u32::from_le_bytes(counter).to_be_bytes());
        let configuration = self.configuration.raw().to_le_bytes();
        let mut associated_data = [0u8; 263];
        associated_data[..5].copy_from_slice(&[
            self.control_information,
            self.context.access_number,
            self.status,
            configuration[0],
            configuration[1],
        ]);
        let header_len = 5 + extension_len;
        associated_data[5..header_len].copy_from_slice(authenticated_extension);
        let associated_data_len = header_len + cleartext.len();
        associated_data[header_len..associated_data_len].copy_from_slice(cleartext);
        let tag = Tag::<U8>::try_from(tag).map_err(|_| DecryptionError::InvalidDataLength)?;
        let target = output
            .get_mut(..payload_len)
            .ok_or(DecryptionError::InvalidDataLength)?;
        target.copy_from_slice(data);
        if cipher
            .decrypt_inout_detached(
                (&nonce).into(),
                &associated_data[..associated_data_len],
                (&mut target[..encrypted_len]).into(),
                &tag,
            )
            .is_err()
        {
            target.fill(0);
            return Err(DecryptionError::AuthenticationFailed);
        }
        Ok(payload_len)
    }
}
