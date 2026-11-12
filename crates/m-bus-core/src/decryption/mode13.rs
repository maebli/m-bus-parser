use crate::{ConfigurationField, SecurityMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode13Error {
    InvalidConfiguration,
    IncompleteRecord,
    UnsupportedContentType,
    UnsupportedVersion,
    InvalidDataLength,
    InvalidKeyLength,
    AuthenticationFailed,
    SequenceNumberOverflow,
    NotApplicationData,
}

impl core::fmt::Display for Mode13Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "invalid mode 13 configuration or extension",
            Self::IncompleteRecord => "incomplete TLS record; supply complete, reassembled records",
            Self::UnsupportedContentType => "unsupported TLS content type",
            Self::UnsupportedVersion => "only TLS 1.2 records are supported",
            Self::InvalidDataLength => "invalid TLS record or output length",
            Self::InvalidKeyLength => "TLS key length does not match the cipher suite",
            Self::AuthenticationFailed => {
                "TLS record authentication failed; check the session, direction, and counter"
            }
            Self::SequenceNumberOverflow => "TLS record counter would overflow",
            Self::NotApplicationData => "offline decryption requires TLS application records",
        })
    }
}

impl core::error::Error for Mode13Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsContentType {
    ChannelRequest,
    ChangeCipherSpec,
    Alert,
    Handshake,
    ApplicationData,
}

#[derive(Debug, Clone, Copy)]
pub struct TlsRecord<'a> {
    pub content_type: TlsContentType,
    pub version: [u8; 2],
    pub payload: &'a [u8],
    pub encoded: &'a [u8],
}

fn parse_record(data: &[u8]) -> Result<(TlsRecord<'_>, &[u8]), Mode13Error> {
    let [kind, major, minor, length_high, length_low, rest @ ..] = data else {
        return Err(Mode13Error::IncompleteRecord);
    };
    let content_type = match kind {
        0 => TlsContentType::ChannelRequest,
        20 => TlsContentType::ChangeCipherSpec,
        21 => TlsContentType::Alert,
        22 => TlsContentType::Handshake,
        23 => TlsContentType::ApplicationData,
        _ => return Err(Mode13Error::UnsupportedContentType),
    };
    let version = [*major, *minor];
    let length = usize::from(u16::from_be_bytes([*length_high, *length_low]));
    if content_type == TlsContentType::ChannelRequest {
        if version != [0, 0] || length != 0 {
            return Err(Mode13Error::InvalidConfiguration);
        }
    } else if version != [3, 3] {
        return Err(Mode13Error::UnsupportedVersion);
    }
    if length > 18_432 {
        return Err(Mode13Error::InvalidDataLength);
    }
    let (payload, remaining) = rest
        .split_at_checked(length)
        .ok_or(Mode13Error::IncompleteRecord)?;
    let encoded = data
        .get(..5 + length)
        .ok_or(Mode13Error::IncompleteRecord)?;
    Ok((
        TlsRecord {
            content_type,
            version,
            payload,
            encoded,
        },
        remaining,
    ))
}

#[derive(Debug, Clone, Copy)]
pub struct Mode13Payload<'a> {
    records: &'a [u8],
    channel_request: bool,
}

impl<'a> Mode13Payload<'a> {
    pub fn parse(configuration: ConfigurationField, data: &'a [u8]) -> Result<Self, Mode13Error> {
        if configuration.security_mode() != SecurityMode::SpecificUsage13
            || configuration.raw() & 0x2000 != 0
            || configuration.mode_specific_lower() != 0xff
        {
            return Err(Mode13Error::InvalidConfiguration);
        }
        let [extension, records @ ..] = data else {
            return Err(Mode13Error::IncompleteRecord);
        };
        if *extension > 1 {
            return Err(Mode13Error::InvalidConfiguration);
        }
        if records.is_empty() {
            return Err(Mode13Error::IncompleteRecord);
        }
        let channel_request = *extension == 0;
        let mut remaining = records;
        while !remaining.is_empty() {
            let (record, rest) = parse_record(remaining)?;
            if channel_request != (record.content_type == TlsContentType::ChannelRequest)
                || (channel_request && !rest.is_empty())
            {
                return Err(Mode13Error::InvalidConfiguration);
            }
            remaining = rest;
        }
        Ok(Self {
            records,
            channel_request,
        })
    }

    pub fn is_channel_request(&self) -> bool {
        self.channel_request
    }

