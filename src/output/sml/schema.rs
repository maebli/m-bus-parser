use super::Field;
use Shape::*;

#[derive(Clone, Copy)]
enum Shape {
    Octets,
    Boolean,
    Uint8,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int64,
    Optional(&'static Shape),
    Items(&'static Shape),
    Nonempty(&'static Shape),
    Sequence(&'static [Shape]),
    Time,
    Tree,
    Parameter,
    Value,
    CosemValue,
    CosemContent,
}

const PATH: Shape = Nonempty(&Octets);
const ENTRY: Shape = Sequence(&[
    Octets,
    Optional(&Uint64),
    Optional(&Time),
    Optional(&Uint8),
    Optional(&Int8),
    Value,
    Octets,
]);
const PERIOD: Shape = Sequence(&[Octets, Uint8, Int8, Value, Octets]);
const HEADER: Shape = Sequence(&[Octets, Uint8, Int8]);
const VALUES: Shape = Items(&Sequence(&[Value, Octets]));
const PROFILE: Shape = Sequence(&[Time, Uint64, VALUES, Octets]);
const TUPLE: Shape = Sequence(&[
    Octets, Time, Uint64, Uint8, Int8, Int64, Uint8, Int8, Int64, Uint8, Int8, Int64, Octets,
    Uint8, Int8, Int64, Uint8, Int8, Int64, Uint8, Int8, Int64, Octets,
]);
const ATTRIBUTE_DESC: Shape = Sequence(&[Int16, Optional(&Sequence(&[Uint8, CosemValue]))]);
const ATTRIBUTES: Shape = Items(&Sequence(&[ATTRIBUTE_DESC, CosemContent]));

fn sequence(fields: &[Field<'_>], schema: &[Shape]) -> bool {
    fields.len() == schema.len()
        && fields
            .iter()
            .zip(schema)
            .all(|(field, shape)| shape.accepts(field))
}

impl Shape {
    fn accepts(self, field: &Field<'_>) -> bool {
        match (self, field) {
            (Octets, Field::Bytes(_))
            | (Boolean, Field::Bool(_))
            | (Uint64, Field::Unsigned(_))
            | (Int64, Field::Signed(_)) => true,
            (Uint8, Field::Unsigned(value)) => u8::try_from(*value).is_ok(),
            (Uint32, Field::Unsigned(value)) => u32::try_from(*value).is_ok(),
            (Int8, Field::Signed(value)) => i8::try_from(*value).is_ok(),
            (Int16, Field::Signed(value)) => i16::try_from(*value).is_ok(),
            (Optional(_), Field::Bytes([])) => true,
            (Optional(shape), value) => shape.accepts(value),
            (Items(shape) | Nonempty(shape), Field::List(items)) => {
                (!matches!(self, Nonempty(_)) || !items.is_empty())
                    && items.iter().all(|item| shape.accepts(item))
            }
            (Sequence(schema), Field::List(items)) => sequence(items, schema),
            (Tree, Field::List(items)) => sequence(
                items,
                &[Octets, Optional(&Parameter), Optional(&Nonempty(&Tree))],
            ),
            (Time | Parameter | CosemContent, Field::List(items)) => {
                let [Field::Unsigned(tag), value] = items.as_slice() else {
                    return false;
                };
                match (self, tag) {
                    (Time, 1 | 2) => Uint32.accepts(value),
                    (Time, 3) => Sequence(&[Uint32, Int16, Int16]).accepts(value),
                    (Parameter, 1) => Value.accepts(value),
                    (Parameter, 2) => PERIOD.accepts(value),
                    (Parameter, 3) => TUPLE.accepts(value),
                    (Parameter, 4) => Time.accepts(value),
                    (Parameter, 5) => ENTRY.accepts(value),
                    (CosemContent, 1) => CosemValue.accepts(value),
                    (CosemContent, 2) => Uint8.accepts(value),
                    _ => false,
                }
            }
            (Value, Field::List(items)) => {
                matches!(items.as_slice(), [Field::Unsigned(1), time] if Time.accepts(time))
            }
            (CosemValue, Field::List(items)) => items.iter().all(|item| CosemValue.accepts(item)),
            (
                Value | CosemValue,
                Field::Bool(_) | Field::Bytes(_) | Field::Unsigned(_) | Field::Signed(_),
            ) => true,
            _ => false,
        }
    }
}

fn ordered_profile(headers: &[Field<'_>], periods: &[Field<'_>]) -> bool {
    let mut previous = None;
    for period in periods {
        let Field::List(fields) = period else {
            return false;
        };
        let [Field::List(time), _, Field::List(values), _] = fields.as_slice() else {
            return false;
        };
        if headers.len() != values.len() {
            return false;
        }
        if let [Field::Unsigned(kind @ (1 | 2)), Field::Unsigned(current)] = time.as_slice() {
            if previous.is_some_and(|(k, prior)| k == *kind && prior > *current) {
                return false;
            }
            previous = Some((*kind, *current));
        } else {
            previous = None;
        }
    }
    true
}

pub(super) fn body_shape(tag: u64, fields: &[Field<'_>]) -> Option<bool> {
    let schema: &[Shape] = match tag {
        0x0100 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Optional(&Uint8),
        ],
        0x0101 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Optional(&Time),
            Optional(&Uint8),
        ],
        0x0200 | 0x0201 => &[Octets],
        0x0300 | 0x0400 => &[
            Octets,
            Octets,
            Octets,
            Optional(&Boolean),
            Optional(&Time),
            Optional(&Time),
            PATH,
            Optional(&Nonempty(&Octets)),
            Optional(&Tree),
        ],
        0x0301 => &[
            Octets,
            Time,
            Uint32,
            PATH,
            Items(&HEADER),
            Items(&PROFILE),
            Octets,
            Octets,
        ],
        0x0401 => &[
            Octets,
            Time,
            Uint32,
            PATH,
            Time,
            Uint64,
            Items(&PERIOD),
            Octets,
            Octets,
        ],
        0x0500 => &[Octets, Octets, Octets, PATH, Octets],
        0x0501 => &[Octets, PATH, Tree],
        0x0600 => &[Octets, Octets, Octets, PATH, Tree],
        0x0700 => &[Octets, Octets, Octets, Octets, Octets],
        0x0701 => &[
            Octets,
            Octets,
            Octets,
            Optional(&Time),
            Items(&ENTRY),
            Octets,
            Optional(&Time),
        ],
        0xff01 => &[Octets, Octets, Octets, Optional(&Tree)],
        0x0800 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Int16,
            Int16,
            Optional(&Items(&ATTRIBUTE_DESC)),
        ],
        0x0801 => &[Octets, Octets, Octets, Int16, Int16, ATTRIBUTES],
        0x0900 => &[
            Octets, Octets, Octets, Octets, Octets, Int16, Int16, ATTRIBUTES,
        ],
        0x0901 | 0x0a01 => &[Octets, Octets, Octets, Int16, Int16, Optional(&ATTRIBUTES)],
        0x0a00 => &[
            Octets,
            Octets,
            Octets,
            Octets,
            Octets,
            Int16,
            Int16,
            Uint8,
            Optional(&CosemValue),
        ],
        _ => return None,
    };
    let valid = sequence(fields, schema);
    if let (true, 0x0301, [_, _, _, _, Field::List(headers), Field::List(periods), _, _]) =
        (valid, tag, fields)
    {
        return Some(ordered_profile(headers, periods));
    }
    Some(valid)
}
