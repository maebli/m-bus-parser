#![cfg(all(feature = "std", feature = "manufacturer-decoders"))]
// Fixture assertions deliberately index known record/field positions.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use m_bus_parser::{
    decode_bytes, decode_bytes_with_decoders,
    manufacturer::{
        Cursor, DecodeError, DecoderDescriptor, Field, Integer, ManufacturerDecoder, MeterInfo,
        Registry, Selector, Unit, UnitName,
    },
    output::render_decoded,
    DecodeOptions, OutputFormat,
};

struct Temperature;
impl ManufacturerDecoder for Temperature {
    fn descriptor(&self) -> &'static DecoderDescriptor {
        static DESCRIPTOR: DecoderDescriptor = DecoderDescriptor {
            name: "Synthetic temperature",
            source: "Synthetic integration test",
            selector: Selector {
                manufacturer: *b"ABC",
                versions: Some((1, 1)),
                device: None,
            },
        };
        &DESCRIPTOR
    }
    fn decode(
        &self,
        _: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Result<usize, DecodeError> {
        let mut cursor = Cursor::new(tail);
        emit(
            Field::unsigned("status", cursor.u8()?.into(), 0..1)
                .flags(&[(0, "active"), (1, "low_battery")]),
        );
        emit(
            Field::signed("temperature", cursor.i16_le()?, 1..3)
                .exponent(-2)
                .units(&[Unit {
                    name: UnitName::Celsius,
                    exponent: 1,
                }]),
        );
        Ok(cursor.position())
    }
}

fn application(tail: &[u8], dif: u8) -> Vec<u8> {
    // Long TPL: ID 12345678, manufacturer ABC, version 1, medium water.
    let mut app = vec![
        0x72, 0x78, 0x56, 0x34, 0x12, 0x43, 0x04, 1, 7, 0, 0, 0, 0, 1, 0x13, 5, dif,
    ]; // one standard volume record before the vendor tail
    app.extend_from_slice(tail);
    app
}
fn wired(tail: &[u8], dif: u8) -> Vec<u8> {
    let mut body = vec![8, 1];
    body.extend(application(tail, dif));
    let mut frame = vec![0x68, body.len() as u8, body.len() as u8, 0x68];
    let checksum = body.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    frame.extend(body);
    frame.extend([checksum, 0x16]);
    frame
}
fn wireless(app: &[u8]) -> Vec<u8> {
    // Deliberately different link identity XYZ; long TPL must take precedence.
    let mut frame = vec![0, 0x44, 0x3a, 0x63, 0x78, 0x56, 0x34, 0x12, 9, 7];
    frame.extend_from_slice(app);
    frame[0] = (frame.len() - 1) as u8;
    frame
}

#[test]
fn full_wired_frames_keep_standard_and_raw_records() {
    for dif in [0x0f, 0x1f] {
        let frame = wired(&[3, 6, 0xff, 0xaa], dif);
        let baseline = decode_bytes(&frame, &DecodeOptions::default()).unwrap();
        let decoded = decode_bytes_with_decoders(
            &frame,
            &DecodeOptions::default(),
            Registry::new(&[&Temperature]),
        )
        .unwrap();
        assert_eq!(decoded.records.len(), 2);
        assert_eq!(
            serde_json::to_value(&baseline.records[0]).unwrap(),
            serde_json::to_value(&decoded.records[0]).unwrap()
        );
        assert_eq!(
            decoded.raw.original_frame_hex,
            baseline.raw.original_frame_hex
        );
        let vendor = &decoded.records[1];
        assert_eq!(vendor.data_hex, baseline.records[1].data_hex);
        assert_eq!(vendor.header_hex, baseline.records[1].header_hex);
        let fields = vendor.manufacturer_fields.as_ref().unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].labels, ["active", "low_battery"]);
        assert_eq!(fields[1].value.value.as_ref().unwrap(), "-2.5");
        assert_eq!(fields[1].unit.as_deref(), Some("Cel"));
        assert_eq!(fields[1].data_hex, "06 FF");
        assert_eq!((fields[1].offset_start, fields[1].offset_end), (1, 3));
        assert_eq!(fields[2].name, "unparsed");
        assert_eq!(fields[2].value.value.as_ref().unwrap(), "AA");
        assert_eq!(decoded.decode_state, "complete");

        for width in [40, 90, 140] {
            let table = render_decoded(&decoded, OutputFormat::Table, Some(width)).unwrap();
            assert!(table.contains("temperature: -2.5"));
        }
        let csv = render_decoded(&decoded, OutputFormat::Csv, None).unwrap();
        assert_eq!(csv.lines().count(), 2, "preserve one frame per CSV row");
        assert!(csv.contains("record_1_manufacturer_field_1_value"));
        assert!(csv.contains("-2.5"));
        let json = render_decoded(&decoded, OutputFormat::Json, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["records"][1]["manufacturer_fields"][1]["value"]["value"],
            "-2.5"
        );
    }
}

