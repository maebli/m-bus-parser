use super::DecryptionError;
use aes::Aes128;
use ccm::{
    AeadInOut, Ccm, KeyInit,
    consts::{U8, U13},
};

pub fn decrypt_asp20(
    body: &mut [u8],
    key: &[u8; 16],
    meter: &[u8; 8],
    downlink: bool,
) -> Result<(), DecryptionError> {
    let (counter, protected) = body
        .split_first_chunk_mut::<4>()
        .ok_or(DecryptionError::InvalidDataLength)?;
    let (ciphertext, tag) = protected
        .split_last_chunk_mut::<8>()
        .ok_or(DecryptionError::InvalidDataLength)?;
    if ciphertext.len() < 6 {
        return Err(DecryptionError::InvalidDataLength);
    }
    let mut nonce = [0; 13];
    nonce[..8].copy_from_slice(meter);
    nonce[8] = if downlink { 1 } else { 2 };
    nonce[9..].copy_from_slice(&u32::from_le_bytes(*counter).to_be_bytes());
    let cipher = Ccm::<Aes128, U8, U13>::new(key.into());
    if cipher
        .decrypt_inout_detached(
            (&nonce).into(),
            &[],
            (&mut *ciphertext).into(),
            (&*tag).into(),
        )
        .is_err()
    {
        ciphertext.fill(0);
        return Err(DecryptionError::AuthenticationFailed);
    }
    Ok(())
}
