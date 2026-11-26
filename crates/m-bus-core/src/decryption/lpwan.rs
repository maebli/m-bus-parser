use super::DecryptionError;
use aes::Aes128;
use ccm::{
    AeadInOut, Ccm, KeyInit,
    consts::{U4, U13},
};
use cmac::{Cmac, Mac};

pub struct LpwanMacSecurity {
    pub meter: [u8; 8],
    pub derivation_counter: u8,
    pub message_counter: u16,
    pub usage: u8,
}

impl LpwanMacSecurity {
    pub fn authenticate_blocks(
        &self,
        master_key: &[u8; 16],
        control_fields: &[u8],
        tag: &[u8; 4],
        blocks: &mut [u8],
    ) -> Result<(), DecryptionError> {
        let mut derivation = [9; 16];
        derivation[0] = self.derivation_counter;
        derivation[1..7].copy_from_slice(&self.meter[..6]);
        let mut mac = Cmac::<Aes128>::new_from_slice(master_key)
            .map_err(|_| DecryptionError::InvalidKeyLength)?;
        mac.update(&derivation);
        let cipher = Ccm::<Aes128, U4, U13>::new(&mac.finalize().into_bytes());
        let mut nonce = [0; 13];
        nonce[..8].copy_from_slice(&self.meter);
        nonce[8] = self.usage;
        nonce[11..].copy_from_slice(&self.message_counter.to_be_bytes());
        if cipher
            .decrypt_inout_detached(
                (&nonce).into(),
                control_fields,
                (&mut *blocks).into(),
                tag.into(),
            )
            .is_err()
        {
            blocks.fill(0);
            return Err(DecryptionError::AuthenticationFailed);
        }
        Ok(())
    }
}
