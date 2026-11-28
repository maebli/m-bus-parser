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

pub(super) fn decode(mut data: &[u8]) -> Result<Value, &'static str> {
    let mut messages = Vec::new();
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
        let content = match (*tag, fields.as_slice()) {
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
        messages.push(json!({"transaction_id":hex_string(transaction),"group":group,"abort_on_error":abort,"body":content}));
        while let Some(rest) = data.strip_prefix(&[0x2f]) {
            data = rest;
        }
    }
    if messages.is_empty() {
        return Err("empty SML payload");
    }
    Ok(json!({"messages":messages}))
}
