//! Allocation-free known-answer fixtures for decoder authors.
//!
//! These assertion helpers intentionally panic on test failures; use them in tests only.
use crate::{DecodeError, Field, ManufacturerDecoder, MeterInfo};

pub struct Fixture<'a> {
    pub name: &'a str,
    pub meter: MeterInfo,
    pub tail: &'a [u8],
    pub fields: &'a [Field<'a>],
    pub result: Result<usize, DecodeError>,
}

/// Compare every emitted field (including ranges, units, exponents and labels),
/// the exact consumed length/error, and require at least one successful reading.
pub fn assert_fixtures(decoder: &dyn ManufacturerDecoder, fixtures: &[Fixture<'_>]) {
    assert!(
        fixtures
            .iter()
            .any(|f| f.result.is_ok() && !f.fields.is_empty()),
        "include a successful fixture with expected fields"
    );
    for fixture in fixtures {
        assert!(
            decoder.descriptor().selector.matches(&fixture.meter),
            "{}: selector does not match",
            fixture.name
        );
        let mut index = 0;
        let result = decoder.decode(&fixture.meter, fixture.tail, &mut |field| {
            assert_eq!(
                Some(&field),
                fixture.fields.get(index),
                "{}: field {index}",
                fixture.name
            );
            index += 1;
        });
        assert_eq!(index, fixture.fields.len(), "{}: field count", fixture.name);
        assert_eq!(
            result, fixture.result,
            "{}: consumed length/error",
            fixture.name
        );
    }
}