    pub fn records(&self) -> TlsRecords<'a> {
        TlsRecords {
            remaining: self.records,
        }
    }

    pub fn encoded_records(&self) -> &'a [u8] {
        self.records
    }

    pub fn decrypt_into(
        &self,
        session: &TlsSession<'_>,
        output: &mut [u8],
    ) -> Result<usize, Mode13Error> {
        if session.key.len() != session.cipher_suite.key_len() {
            return Err(Mode13Error::InvalidKeyLength);
        }
        let mut total = 0usize;
        let mut count = 0u64;
        for record in self.records() {
            if record.content_type != TlsContentType::ApplicationData {
                return Err(Mode13Error::NotApplicationData);
            }
            let length = record
                .payload
                .len()
                .checked_sub(24)
                .ok_or(Mode13Error::InvalidDataLength)?;
            if length > 16_384 {
                return Err(Mode13Error::InvalidDataLength);
            }
            total = total
                .checked_add(length)
                .ok_or(Mode13Error::InvalidDataLength)?;
            count = count
                .checked_add(1)
                .ok_or(Mode13Error::SequenceNumberOverflow)?;
        }
        session
            .sequence_number
            .checked_add(count - 1)
            .ok_or(Mode13Error::SequenceNumberOverflow)?;
        let target = output
            .get_mut(..total)
            .ok_or(Mode13Error::InvalidDataLength)?;
        let mut offset = 0;
        for (index, record) in self.records().enumerate() {
            let length = record.payload.len() - 24;
            let result = decrypt_record(
                &record,
                session,
                session.sequence_number + index as u64,
                &mut target[offset..offset + length],
            );
            if let Err(error) = result {
                target.fill(0);
                return Err(error);
            }
            offset += length;
        }
        Ok(total)
    }
}

#[derive(Debug, Clone)]
pub struct TlsRecords<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for TlsRecords<'a> {
    type Item = TlsRecord<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (record, rest) = parse_record(self.remaining).ok()?;
        self.remaining = rest;
        Some(record)
    }
}

impl core::iter::FusedIterator for TlsRecords<'_> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsCipherSuite {
    EcdheEcdsaAes128GcmSha256,
    EcdheEcdsaAes256GcmSha384,
}

impl TlsCipherSuite {
    pub const fn key_len(self) -> usize {
        match self {
            Self::EcdheEcdsaAes128GcmSha256 => 16,
            Self::EcdheEcdsaAes256GcmSha384 => 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsDirection {
    ClientToServer,
    ServerToClient,
}

pub struct TlsSession<'a> {
    pub cipher_suite: TlsCipherSuite,
    pub direction: TlsDirection,
    pub key: &'a [u8],
    pub fixed_iv: [u8; 4],
    pub sequence_number: u64,
}

fn decrypt_record(
    record: &TlsRecord<'_>,
    session: &TlsSession<'_>,
    sequence_number: u64,
    output: &mut [u8],
) -> Result<(), Mode13Error> {
    use aes_gcm::{AeadInOut, Aes128Gcm, Aes256Gcm, KeyInit, Tag};

    let (explicit_nonce, encrypted) = record.payload.split_at(8);
    let (ciphertext, tag) = encrypted.split_at(encrypted.len() - 16);
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(&session.fixed_iv);
    nonce[4..].copy_from_slice(explicit_nonce);
    let mut associated_data = [0u8; 13];
    associated_data[..8].copy_from_slice(&sequence_number.to_be_bytes());
    associated_data[8..11].copy_from_slice(&[23, 3, 3]);
    associated_data[11..].copy_from_slice(&(ciphertext.len() as u16).to_be_bytes());
    let tag = Tag::try_from(tag).map_err(|_| Mode13Error::InvalidDataLength)?;
    output.copy_from_slice(ciphertext);
    let result = match session.cipher_suite {
        TlsCipherSuite::EcdheEcdsaAes128GcmSha256 => Aes128Gcm::new_from_slice(session.key)
            .map_err(|_| Mode13Error::InvalidKeyLength)?
            .decrypt_inout_detached((&nonce).into(), &associated_data, output.into(), &tag),
        TlsCipherSuite::EcdheEcdsaAes256GcmSha384 => Aes256Gcm::new_from_slice(session.key)
            .map_err(|_| Mode13Error::InvalidKeyLength)?
            .decrypt_inout_detached((&nonce).into(), &associated_data, output.into(), &tag),
    };
    result.map_err(|_| Mode13Error::AuthenticationFailed)
}
