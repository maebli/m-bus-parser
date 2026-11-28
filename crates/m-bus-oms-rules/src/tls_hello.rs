#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsHello {
    pub client: bool,
    pub truncated_hmac: bool,
    pub encrypt_then_mac: bool,
    pub uncompressed_points: Option<bool>,
    pub required_curves: Option<bool>,
    pub selected_gcm: Option<bool>,
    pub maximum_fragment_length: Option<u8>,
}

fn vector<'a>(data: &mut &'a [u8], wide: bool) -> Option<&'a [u8]> {
    let length = if wide {
        let (length, rest) = data.split_first_chunk::<2>()?;
        *data = rest;
        usize::from(u16::from_be_bytes(*length))
    } else {
        let (length, rest) = data.split_first()?;
        *data = rest;
        usize::from(*length)
    };
    let (value, rest) = data.split_at_checked(length)?;
    *data = rest;
    Some(value)
}

impl TlsHello {
    pub fn parse(record: &[u8]) -> Option<Self> {
        let [kind @ (1 | 2), high, mid, low, body @ ..] = record else {
            return None;
        };
        let length = usize::try_from(u32::from_be_bytes([0, *high, *mid, *low])).ok()?;
        let body = body.get(..length)?;
        let [3, 3, rest @ ..] = body else { return None };
        let mut fields = rest.get(32..)?;
        if vector(&mut fields, false)?.len() > 32 {
            return None;
        }
        let client = *kind == 1;
        let selected_gcm = if client {
            let suites = vector(&mut fields, true)?;
            if suites.is_empty() || suites.len() % 2 != 0 || vector(&mut fields, false)?.is_empty()
            {
                return None;
            }
            None
        } else {
            let [suite_hi, suite_lo, _compression, rest @ ..] = fields else {
                return None;
            };
            fields = rest;
            match u16::from_be_bytes([*suite_hi, *suite_lo]) {
                0xc02b | 0xc02c => Some(true),
                0xc023 | 0xc024 => Some(false),
                _ => None,
            }
        };
        let mut hello = Self {
            client,
            selected_gcm,
            truncated_hmac: false,
            encrypt_then_mac: false,
            uncompressed_points: None,
            required_curves: None,
            maximum_fragment_length: None,
        };
        if fields.is_empty() {
            return Some(hello);
        }
        let mut extensions = vector(&mut fields, true)?;
        if !fields.is_empty() {
            return None;
        }
        while !extensions.is_empty() {
            let ([first, second], rest) = extensions.split_first_chunk::<2>()?;
            extensions = rest;
            let mut extension = vector(&mut extensions, true)?;
            match u16::from_be_bytes([*first, *second]) {
                1 => {
                    let [size] = extension else { return None };
                    hello.maximum_fragment_length = Some(*size);
                }
                4 => hello.truncated_hmac = true,
                10 if client => {
                    let list = vector(&mut extension, true)?;
                    if !extension.is_empty() || list.len() % 2 != 0 {
                        return None;
                    }
                    let contains = |id| list.chunks_exact(2).any(|pair| pair == [0, id]);
                    hello.required_curves = Some(contains(0x17) && contains(0x1a));
                }
                11 => {
                    let formats = vector(&mut extension, false)?;
                    if !extension.is_empty() {
                        return None;
                    }
                    hello.uncompressed_points = Some(formats.contains(&0));
                }
                22 if extension.is_empty() => hello.encrypt_then_mac = true,
                _ => {}
            }
        }
        Some(hello)
    }
}
