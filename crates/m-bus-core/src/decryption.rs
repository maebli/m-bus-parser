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
    pub access_number: u8,
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

impl core::fmt::Display for DecryptionError {
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

impl core::error::Error for DecryptionError {}

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

    #[cfg(feature = "decryption")]
    fn _derive_iv(&self) -> [u8; 16] {
        let mut iv = [0u8; 16];
        // Bytes 0-1: Manufacturer ID (numeric, little-endian)
        let mfr_id = self.context.manufacturer.to_id();
        iv[0..2].copy_from_slice(&mfr_id.to_le_bytes());
        // Bytes 2-5: Identification number as BCD bytes (how they appear in frame)
        let bcd_bytes = decimal_to_bcd(self.context.identification_number);
        iv[2..6].copy_from_slice(&bcd_bytes);
        // Byte 6: Version
        iv[6] = self.context.version;
        // Byte 7: Device type
        iv[7] = self.context.device_type.into();
        // Bytes 8-15: Access number repeated 8 times
        iv[8..16].fill(self.context.access_number);
        iv
    }
}

/// Convert a decimal number to BCD bytes (little-endian, 4 bytes)
/// e.g., 14639203 -> [0x03, 0x92, 0x63, 0x14]
#[cfg(feature = "decryption")]
fn decimal_to_bcd(mut value: u32) -> [u8; 4] {
    let mut bcd = [0u8; 4];
    for byte in &mut bcd {
        let low = (value % 10) as u8;
        value /= 10;
        let high = (value % 10) as u8;
        value /= 10;
        *byte = (high << 4) | low;
    }
    bcd
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
            access_number: 0x00,
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
            access_number: 0x50,
        };

        let payload = EncryptedPayload { data: &[], context };

        let iv = payload._derive_iv();

        // Check manufacturer ID (first 2 bytes should be 0x1ee6 in little-endian)
        assert_eq!(iv[0], 0xe6);
        assert_eq!(iv[1], 0x1e);

        // Check identification number (bytes 2-5, as BCD)
        // 12345678 decimal -> BCD [0x78, 0x56, 0x34, 0x12]
        assert_eq!(&iv[2..6], &[0x78, 0x56, 0x34, 0x12]);

        // Check version (byte 6)
        assert_eq!(iv[6], 0x42);

        // Check device type (byte 7) - WaterMeter should be 0x07
        assert_eq!(iv[7], 0x07);

        // Check access number repeated (bytes 8-15)
        for i in 8..16 {
            assert_eq!(iv[i], 0x50);
        }
    }

    #[test]
    fn test_mode5_decryption_real_frame() {
        // Test vector from real wMBus Mode 5 encrypted frame
        // Frame: 6644496A3100015514377203926314496A00075000500598A78E0D...
        // Key: F8B24F12F9D113F680BEE765FDE67EC0
        // Expected IV: 496A0392631400075050505050505050

        let key = [
            0xF8, 0xB2, 0x4F, 0x12, 0xF9, 0xD1, 0x13, 0xF6, 0x80, 0xBE, 0xE7, 0x65, 0xFD, 0xE6,
            0x7E, 0xC0,
        ];

        // Encrypted payload (80 bytes after TPL header)
        let encrypted = [
            0x98, 0xA7, 0x8E, 0x0D, 0x71, 0xAA, 0x63, 0x58, 0xEE, 0xBD, 0x0B, 0x20, 0xBF, 0xDF,
            0x99, 0xED, 0xA2, 0xD2, 0x2F, 0xA2, 0x53, 0x14, 0xF3, 0xF1, 0xB8, 0x44, 0x70, 0x89,
            0x8E, 0x49, 0x53, 0x03, 0x92, 0x37, 0x70, 0xBA, 0x8D, 0xDA, 0x97, 0xC9, 0x64, 0xF0,
            0xEA, 0x6C, 0xE2, 0x4F, 0x56, 0x50, 0xC0, 0xA6, 0xCD, 0xF3, 0xDE, 0x37, 0xDE, 0x33,
            0xFB, 0xFB, 0xEB, 0xAC, 0xE4, 0x00, 0x9B, 0xB0, 0xD8, 0xEB, 0xA2, 0xCB, 0xE8, 0x04,
            0x33, 0xFF, 0x13, 0x13, 0x28, 0x20, 0x60, 0x20, 0xB1, 0xBF,
        ];

        // Expected decrypted data
        let expected = [
            0x2F, 0x2F, 0x0C, 0x13, 0x29, 0x73, 0x06, 0x00, 0x02, 0x6C, 0x94, 0x21, 0x82, 0x04,
            0x6C, 0x81, 0x21, 0x8C, 0x04, 0x13, 0x75, 0x44, 0x06, 0x00, 0x8D, 0x04, 0x93, 0x13,
            0x2C, 0xFB, 0xFE, 0x12, 0x44, 0x00, 0x51, 0x41, 0x00, 0x70, 0x35, 0x00, 0x77, 0x33,
            0x00, 0x75, 0x49, 0x00, 0x16, 0x36, 0x00, 0x73, 0x56, 0x00, 0x91, 0x55, 0x00, 0x95,
            0x57, 0x00, 0x31, 0x57, 0x00, 0x28, 0x42, 0x00, 0x18, 0x47, 0x00, 0x61, 0x39, 0x00,
            0x56, 0x42, 0x00, 0x02, 0xFD, 0x17, 0x00, 0x00, 0x2F, 0x2F,
        ];

        // Build context from TPL header:
        // Frame bytes [0x49, 0x6A] = manufacturer ID 0x6A49 when parsed as little-endian
        // ID bytes [0x03, 0x92, 0x63, 0x14] in BCD = 14639203 decimal
        let manufacturer = ManufacturerCode::from_id(0x6A49).unwrap();
        let context = KeyContext {
            manufacturer,
            identification_number: 14639203, // BCD 03926314 parsed as decimal
            version: 0x00,
            device_type: DeviceType::WaterMeter,
            security_mode: crate::SecurityMode::AesCbc128IvNonZero,
            access_number: 0x50,
        };

        // Verify IV derivation
        let payload = EncryptedPayload {
            data: &encrypted,
            context: context.clone(),
        };
        let iv = payload._derive_iv();
        // Expected IV: 496A0392631400075050505050505050
        let expected_iv = [
            0x49, 0x6A, 0x03, 0x92, 0x63, 0x14, 0x00, 0x07, 0x50, 0x50, 0x50, 0x50, 0x50, 0x50,
            0x50, 0x50,
        ];
        assert_eq!(iv, expected_iv);

        // Test decryption
        let mut provider = StaticKeyProvider::<1>::new();
        provider.add_key(0x6A49, 14639203, key).unwrap();

        let mut output = [0u8; 80];
        let len = payload.decrypt_into(&provider, &mut output).unwrap();

        assert_eq!(len, 80);
        assert_eq!(&output[..80], &expected[..]);
    }
}