#[test]
fn truncation_preserves_partial_fields_and_reports_error() {
    let decoded = decode_bytes_with_decoders(
        &wired(&[3, 6], 0x0f),
        &DecodeOptions::default(),
        Registry::new(&[&Temperature]),
    )
    .unwrap();
    let record = &decoded.records[1];
    assert_eq!(record.manufacturer_fields.as_ref().unwrap().len(), 1);
    assert_eq!(record.manufacturer_error.as_ref().unwrap().offset, 1);
    assert_eq!(record.data_hex, "03 06");
    assert_eq!(decoded.decode_state, "partial");
    assert!(decoded
        .diagnostics
        .iter()
        .any(|d| d.code == "manufacturer.partial"));
    assert!(render_decoded(&decoded, OutputFormat::Table, None)
        .unwrap()
        .contains("need 2 bytes, have 1"));
    assert!(render_decoded(&decoded, OutputFormat::Csv, None)
        .unwrap()
        .contains("record_1_manufacturer_error"));
}

#[test]
fn no_match_or_disabled_registry_preserves_output_shape() {
    let mut frame = wired(&[3, 0, 0], 0x0f);
    // Change long-TPL version to 2, then repair the wired checksum.
    frame[13] = 2;
    let checksum_pos = frame.len() - 2;
    frame[checksum_pos] = frame[4..checksum_pos]
        .iter()
        .fold(0u8, |sum, b| sum.wrapping_add(*b));
    let options = DecodeOptions::default();
    let baseline = serde_json::to_value(decode_bytes(&frame, &options).unwrap()).unwrap();
    for registry in [Registry::new(&[&Temperature]), Registry::only(&[])] {
        let decoded =
            serde_json::to_value(decode_bytes_with_decoders(&frame, &options, registry).unwrap())
                .unwrap();
        assert_eq!(baseline, decoded);
        assert!(decoded["records"][1].get("manufacturer_fields").is_none());
    }
}

#[test]
fn wireless_uses_long_tpl_identity_then_link_identity_for_short_tpl() {
    let options = DecodeOptions::default();
    let long = wireless(&application(&[3, 6, 0xff], 0x0f));
    let decoded =
        decode_bytes_with_decoders(&long, &options, Registry::new(&[&Temperature])).unwrap();
    assert_eq!(decoded.protocol, "wireless");
    assert!(decoded.records[1].manufacturer_fields.is_some());
    let mut short = wireless(&[0x7a, 0, 0, 0, 0, 0x0f, 3, 6, 0xff]);
    short[2..4].copy_from_slice(&[0x43, 0x04]);
    short[8] = 1;
    let decoded =
        decode_bytes_with_decoders(&short, &options, Registry::new(&[&Temperature])).unwrap();
    assert!(decoded.records[0].manufacturer_fields.is_some());
}

struct Exact;
impl ManufacturerDecoder for Exact {
    fn descriptor(&self) -> &'static DecoderDescriptor {
        Temperature.descriptor()
    }
    fn decode(
        &self,
        _: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Result<usize, DecodeError> {
        let mut cursor = Cursor::new(tail);
        emit(
            Field::unsigned("counter", cursor.u64_le()?, 0..8)
                .enumeration(&[(Integer::Unsigned(u64::MAX), "maximum")]),
        );
        Ok(cursor.position())
    }
}
#[test]
fn integers_larger_than_f64_precision_remain_exact() {
    let decoded = decode_bytes_with_decoders(
        &wired(&[0xff; 8], 0x0f),
        &DecodeOptions::default(),
        Registry::new(&[&Exact, &Temperature]),
    )
    .unwrap();
    let fields = decoded.records[1].manufacturer_fields.as_ref().unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].value.value.as_ref().unwrap(),
        "18446744073709551615"
    );
    assert_eq!(fields[0].labels, ["maximum"]);
}
