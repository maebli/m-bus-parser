mod file;
mod schema;

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

fn transport_payload(wire: &[u8], mut remaining: &[u8]) -> Option<Vec<u8>> {
    const ESCAPE: [u8; 4] = [0x1b; 4];
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

pub(super) fn version_two_block(
    wire: &[u8],
) -> Option<(m_bus_oms_rules::telegram::SmlBlock, Vec<u8>)> {
    let [0x1b, 0x1b, 0x1b, 0x1b, 2, control, file, 1, rest @ ..] = wire else {
        return None;
    };
    let block = control & 0x3f;
    if block == 0 && control & 0x40 != 0 {
        return None;
    }
    let (negotiation, body) = if block == 0 {
        if let [0x1b, 0x1b, 0x1b, 0x1b, 3, 0, hi, lo, 0x1b, 0x1b, 0x1b, 0x1b, 4, 0, msb, lsb, body @ ..] =
            rest
        {
            let timeout = u16::from_be_bytes([*hi, *lo]);
            let size = u16::from_be_bytes([*msb, *lsb]);
            if timeout == 0 || size == 0 {
                return None;
            }
            (Some((timeout, size)), body)
        } else if *control == 0x80 {
            (None, rest)
        } else {
            return None;
        }
    } else {
        (None, rest)
    };
    let decoded = transport_payload(wire, body)?;
    if (block == 0 || control & 0x80 != 0) && !decoded.is_empty() {
        return None;
    }
    Some((
        m_bus_oms_rules::telegram::SmlBlock {
            control: *control,
            file: *file,
            negotiation,
            payload_len: decoded.len(),
            wire_len: wire.len(),
        },
        decoded,
    ))
}

pub(super) fn decode(
    data: &[u8],
    parsed: &mut ProtocolData,
) -> Option<Result<Value, &'static str>> {
    let data = data.get(data.iter().position(|b| *b != 0x2f)?..)?;
    if data.starts_with(&[0x1b, 0x1b, 0x1b, 0x1b, 2]) {
        let block = version_two_block(data);
        parsed.evidence.sml_v2 = Some(block.as_ref().map(|(header, _)| *header).ok_or(()));
        return Some(
            block
                .map(|(header, bytes)| {
                    parsed.block_data = bytes;
                    json!({"transport_version":2, "control":header.control, "file":header.file,
                "negotiation":header.negotiation,"payload_hex":hex_string(&parsed.block_data)})
                })
                .ok_or("invalid SML version 2 block"),
        );
    }
    let file;
    let stream = data.strip_prefix(&[0x1b, 0x1b, 0x1b, 0x1b, 1, 1, 1, 1]);
    let complete_file = stream.is_some();
    let data = if let Some(stream) = stream {
        file = transport_payload(data, stream);
        parsed.evidence.sml_transport = Some(file.is_some());
        let Some(bytes) = file.as_deref() else {
            return Some(Err("invalid SML transport frame"));
        };
        bytes
    } else if data.first() == Some(&0x76) {
        data
    } else {
        return None;
    };
    let mut output = Vec::new();
    let mut render_error = None;
    let result = read_messages(
        data,
        complete_file,
        &mut parsed.evidence,
        |transaction, group, abort, tag, fields| match message_output(
            transaction,
            group,
            abort,
            tag,
            fields,
        ) {
            Ok(message) => output.push(message),
            Err(error) => render_error = Some(error),
        },
    );
    Some(result.and_then(|()| match render_error {
        Some(error) => Err(error),
        None => Ok(json!({"messages":output})),
    }))
}

pub(super) fn inspect_messages(
    data: &[u8],
    complete_file: bool,
    evidence: &mut m_bus_oms_rules::telegram::ApplicationProtocolEvidence,
) {
    let _ = read_messages(data, complete_file, evidence, |_, _, _, _, _| {});
}

fn read_messages(
    data: &[u8],
    complete_file: bool,
    evidence: &mut m_bus_oms_rules::telegram::ApplicationProtocolEvidence,
    mut visit: impl FnMut(&[u8], u64, u64, u64, &[Field<'_>]),
) -> Result<(), &'static str> {
    let mut file_messages = complete_file.then(Vec::new);
    let mut shapes = Some(true);
    let mut message_fields = true;
    let result = messages(data, |transaction, group, abort, tag, fields| {
        visit(transaction, group, abort, tag, fields);
        if let Some(messages) = &mut file_messages {
            messages.push(file::Message::new(tag, group, fields));
        }
        message_fields &= u8::try_from(group).is_ok()
            && matches!(abort, 0 | 1 | 2 | 255)
            && u32::try_from(tag).is_ok();
        shapes = match (shapes, schema::body_shape(tag, fields)) {
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
        if let Some(messages) = file_messages {
            file::check(&messages, evidence);
        }
        evidence.sml_body = shapes;
    }
    result
}

fn message_output(
    transaction: &[u8],
    group: u64,
    abort: u64,
    tag: u64,
    fields: &[Field<'_>],
) -> Result<Value, &'static str> {
    let content = match (tag, fields) {
        (0x0701, [client, server, name, time, Field::List(entries), signature, gateway_time]) => {
            let values = entries
                .iter()
                .map(list_entry)
                .collect::<Option<Vec<_>>>()
                .ok_or("invalid SML list entry")?;
            json!({"type":"get_list_response","server_id":server.json(),"client_id":client.json(),"list_name":name.json(),
                    "sensor_time":time.json(),"gateway_time":gateway_time.json(),"signature":signature.json(),"entries":values})
        }
        _ => {
            json!({"message_type":tag,"fields":fields.iter().map(Field::json).collect::<Vec<_>>()})
        }
    };
    Ok(
        json!({"transaction_id":hex_string(transaction),"group":group,"abort_on_error":abort,"body":content}),
    )
}
