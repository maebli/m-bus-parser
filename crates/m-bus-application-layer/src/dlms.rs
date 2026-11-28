pub use dlms_cosem_kit::xdlms::{Apdu, ApduTag, GetDataResult, GetRequest, GetResponse};

#[derive(Clone, Copy, Debug, Default)]
pub struct DlmsPayload<'a> {
    pub tag: Option<ApduTag>,
    pub apdu: Option<Apdu<'a>>,
    pub valid: Option<bool>,
    pub uplink: Option<bool>,
    pub protected: bool,
    pub invoke: Option<u8>,
}

impl<'a> DlmsPayload<'a> {
    pub fn decode(payload: &'a [u8], padded: bool) -> Self {
        use dlms_cosem_kit::codec::{Decode, ErrorKind, Reader};
        let [_, _, data @ ..] = payload else {
            return Self {
                valid: Some(false),
                ..Self::default()
            };
        };
        let Some(tag) = data.first().copied().and_then(ApduTag::from_u8) else {
            return Self::default();
        };
        let mut outer = None;
        let mut protected = false;
        let mut current = data;
        let mut allow_padding = padded;
        let mut depth = 0;
        let valid = loop {
            let mut reader = Reader::new(current);
            let apdu = match Apdu::decode(&mut reader) {
                Ok(apdu) => apdu,
                Err(err)
                    if matches!(
                        err.kind,
                        ErrorKind::Unsupported | ErrorKind::WorkExceeded | ErrorKind::DepthExceeded
                    ) =>
                {
                    break None;
                }
                Err(_) => break Some(false),
            };
            if !(reader.is_empty() || (allow_padding && reader.rest().iter().all(|b| *b == 0x2f))) {
                break Some(false);
            }
            outer.get_or_insert(apdu);
            protected |= apdu.is_protected();
            let inner = match apdu {
                Apdu::Gateway { payload, .. } => Some(payload),
                Apdu::Aarq(message) => message.user_information,
                Apdu::Aare(message) => message.user_information,
                Apdu::Rlrq(message) => message.user_information,
                Apdu::Rlre(message) => message.user_information,
                _ => None,
            };
            let Some(inner) = inner else { break Some(true) };
            if inner.len() >= current.len() {
                break Some(false);
            }
            depth += 1;
            if depth > 32 {
                break None;
            }
            current = inner;
            allow_padding = false;
        };
        let plain = tag.unprotect().map_or(tag, |(_, plain)| plain);
        let uplink = match plain {
            ApduTag::Aarq
            | ApduTag::ReleaseRequest
            | ApduTag::InitiateRequest
            | ApduTag::ReadRequest
            | ApduTag::WriteRequest
            | ApduTag::UnconfirmedWriteRequest
            | ApduTag::GetRequest
            | ApduTag::SetRequest
            | ApduTag::ActionRequest
            | ApduTag::AccessRequest => Some(false),
            ApduTag::Aare
            | ApduTag::ReleaseResponse
            | ApduTag::InitiateResponse
            | ApduTag::ReadResponse
            | ApduTag::WriteResponse
            | ApduTag::GetResponse
            | ApduTag::SetResponse
            | ApduTag::ActionResponse
            | ApduTag::AccessResponse
            | ApduTag::EventNotificationRequest
            | ApduTag::InformationReportRequest => Some(true),
            _ => None,
        };
        Self {
            tag: Some(tag),
            apdu: outer,
            valid,
            uplink,
            protected,
            invoke: if matches!(
                tag,
                ApduTag::GetRequest
                    | ApduTag::SetRequest
                    | ApduTag::ActionRequest
                    | ApduTag::GetResponse
                    | ApduTag::SetResponse
                    | ApduTag::ActionResponse
            ) {
                data.get(2).copied()
            } else {
                None
            },
        }
    }
}
