use crate::{
    DecodeError, DecoderDescriptor, ErrorKind, Field, Integer, Labels, ManufacturerDecoder,
    MeterInfo, Value,
};

/// Ordered borrowed decoder list. The first matching selector wins.
#[derive(Clone, Copy, Default)]
pub struct Registry<'a> {
    decoders: &'a [&'a dyn ManufacturerDecoder],
}
#[derive(Debug)]
pub struct DecodeSummary {
    pub decoder: DecoderDescriptor,
    pub consumed: usize,
}
impl<'a> Registry<'a> {
    pub const fn new(decoders: &'a [&'a dyn ManufacturerDecoder]) -> Self {
        Self { decoders }
    }
    pub fn find(&self, meter: &MeterInfo) -> Option<&'a dyn ManufacturerDecoder> {
        self.decoders
            .iter()
            .copied()
            .find(|decoder| decoder.descriptor().selector.matches(meter))
    }
    /// `None` means no match. Successful dispatch emits any leftover bytes as `unparsed`.
    /// Errors retain preceding valid fields; there is never a retry with another decoder.
    pub fn decode(
        &self,
        meter: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Option<Result<DecodeSummary, DecodeError>> {
        let decoder = self.find(meter)?;
        let mut field_error = None;
        let mut greatest_end = 0;
        let result = decoder.decode(meter, tail, &mut |field| {
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
            Ok(DecodeSummary {
                decoder: decoder.descriptor(),
                consumed,
            })
        }))
    }
}
