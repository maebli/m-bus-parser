use crate::{DeviceType, ManufacturerCode, SecurityMode};

#[cfg(feature = "decryption")]
use aes::Aes128;
#[cfg(feature = "decryption")]
use cbc::{
    Decryptor,
    cipher::{BlockDecryptMut, KeyIvInit},
};

#[derive(Debug, Clone, PartialEq)]
pub struct KeyContext {
    pub manufacturer: ManufacturerCode,
    pub identification_number: u32,
    pub version: u8,
    pub device_type: DeviceType,
    pub security_mode: SecurityMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecryptionError {
    UnsupportedMode(SecurityMode),
    KeyNotFound,
    DecryptionFailed,
    InvalidKeyLength,
    InvalidDataLength,
    NotEncrypted,
    UnknownEncryptionState,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DecryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedMode(mode) => write!(f, "Unsupported security mode: {:?}", mode),
            Self::KeyNotFound => write!(f, "Decryption key not found"),
            Self::DecryptionFailed => write!(f, "Decryption operation failed"),
            Self::InvalidKeyLength => write!(f, "Invalid key length"),
            Self::InvalidDataLength => write!(f, "Invalid data length"),
            Self::NotEncrypted => write!(f, "Data is not encrypted"),
            Self::UnknownEncryptionState => {
                write!(f, "Unknown encryption state for this data block type")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecryptionError {}

pub trait KeyProvider {
    fn get_key(&self, context: &KeyContext) -> Result<&[u8], DecryptionError>;
}

#[derive(Debug)]
pub struct EncryptedPayload<'a> {
    pub data: &'a [u8],
    pub context: KeyContext,
}

impl<'a> EncryptedPayload<'a> {
    pub fn new(data: &'a [u8], context: KeyContext) -> Self {
        Self { data, context }
    }

    #[cfg(feature = "decryption")]
    pub fn decrypt_into<K: KeyProvider>(
        &self,
        provider: &K,
        output: &mut [u8],
    ) -> Result<usize, DecryptionError> {
        let key = provider.get_key(&self.context)?;

        match self.context.security_mode {
            SecurityMode::NoEncryption => {
                let len = self.data.len();
                let dest = output
                    .get_mut(..len)
                    .ok_or(DecryptionError::InvalidDataLength)?;
                dest.copy_from_slice(self.data);
                Ok(len)
            }
            SecurityMode::AesCbc128IvZero => {
                decrypt_aes_cbc_into(self.data, key, &[0u8; 16], output)
            }
            SecurityMode::AesCbc128IvNonZero => {
                let iv = self._derive_iv();
                decrypt_aes_cbc_into(self.data, key, &iv, output)
            }
            mode => Err(DecryptionError::UnsupportedMode(mode)),
        }
    }

    fn _derive_iv(&self) -> [u8; 16] {
        let mut iv = [0u8; 16];
        iv[0] = self.context.manufacturer.code[0] as u8;
        iv[1] = self.context.manufacturer.code[1] as u8;
        iv[2..6].copy_from_slice(&self.context.identification_number.to_le_bytes());
        iv[10] = self.context.version;
        iv[11] = self.context.device_type.to_byte();
        iv
    }
}

pub struct StaticKeyProvider<const N: usize> {
    entries: [(u64, [u8; 16]); N],
    count: usize,
}

impl<const N: usize> Default for StaticKeyProvider<N> {
    fn default() -> Self {
        Self {
            entries: [(0, [0u8; 16]); N],
            count: 0,
        }
    }
}

impl<const N: usize> StaticKeyProvider<N> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_key(
        &mut self,
        manufacturer_id: u16,
        identification_number: u32,
        key: [u8; 16],
    ) -> Result<(), DecryptionError> {
        let key_id = Self::compute_key_id(manufacturer_id, identification_number);
        let entry = self
            .entries
            .get_mut(self.count)
            .ok_or(DecryptionError::InvalidDataLength)?;
        *entry = (key_id, key);
        self.count += 1;
        Ok(())
    }

    const fn compute_key_id(manufacturer_id: u16, identification_number: u32) -> u64 {
        ((manufacturer_id as u64) << 32) | (identification_number as u64)
    }
}

impl<const N: usize> KeyProvider for StaticKeyProvider<N> {
    fn get_key(&self, context: &KeyContext) -> Result<&[u8], DecryptionError> {
        let manufacturer_id = context.manufacturer.to_id();

        let key_id = Self::compute_key_id(manufacturer_id, context.identification_number);

        self.entries[..self.count]
            .iter()
            .find(|(id, _)| *id == key_id)
            .map(|(_, key)| key.as_slice())
            .ok_or(DecryptionError::KeyNotFound)
    }
}

#[cfg(feature = "decryption")]
fn decrypt_aes_cbc_into(
    data: &[u8],
    key: &[u8],
    iv: &[u8],
    output: &mut [u8],
) -> Result<usize, DecryptionError> {
    if key.len() != 16 {
        return Err(DecryptionError::InvalidKeyLength);
    }

    if iv.len() != 16 {
        return Err(DecryptionError::InvalidDataLength);
    }

    if data.is_empty() {
        return Err(DecryptionError::InvalidDataLength);
    }

    // Check if data length is a multiple of block size (16 bytes for AES)
    if !data.len().is_multiple_of(16) {
        return Err(DecryptionError::InvalidDataLength);
    }

    let len = data.len();

    // Copy encrypted data to output buffer
    let dest = output
        .get_mut(..len)
        .ok_or(DecryptionError::InvalidDataLength)?;
    dest.copy_from_slice(data);

    // Create decryptor
    type Aes128CbcDec = Decryptor<Aes128>;

    let key_array: [u8; 16] = key
        .try_into()
        .map_err(|_| DecryptionError::InvalidKeyLength)?;
    let iv_array: [u8; 16] = iv
        .try_into()
        .map_err(|_| DecryptionError::InvalidDataLength)?;

    let decryptor = Aes128CbcDec::new(&key_array.into(), &iv_array.into());

    // Decrypt in place
    let decrypt_buf = output
        .get_mut(..len)
        .ok_or(DecryptionError::InvalidDataLength)?;
    let decrypted = decryptor
        .decrypt_padded_mut::<cipher::block_padding::NoPadding>(decrypt_buf)
        .map_err(|_| DecryptionError::DecryptionFailed)?;

    Ok(decrypted.len())
}

#[cfg(all(test, feature = "decryption"))]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_aes_cbc_basic() {
        // Test vector: simple AES-128-CBC decryption
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let encrypted = [
            0x66, 0xe9, 0x4b, 0xd4, 0xef, 0x8a, 0x2c, 0x3b, 0x88, 0x4c, 0xfa, 0x59, 0xca, 0x34,
            0x2b, 0x2e,
        ];
        let mut output = [0u8; 16];

