use crate::ApplicationLayerError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SitpBlock<'a> {
    pub block_id: u8,
    pub block_control: u8,
    pub recipient: u8,
    pub data_selector: u8,
    pub key_id: u8,
    pub key_version: u8,
    pub body: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct SitpBlocks<'a>(&'a [u8]);

impl<'a> SitpBlocks<'a> {
    pub const fn new(payload: &'a [u8]) -> Self {
        Self(payload)
    }
}

impl<'a> Iterator for SitpBlocks<'a> {
    type Item = Result<SitpBlock<'a>, ApplicationLayerError>;
    fn next(&mut self) -> Option<Self::Item> {
        let data = core::mem::take(&mut self.0);
        if data.is_empty() || data == [0, 0] {
            return None;
        }
        let Some((length, rest)) = data.split_first_chunk::<2>() else {
            return Some(Err(ApplicationLayerError::InsufficientData));
        };
        let size = usize::from(u16::from_le_bytes(*length));
        let Some((fields, next)) = rest.split_at_checked(size) else {
            return Some(Err(ApplicationLayerError::InsufficientData));
        };
        let [block_id, block_control, recipient, data_selector, key_id, key_version, body @ ..] =
            fields
        else {
            return Some(Err(ApplicationLayerError::InvalidPayload {
                field: "SITP block length",
            }));
        };
        self.0 = next;
        Some(Ok(SitpBlock {
            block_id: *block_id,
            block_control: *block_control,
            recipient: *recipient,
            data_selector: *data_selector,
            key_id: *key_id,
            key_version: *key_version,
            body,
        }))
    }
}
