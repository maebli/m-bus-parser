#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtendedLinkLayer {
    pub communication_control: u8,
    pub access_number: u8,
    pub receiver_address: Option<ReceiverAddress>,
    pub encryption: Option<EncryptionFields>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReceiverAddress {
    pub manufacturer: u16,
    pub address: [u8; 6],
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EncryptionFields {
    pub session_number: [u8; 4],
    pub payload_crc: u16,
}
