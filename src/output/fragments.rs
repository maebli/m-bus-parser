use super::*;

#[derive(Debug, PartialEq)]
enum Link {
    Wired(wired::Address),
    Wireless(wireless::ManufacturerId, Option<[u8; 8]>),
}

fn invalid(message: &str) -> OutputError {
    OutputError::Rendering {
        code: "fragment.invalid",
        message: message.to_owned(),
    }
}

pub(super) fn is_fragment(mut data: &[u8]) -> bool {
    data = match data {
        [0x8c, _, _, tail @ ..] => tail,
        [0x8e, _, _, tail @ ..] => tail.get(8..).unwrap_or_default(),
        data => data,
    };
    let [0x90, size, id, flags, rest @ ..] = data else {
        return false;
    };
    let Some(header_len) = size.checked_sub(2).map(usize::from) else {
        return false;
    };
    if *id == 0 || flags & 0x81 != 0 || header_len > rest.len() {
        return false;
    }
    if header_len < rest.len() {
        return true;
    }
    if *id == 1 || flags & 0x64 != 0x04 {
        return false;
    }
    let other_fields = 2 * usize::from(flags & 0x02 != 0)
        + 4 * usize::from(flags & 0x08 != 0)
        + 2 * usize::from(flags & 0x10 != 0);
    matches!(header_len.checked_sub(other_fields), Some(8 | 12 | 16))
}

fn link_payload<'a>(
    frame: &'a [u8],
    scratch: &'a mut [u8],
) -> Result<(Link, bool, &'a [u8]), OutputError> {
    let (link, function, data) = if let Ok(wired::WiredFrame::LongFrame {
        address,
        function,
        data,
    }) = wired::WiredFrame::try_from(frame)
    {
        (Link::Wired(address), Some(function), data)
    } else {
        let bytes = wireless::strip_format_a_crcs(frame, scratch).unwrap_or(frame);
        let frame = wireless::WirelessFrame::try_from(bytes)
            .map_err(|_| invalid("invalid fragment link frame"))?;
        let (receiver, data) = match frame.data {
            [0x8e, _, _, tail @ ..] => {
                let (receiver, data) = tail
                    .split_first_chunk::<8>()
                    .ok_or_else(|| invalid("short ELL receiver address"))?;
                (Some(*receiver), data)
            }
            [0x8c, _, _, data @ ..] => (None, data),
            data => (None, data),
        };
        (
            Link::Wireless(frame.manufacturer_id, receiver),
            frame.function,
            data,
        )
    };
    let from_meter = match function {
        Some(crate::Function::RspUd { .. }) => true,
        Some(crate::Function::SndUd { .. } | crate::Function::SndUd2) => false,
        _ => return Err(invalid("unexpected fragment link function")),
    };
    Ok((link, from_meter, data))
}

#[derive(Default)]
struct Fields {
    control: Option<u8>,
    key: Option<[u8; 2]>,
    counter: Option<[u8; 4]>,
    length: Option<u16>,
    mac: Option<Vec<u8>>,
}

fn take<const N: usize>(data: &mut &[u8]) -> Result<[u8; N], OutputError> {
    let (field, rest) = data
        .split_first_chunk::<N>()
        .ok_or_else(|| invalid("short AFL field"))?;
    *data = rest;
    Ok(*field)
}

fn remember<T: PartialEq>(slot: &mut Option<T>, value: T) -> Result<(), OutputError> {
    if slot.as_ref().is_some_and(|previous| *previous != value) {
        return Err(invalid("inconsistent AFL field across fragments"));
    }
    *slot = Some(value);
    Ok(())
}

