use crate::{
    DecodeError, DecoderDescriptor, ErrorKind, Field, Integer, Labels, ManufacturerDecoder,
    MeterInfo, Value, BUILTIN,
};

/// Ordered borrowed decoder lists; custom decoders take precedence over built-ins.
#[derive(Clone, Copy)]
pub struct Registry<'a> {
    custom: &'a [&'a dyn ManufacturerDecoder],
    builtins: &'a [&'a dyn ManufacturerDecoder],
}
impl core::fmt::Debug for Registry<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Registry")
            .field("custom_count", &self.custom.len())
            .field("builtin_count", &self.builtins.len())
            .finish()
    }
}
impl Default for Registry<'_> {
    fn default() -> Self {
        Self::new(&[])
    }
}
#[derive(Debug)]
pub struct DecodeSummary {
    pub decoder: &'static DecoderDescriptor,
    pub consumed: usize,
}
impl<'a> Registry<'a> {
    pub const fn new(custom: &'a [&'a dyn ManufacturerDecoder]) -> Self {
        Self {
            custom,
            builtins: BUILTIN,
        }
    }
    /// Use exactly this list (including an empty list to disable decoding).
    pub const fn only(decoders: &'a [&'a dyn ManufacturerDecoder]) -> Self {
        Self {
            custom: decoders,
            builtins: &[],
        }
    }
    pub fn find(&self, meter: &MeterInfo) -> Option<&'a dyn ManufacturerDecoder> {
        self.custom
            .iter()
            .chain(self.builtins)
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