        let result = decrypt_aes_cbc_into(&encrypted, &key, &iv, &mut output);
        assert!(result.is_ok());
        let len = result.unwrap();
        assert_eq!(len, 16);
    }

    #[test]
    fn test_key_provider_basic() {
        let mut provider = StaticKeyProvider::<10>::new();

        // Add a test key
        let key = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];

        // Manufacturer code "ABC" -> ID calculation
        let manufacturer_code = ManufacturerCode::from_id(0x0421).unwrap(); // ABC
        let identification_number = 12345678u32;

        // Add the key
        provider
            .add_key(0x0421, identification_number, key)
            .unwrap();

        // Create context
        let context = KeyContext {
            manufacturer: manufacturer_code,
            identification_number,
            version: 0x01,
            device_type: DeviceType::WaterMeter,
            security_mode: crate::SecurityMode::AesCbc128IvZero,
        };

        // Retrieve the key
        let retrieved_key = provider.get_key(&context).unwrap();
        assert_eq!(retrieved_key, &key);
    }

    #[test]
    fn test_derive_iv() {
        let manufacturer_code = ManufacturerCode::from_id(0x1ee6).unwrap(); // GWF
        let context = KeyContext {
            manufacturer: manufacturer_code,
            identification_number: 12345678,
            version: 0x42,
            device_type: DeviceType::WaterMeter,
            security_mode: crate::SecurityMode::AesCbc128IvNonZero,
        };

        let payload = EncryptedPayload { data: &[], context };

        let iv = payload._derive_iv();

        // Check manufacturer code (first 2 bytes should be 'G' and 'W')
        assert_eq!(iv[0], b'G');
        assert_eq!(iv[1], b'W');

        // Check identification number (bytes 2-9, little endian)
        let id_bytes = 12345678u32.to_le_bytes();
        assert_eq!(&iv[2..6], &id_bytes[0..4]);

        // Check version (byte 10)
        assert_eq!(iv[10], 0x42);

        // Check device type (byte 11) - WaterMeter should be 0x07
        assert_eq!(iv[11], 0x07);
    }
}
