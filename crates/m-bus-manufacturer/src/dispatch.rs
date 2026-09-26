use crate::{DecodeError, Decoder, ErrorKind, Field, Integer, Labels, MeterInfo, Value};

#[derive(Debug)]
pub struct DecodeSummary<'a> {
    pub decoder: &'a Decoder,
    pub consumed: usize,
}

/// Run the first matching entry. `None` means no match; errors never retry another entry.
/// Successful dispatch emits leftover bytes as `unparsed`. Errors retain earlier fields.
pub fn decode<'a>(
    decoders: &'a [Decoder],
    meter: &MeterInfo,
    tail: &[u8],
    emit: &mut dyn FnMut(Field<'_>),
) -> Option<Result<DecodeSummary<'a>, DecodeError>> {
    let decoder = decoders.iter().find(|decoder| decoder.matches(meter))?;
    let mut field_error = None;
    let mut greatest_end = 0;
    let result = (decoder.decode)(meter, tail, &mut |field| {
        if field_error.is_some() {
            return;
        }
        let valid_labels = match field.labels {
            Labels::None => true,
            Labels::Flags(labels) => {
                matches!(field.value, Value::Integer(Integer::Unsigned(_)))
                    && labels.iter().all(|(bit, _)| *bit < 64)
            }
            Labels::Enum(labels) => match field.value {
                Value::Integer(Integer::Unsigned(_)) => labels
                    .iter()
                    .all(|(key, _)| matches!(key, Integer::Unsigned(_))),
                Value::Integer(Integer::Signed(_)) => labels
                    .iter()
                    .all(|(key, _)| matches!(key, Integer::Signed(_))),
                _ => false,
            },
        };
        if field.name.is_empty()
            || field.range.start > field.range.end
            || field.range.end > tail.len()
            || !valid_labels
        {
            field_error = Some(DecodeError::new(field.range.start, ErrorKind::InvalidField));
            return;
        }
        greatest_end = greatest_end.max(field.range.end);
        emit(field);
    });
    if let Some(error) = field_error {
        return Some(Err(error));
    }
    Some(result.and_then(|consumed| {
        if consumed > tail.len() || consumed < greatest_end {
            return Err(DecodeError::new(consumed, ErrorKind::InvalidConsumedLength));
        }
        if let Some(rest) = tail.get(consumed..).filter(|rest| !rest.is_empty()) {
            emit(Field::bytes("unparsed", rest, consumed..tail.len()));
        }
        Ok(DecodeSummary { decoder, consumed })
    }))
}
