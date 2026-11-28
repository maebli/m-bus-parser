use crate::ApplicationLayerError;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AuthenticationHeader<'a> {
    pub message_control: u8,
    pub key_information: Option<u16>,
    pub message_counter: u32,
    pub message_length: Option<u16>,
    pub mac: &'a [u8],
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub transport: &'a [u8],
}

impl<'a> AuthenticationHeader<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ApplicationLayerError> {
        let [0x90, length, rest @ ..] = data else {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "AFL header",
            });
        };
        let (header, transport) = rest
            .split_at_checked(usize::from(*length))
            .ok_or(ApplicationLayerError::InsufficientData)?;
        let [low, high, control, remaining @ ..] = header else {
            return Err(ApplicationLayerError::InsufficientData);
        };
        let mut remaining = remaining;
        let flags = u16::from_le_bytes([*low, *high]);
        let tls_mode = match transport.first() {
            Some(0x5f | 0x9f) => transport.get(12),
            Some(0x9e) => transport.get(4),
            _ => None,
        };
        let complete_tls_handshake =
            flags & 0xc1ff == 1 && tls_mode.is_some_and(|cf| cf & 0x1f == 13);
        if flags & 0xc1ff != 0 && !complete_tls_handshake {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "fragmented or reserved AFL control",
            });
        }
        if flags & 0x2c00 != 0x2c00
            || control & 0x80 != 0
            || control & 0x20 == 0
            || (flags & 0x0200 != 0) != (control & 0x10 != 0)
            || (flags & 0x1000 != 0) != (control & 0x40 != 0)
        {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "AFL authentication fields",
            });
        }
        let key_information = if flags & 0x0200 != 0 {
            let [low, high, rest @ ..] = remaining else {
                return Err(ApplicationLayerError::InsufficientData);
            };
            remaining = rest;
            Some(u16::from_le_bytes([*low, *high]))
        } else {
            None
        };
        let [c0, c1, c2, c3, rest @ ..] = remaining else {
            return Err(ApplicationLayerError::InsufficientData);
        };
        let message_counter = u32::from_le_bytes([*c0, *c1, *c2, *c3]);
        remaining = rest;
        let mac_length = match control & 0x0f {
            5 => 8,
            6 => 12,
            7 => 16,
            _ => {
                return Err(ApplicationLayerError::InvalidPayload {
                    field: "AFL authentication type",
                });
            }
        };
        let (mac, remaining) = remaining
            .split_at_checked(mac_length)
            .ok_or(ApplicationLayerError::InsufficientData)?;
        let message_length = if flags & 0x1000 != 0 {
            let [low, high] = remaining else {
                return Err(ApplicationLayerError::InvalidPayload {
                    field: "AFL message length",
                });
            };
            let length = u16::from_le_bytes([*low, *high]);
            if usize::from(length) != transport.len() || length > 16_384 {
                return Err(ApplicationLayerError::InvalidPayload {
                    field: "AFL message length",
                });
            }
            Some(length)
        } else {
            if !remaining.is_empty() {
                return Err(ApplicationLayerError::InvalidPayload {
                    field: "AFL length",
                });
            }
            None
        };
        if transport.is_empty() || matches!(transport.first(), Some(0x8c..=0x90)) {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "AFL transport header",
            });
        }
        Ok(Self {
            message_control: *control,
            key_information,
            message_counter,
            message_length,
            mac,
            transport,
        })
    }
}