impl Fields {
    fn read<'a>(
        &mut self,
        data: &'a [u8],
        index: usize,
        last: bool,
    ) -> Result<&'a [u8], OutputError> {
        let [0x90, size, rest @ ..] = data else {
            return Err(invalid("missing AFL header"));
        };
        let (mut fields, transport) = rest
            .split_at_checked(usize::from(*size))
            .ok_or_else(|| invalid("short AFL header"))?;
        let [id, flags] = take(&mut fields)?;
        if usize::from(id) != index + 1
            || (flags & 0x40 != 0) == last
            || flags & 0x81 != 0
            || (transport.is_empty() && !(last && flags & 0x04 != 0))
        {
            return Err(invalid("invalid fragment order, length or control"));
        }
        if flags & 0x20 != 0 {
            if index != 0 {
                return Err(invalid("message control outside first fragment"));
            }
            self.control = Some(take::<1>(&mut fields)?[0]);
        }
        if flags & 0x02 != 0 {
            remember(&mut self.key, take(&mut fields)?)?;
        }
        if flags & 0x08 != 0 {
            remember(&mut self.counter, take(&mut fields)?)?;
        }
        if flags & 0x04 != 0 {
            if !last {
                return Err(invalid("AFL MAC outside final fragment"));
            }
            let size = match self.control.map(|v| v & 15) {
                Some(5) => 8,
                Some(6) => 12,
                Some(7) => 16,
                _ => return Err(invalid("unsupported AFL authentication type")),
            };
            let (mac, rest) = fields
                .split_at_checked(size)
                .ok_or_else(|| invalid("short fragment MAC"))?;
            self.mac = Some(mac.to_vec());
            fields = rest;
        }
        if flags & 0x10 != 0 {
            remember(&mut self.length, u16::from_le_bytes(take(&mut fields)?))?;
        }
        if !fields.is_empty() {
            return Err(invalid("unexpected AFL fields"));
        }
        Ok(transport)
    }

    fn application(self, transport: Vec<u8>) -> Result<Vec<u8>, OutputError> {
        let control = self.control.unwrap_or(0);
        if control & 0x80 != 0
            || (control & 0x10 != 0) != self.key.is_some()
            || (control & 0x20 != 0) != self.counter.is_some()
            || (control & 0x40 != 0) != self.length.is_some()
            || self
                .length
                .is_some_and(|length| usize::from(length) != transport.len())
        {
            return Err(invalid("inconsistent AFL message fields or length"));
        }
        if matches!(transport.first(), Some(0x8c..=0x90)) {
            return Err(invalid("unexpected nested link or fragmentation header"));
        }
        if control & 15 == 0 {
            if self.key.is_some() || self.counter.is_some() || self.mac.is_some() {
                return Err(invalid("unexpected fields on an unauthenticated AFL"));
            }
            return Ok(transport);
        }
        let counter = self
            .counter
            .ok_or_else(|| invalid("missing message counter"))?;
        let mac = self.mac.ok_or_else(|| invalid("missing final MAC"))?;
        let flags =
            0x2c | (u8::from(self.key.is_some()) << 1) | (u8::from(self.length.is_some()) << 4);
        let mut header = vec![0, flags, control];
        if let Some(key) = self.key {
            header.extend(key);
        }
        header.extend(counter);
        header.extend(mac);
        if let Some(length) = self.length {
            header.extend(length.to_le_bytes());
        }
        let mut message = vec![0x90, header.len() as u8];
        message.extend(header);
        message.extend(transport);
        Ok(message)
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FragmentOutput {
    Single(Box<DecodedOutput>),
    Multiple(Vec<DecodedOutput>),
}

pub fn decode_fragment_frames(
    packets: &[&[u8]],
    options: &DecodeOptions,
) -> Result<FragmentOutput, OutputError> {
    if let [stream] = packets {
        if wired_frame_size(stream).is_some_and(|size| stream.len() > size) {
            return decode_wired_stream(stream, options);
        }
    }
    if !matches!(packets.first(), Some([1, ..])) {
        return decode_afl_fragments(packets, options)
            .map(|frame| FragmentOutput::Single(Box::new(frame)));
    }
    let total = u8::try_from(packets.len()).map_err(|_| invalid("too many ACRIOS packets"))?;
    let mut joined = Vec::new();
    for (index, packet) in packets.iter().enumerate() {
        let [sequence, count, bytes @ ..] = packet else {
            return Err(invalid("missing ACRIOS packet header"));
        };
        if usize::from(*sequence) != index + 1 || *count != total || bytes.is_empty() {
            return Err(invalid("inconsistent ACRIOS packet order, count or length"));
        }
        joined.extend_from_slice(bytes);
    }
    let mut result = decode_wired_stream(acrios_meter_data(&joined), options)?;
    if let FragmentOutput::Single(frame) = &mut result {
        frame.raw.fragment_frames_hex = packets.iter().map(|packet| hex_string(packet)).collect();
    }
    Ok(result)
}

fn wired_frame_size(payload: &[u8]) -> Option<usize> {
    match payload {
        [0x68, length, repeat, 0x68, ..] if length == repeat => Some(usize::from(*length) + 6),
        [0x10, ..] => Some(5),
        [0xe5, ..] => Some(1),
        _ => None,
    }
}

fn decode_wired_stream(
    mut payload: &[u8],
    options: &DecodeOptions,
) -> Result<FragmentOutput, OutputError> {
    let mut decoded = Vec::new();
    while !payload.is_empty() {
        let size =
            wired_frame_size(payload).ok_or_else(|| invalid("invalid wired frame header"))?;
        let (frame, rest) = payload
            .split_at_checked(size)
            .ok_or_else(|| invalid("truncated wired frame in ACRIOS data"))?;
        wired::WiredFrame::try_from(frame)
            .map_err(|_| invalid("invalid wired frame in ACRIOS data"))?;
        decoded.push(decode_bytes(frame, options)?);
        payload = rest;
    }
    if decoded.len() == 1 {
        let frame = decoded
            .pop()
            .ok_or_else(|| invalid("missing wired frame"))?;
        Ok(FragmentOutput::Single(Box::new(frame)))
    } else {
        Ok(FragmentOutput::Multiple(decoded))
    }
}

fn acrios_meter_data(bytes: &[u8]) -> &[u8] {
    match bytes {
        [0x68, length, repeat, 0x68, ..] if length == repeat => bytes,
        [_, inner @ ..] if matches!(inner, [0x68, length, repeat, 0x68, ..] if length == repeat) => {
            inner
        }
        _ => bytes,
    }
}

pub(super) fn singleton_payload(packet: &[u8]) -> Result<Option<&[u8]>, OutputError> {
    let [1, count, bytes @ ..] = packet else {
        return Ok(None);
    };
    let payload = acrios_meter_data(bytes);
    let [0x68, length, repeat, 0x68, ..] = payload else {
        return Ok(None);
    };
    if *count > 1 && length == repeat {
        return Err(invalid(
            "supply all numbered ACRIOS packets to the fragment decoder",
        ));
    }
    if *count != 1 || length != repeat {
        return Err(invalid("invalid ACRIOS packet count or frame length"));
    }
    if payload.len() > usize::from(*length) + 6 {
        return Err(invalid(
            "use the fragment decoder for an ACRIOS packet containing multiple frames",
        ));
    }
    wired::WiredFrame::try_from(payload)
        .map_err(|_| invalid("invalid wired frame in ACRIOS data"))?;
    Ok(Some(payload))
}

pub fn decode_fragments(
    frames: &[&[u8]],
    options: &DecodeOptions,
) -> Result<DecodedOutput, OutputError> {
    match decode_fragment_frames(frames, options)? {
        FragmentOutput::Single(frame) => Ok(*frame),
        FragmentOutput::Multiple(_) => Err(invalid(
            "use decode_fragment_frames for multiple meter frames",
        )),
    }
}

fn decode_afl_fragments(
    frames: &[&[u8]],
    options: &DecodeOptions,
) -> Result<DecodedOutput, OutputError> {
    if !(2..=255).contains(&frames.len()) {
        return Err(invalid("expected 2 to 255 ordered fragments"));
    }
    let mut origin = None;
    let mut fields = Fields::default();
    let mut transport = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let mut scratch = [0; 290];
        let (link, direction, data) = link_payload(frame, &mut scratch)?;
        remember(&mut origin, (link, direction))?;
        let part = fields.read(data, index, index + 1 == frames.len())?;
        if transport.len() + part.len() > 16384 {
            return Err(invalid("fragmented message exceeds 16 KiB"));
        }
        transport.extend_from_slice(part);
    }
    let application = fields.application(transport)?;
    let block =
        user_data::parse_application_layer(&application).map_err(|e| invalid(&e.to_string()))?;
    let mode7 = block.transport_payload().is_some_and(|(header, _)| {
        header.configuration_field.security_mode() == SecurityMode::AesCbc128IvZero
    });
    if mode7 != block.authentication().is_some() {
        return Err(invalid(
            "inconsistent AFL and transport authentication mode",
        ));
    }
    let (protocol, wireless_id, address) = match origin {
        Some((Link::Wireless(ref id, _), _)) => ("wireless", Some(id), None),
        Some((Link::Wired(ref address), _)) => ("wired", None, Some(address.to_string())),
        None => return Err(invalid("missing fragment origin")),
    };
    let mut buffer = vec![0; application.len()];
    let security = prepare_security(
        Some(&block),
        wireless_id,
        options.key.as_ref(),
        None,
        &mut buffer,
    )?;
    let records = if let Some(plain) = security.decrypted_payload.as_deref() {
        decrypted_records(Some(&block), plain)
    } else {
        block.data_records()
    };
    let mut decoded = build_output(
        protocol,
        FrameOutput {
            kind: "reassembled".into(),
            function: None,
            address,
            control_field: None,
        },
        &application,
        &application,
        false,
        Some(&application),
        Some(&block),
        records.as_ref(),
        None,
        wireless_id,
        security.clone(),
        options.include_enrichment,
    );
    decoded.raw.fragment_frames_hex = frames.iter().map(|frame| hex_string(frame)).collect();
    Ok(decoded)
}
