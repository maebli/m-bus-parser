use super::*;
use serde_json::{json, Value};

enum Field<'a> {
    End,
    Bytes(&'a [u8]),
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    List(Vec<Field<'a>>),
}

impl<'a> Field<'a> {
    fn read(input: &mut &'a [u8], depth: usize) -> Option<Self> {
        if depth > 24 {
            return None;
        }
        let (&first, mut rest) = input.split_first()?;
        if first == 0 {
            *input = rest;
            return Some(Self::End);
        }
        let kind = (first >> 4) & 7;
        let mut byte = first;
        let mut length = usize::from(first & 15);
        let mut header_size = 1;
        while byte & 128 != 0 {
            let (&next, tail) = rest.split_first()?;
            byte = next;
            rest = tail;
            if byte & 0x70 != 0 {
                return None;
            }
            length = length
                .checked_mul(16)?
                .checked_add(usize::from(byte & 15))?;
            header_size += 1;
        }
        if kind == 7 {
            if length > rest.len() {
                return None;
            }
            let mut items = Vec::new();
            for _ in 0..length {
                items.push(Self::read(&mut rest, depth + 1)?);
            }
            *input = rest;
            return Some(Self::List(items));
        }
        let (value, tail) = rest.split_at_checked(length.checked_sub(header_size)?)?;
        *input = tail;
        match (kind, value) {
            (0, v) => Some(Self::Bytes(v)),
            (4, [b]) => Some(Self::Bool(*b != 0)),
            (5 | 6, v) if (1..=8).contains(&v.len()) => {
                let number = v.iter().fold(0u64, |n, b| (n << 8) | u64::from(*b));
                let shift = (8 - v.len()) * 8;
                Some(if kind == 5 {
                    Self::Signed(((number << shift) as i64) >> shift)
                } else {
                    Self::Unsigned(number)
                })
            }
            _ => None,
        }
    }

    fn json(&self) -> Value {
        match self {
            Self::End | Self::Bytes([]) => Value::Null,
            Self::Bytes(bytes) => json!(hex_string(bytes)),
            Self::Bool(b) => json!(b),
            Self::Signed(n) => json!(n),
            Self::Unsigned(n) => json!(n),
            Self::List(list) => Value::Array(list.iter().map(Self::json).collect()),
        }
    }
}

fn list_entry(entry: &Field<'_>) -> Option<Value> {
    let Field::List(fields) = entry else {
        return None;
    };
    let [Field::Bytes(obis), status, time, unit, scaler, raw, signature] = fields.as_slice() else {
        return None;
    };
    let scale = match scaler {
        Field::Bytes([]) => 0,
        Field::Signed(v) => i8::try_from(*v).ok()?,
        _ => return None,
    };
    let value = match raw {
        Field::Signed(n) => json!(scaled_decimal(&n.to_string(), isize::from(scale), None)),
        Field::Unsigned(n) => json!(scaled_decimal(&n.to_string(), isize::from(scale), None)),
        _ => raw.json(),
    };
    Some(
        json!({"object_name_hex":hex_string(obis),"status":status.json(),"value_time":time.json(),
        "unit_code":unit.json(), "scaler":scale, "value":value,"value_signature":signature.json()}),
    )
}

fn messages(
    mut data: &[u8],
    mut visit: impl FnMut(&[u8], u64, u64, u64, &[Field<'_>]) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let mut read_message = false;
    while let Some(rest) = data.strip_prefix(&[0x2f]) {
        data = rest;
    }
    while !data.is_empty() {
        let original = data;
        let Some(Field::List(fields)) = Field::read(&mut data, 0) else {
            return Err("invalid SML message");
        };
        let [Field::Bytes(transaction), Field::Unsigned(group), Field::Unsigned(abort), Field::List(body), Field::Unsigned(crc), Field::End] =
            fields.as_slice()
        else {
            return Err("invalid SML message fields");
        };
        let consumed = original.len() - data.len();
        let covered = original
            .get(..consumed.checked_sub(4).ok_or("short SML checksum")?)
            .ok_or("short SML message")?;
        if original.get(covered.len()) != Some(&0x63)
            || u64::from(crc16::State::<crc16::X_25>::calculate(covered).swap_bytes()) != *crc
        {
            return Err("invalid SML message checksum");
        }
        let [Field::Unsigned(tag), Field::List(fields)] = body.as_slice() else {
            return Err("invalid SML message body");
        };
        visit(transaction, *group, *abort, *tag, fields)?;
        read_message = true;
        while let Some(rest) = data.strip_prefix(&[0x2f]) {
            data = rest;
        }
    }
    if !read_message {
        return Err("empty SML payload");
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Shape {
    Octets,
    OptionalByte,
    Uint32,
    Uint64,
    OptionalBoolean,
    List,
    OptionalList,
    Int8,
    Int16,
}

fn body_shape(tag: u64, fields: &[Field<'_>]) -> Option<bool> {
    use Shape::*;
    let shape: &[Shape] = match tag {
        0x0100 => &[Octets, Octets, Octets, Octets, Octets, Octets, OptionalByte],
        0x0101 => &[Octets, Octets, Octets, Octets, OptionalList, OptionalByte],
        0x0200 | 0x0201 => &[Octets],
        0x0300 | 0x0400 => &[
            Octets,
            Octets,
            Octets,
            OptionalBoolean,
            OptionalList,
            OptionalList,
            List,
            OptionalList,
            OptionalList,
        ],
        0x0301 => &[Octets, List, Uint32, List, List, List, Octets, Octets],
        0x0401 => &[
            Octets, List, Uint32, List, List, Uint64, List, Octets, Octets,
        ],
        0x0500 => &[Octets, Octets, Octets, List, Octets],
        0x0501 => &[Octets, List, List],
        0x0600 => &[Octets, Octets, Octets, List, List],
        0x0700 => &[Octets, Octets, Octets, Octets, Octets],
        0x0701 => &[
            Octets,
            Octets,
            Octets,
            OptionalList,
            List,
            Octets,
            OptionalList,
        ],
        0xff01 => &[Octets, Octets, Octets, OptionalList],
        0x0800 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Int16,
            Int16,
            OptionalList,
        ],
        0x0801 => &[Octets, Octets, Octets, Int16, Int16, List],
        0x0900 => &[Octets, Octets, Octets, Octets, Octets, Int16, Int16, List],
        0x0901 | 0x0a01 => &[Octets, Octets, Octets, Int16, Int16, OptionalList],
        0x0a00 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Int16,
            Int16,
            Int8,
            OptionalList,
        ],
        _ => return None,
    };
    Some(
        fields.len() == shape.len()
            && fields
                .iter()
                .zip(shape)
                .all(|(field, shape)| match (field, shape) {
                    (Field::Bytes([]), OptionalByte | OptionalBoolean | OptionalList) => true,
                    (Field::Bytes(_), Octets)
                    | (Field::Bool(_), OptionalBoolean)
                    | (Field::List(_), List | OptionalList) => true,
                    (Field::Unsigned(n), OptionalByte) => u8::try_from(*n).is_ok(),
                    (Field::Unsigned(n), Uint32) => u32::try_from(*n).is_ok(),
                    (Field::Unsigned(_), Uint64) => true,
                    (Field::Signed(n), Int8) => i8::try_from(*n).is_ok(),
                    (Field::Signed(n), Int16) => i16::try_from(*n).is_ok(),
                    _ => false,
                }),
    )
}

fn version_one_file(wire: &[u8]) -> Option<Vec<u8>> {
    const ESCAPE: [u8; 4] = [0x1b; 4];
    let mut remaining = wire.strip_prefix(&[0x1b, 0x1b, 0x1b, 0x1b, 1, 1, 1, 1])?;
    let mut payload = Vec::new();
    loop {
        let next = remaining.windows(4).position(|window| window == ESCAPE)?;
        let (plain, escape) = remaining.split_at_checked(next)?;
        payload.extend_from_slice(plain);
        let control = escape.strip_prefix(&ESCAPE)?;
        if let Some(rest) = control.strip_prefix(&ESCAPE) {
            payload.extend_from_slice(&ESCAPE);
            remaining = rest;
        } else {
            let [0x1a, padding @ 0..=3, lo, hi] = control else {
                return None;
            };
            let covered = wire.get(..wire.len().checked_sub(2)?)?;
            if crc16::State::<crc16::X_25>::calculate(covered) != u16::from_le_bytes([*lo, *hi])
                || !payload.len().is_multiple_of(4)
            {
                return None;
            }
            let length = payload.len().checked_sub(usize::from(*padding))?;
            if payload.get(length..)?.iter().any(|byte| *byte != 0) {
                return None;
            }
            payload.truncate(length);
            return Some(payload);
        }
    }
}

pub(super) fn inspect(
    data: &[u8],
    evidence: &mut m_bus_oms_rules::telegram::ApplicationProtocolEvidence,
) {
    let mut data = data;
    while let Some(rest) = data.strip_prefix(&[0x2f]) {
        data = rest;
    }
    let mut file_bounds = None;
    let file;
    if data.starts_with(&[0x1b, 0x1b, 0x1b, 0x1b, 1, 1, 1, 1]) {
        file = version_one_file(data);
        evidence.sml_transport = Some(file.is_some());
        let Some(bytes) = file.as_deref() else {
            return;
        };
        data = bytes;
        file_bounds = Some((None, None));
    } else if data.first() != Some(&0x76) {
        return;
    }
    let mut shapes = Some(true);
    let mut message_fields = true;
    let result = messages(data, |_, group, abort, tag, fields| {
        if let Some((first, last)) = &mut file_bounds {
            first.get_or_insert(tag);
            *last = Some(tag);
        }
        message_fields &= u8::try_from(group).is_ok()
            && u8::try_from(abort).is_ok()
            && u32::try_from(tag).is_ok();
        shapes = match (shapes, body_shape(tag, fields)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (None, _) | (_, None) => None,
            _ => Some(true),
        };
        Ok(())
    });
    evidence.sml_envelope = match result {
        Ok(()) => Some(message_fields),
        Err("invalid SML message") => None,
        Err(_) => Some(false),
    };
    if result.is_ok() {
        evidence.sml_file = file_bounds.map(|(first, last)| {
            matches!(
                (first, last),
                (Some(0x0100), Some(0x0200)) | (Some(0x0101), Some(0x0201))
            )
        });
        evidence.sml_body = shapes;
    }
}

pub(super) fn decode(data: &[u8]) -> Result<Value, &'static str> {
    let mut messages_out = Vec::new();
    messages(data, |transaction, group, abort, tag, fields| {
        let content = match (tag, fields) {
            (
                0x0701,
                [client, server, name, time, Field::List(entries), signature, gateway_time],
            ) => {
                let values = entries
                    .iter()
                    .map(list_entry)
                    .collect::<Option<Vec<_>>>()
                    .ok_or("invalid SML list entry")?;
                json!({"type":"get_list_response","server_id":server.json(),"client_id":client.json(),"list_name":name.json(),
                    "sensor_time":time.json(),"gateway_time":gateway_time.json(),"signature":signature.json(),"entries":values})
            }
            _ => return Err("unsupported SML message type"),
        };
        messages_out.push(json!({"transaction_id":hex_string(transaction),"group":group,"abort_on_error":abort,"body":content}));
        Ok(())
    })?;
    Ok(json!({"messages":messages_out}))
}
