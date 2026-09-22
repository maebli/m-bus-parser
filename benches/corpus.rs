//! Corpus benchmarks for the allocation-free parser; see benches/README.md.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use m_bus_parser::{mbus_data::MbusData, MbusError, WiredFrame, WirelessFrame};
use std::{fs, hint::black_box, path::PathBuf, time::Duration};

#[derive(Clone, Copy)]
enum Protocol {
    Wired,
    Wireless,
}

struct Fixture {
    name: String,
    bytes: Vec<u8>,
    protocol: Protocol,
    expected_records: Option<usize>,
    #[cfg(feature = "std")]
    xml: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Work {
    records: usize,
    record_errors: usize,
    labels: usize,
    units: usize,
    has_records: bool,
}

/// Force every record and its lazy semantic metadata to be evaluated. Text and
/// manufacturer-specific values retain their borrowed representation (no formatting).
fn consume<F>(parsed: MbusData<'_, F>) -> Result<Work, MbusError> {
    if let Some(error) = parsed.application_error {
        return Err(error.into());
    }
    let mut work = Work {
        has_records: parsed.data_records.is_some(),
        ..Work::default()
    };
    black_box(parsed.frame);
    black_box(parsed.user_data);
    if let Some(records) = parsed.data_records {
        for record in records {
            match record {
                Ok(record) => {
                    if let Some(info) = record.value_information() {
                        for label in info.labels() {
                            black_box(label);
                            work.labels += 1;
                        }
                        for unit in info.units() {
                            black_box(unit);
                            work.units += 1;
                        }
                        black_box((info.decimal_scale_exponent, info.decimal_offset_exponent));
                    }
                    black_box(record);
                    work.records += 1;
                }
                Err(error) => {
                    black_box(error);
                    work.record_errors += 1;
                }
            }
        }
    }
    Ok(work)
}

impl Fixture {
    fn frame(&self) -> Result<(), MbusError> {
        match self.protocol {
            Protocol::Wired => {
                black_box(WiredFrame::try_from(black_box(self.bytes.as_slice()))?);
            }
            Protocol::Wireless => {
                black_box(WirelessFrame::try_from(black_box(self.bytes.as_slice()))?);
            }
        }
        Ok(())
    }

    fn decode(&self) -> Result<Work, MbusError> {
        let bytes = black_box(self.bytes.as_slice());
        match self.protocol {
            Protocol::Wired => consume(MbusData::<WiredFrame>::try_from(bytes)?),
            Protocol::Wireless => consume(MbusData::<WirelessFrame>::try_from(bytes)?),
        }
    }
}

fn decode_hex(input: &str) -> Vec<u8> {
    hex::decode(input.split_whitespace().collect::<String>()).expect("valid fixture hex")
}

fn wired_fixtures() -> Vec<Fixture> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rscada/test-frames");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "hex"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "wired corpus missing");
    paths
        .into_iter()
        .map(|path| {
            // Reference XML supplies an independent expected record count. Byte input
            // and reference files are read once, outside every timed closure.
            let xml = fs::read_to_string(path.with_extension("norm.xml")).unwrap();
            Fixture {
                name: path.file_stem().unwrap().to_str().unwrap().to_owned(),
                bytes: decode_hex(&fs::read_to_string(&path).unwrap()),
                protocol: Protocol::Wired,
                expected_records: Some(xml.matches("<DataRecord ").count()),
                #[cfg(feature = "std")]
                xml: Some(xml),
            }
        })
        .collect()
}

fn wireless_fixtures() -> Vec<Fixture> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/wmbusmeters/test_vectors.json");
    let vectors: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let vectors = vectors.as_array().unwrap();
    assert!(!vectors.is_empty(), "wireless corpus missing");
    vectors
        .iter()
        .enumerate()
        .map(|(index, vector)| Fixture {
            name: format!("vector_{:02}", index + 1),
            bytes: decode_hex(vector["input"].as_str().unwrap()),
            protocol: Protocol::Wireless,
            // Only these vectors have a complete, unencrypted numeric reference.
            // The rest still participate in the link-layer benchmark.
            expected_records: (index < 3).then(|| {
                vector
                    .get("expected")
                    .unwrap()
                    .get("data_records")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len()
            }),
            #[cfg(feature = "std")]
            xml: None,
        })
        .collect()
}

fn bench_corpus(c: &mut Criterion, name: &str, fixtures: &[Fixture]) {
    let mut complete = Vec::new();
    let mut partial = Vec::new();
    for fixture in fixtures {
        fixture
            .frame()
            .unwrap_or_else(|error| panic!("{name}/{}: {error:?}", fixture.name));
        let Some(reference_records) = fixture.expected_records else {
            eprintln!(
                "{name}/{}: link-layer only (no complete semantic reference)",
                fixture.name
            );
            continue;
        };
        let work = fixture
            .decode()
            .unwrap_or_else(|error| panic!("{name}/{}: {error:?}", fixture.name));
        let (expected_records, expected_errors, has_records) = match (name, fixture.name.as_str()) {
            ("wired", "ELS_Elster-F96-Plus" | "abb_f95") => (reference_records - 2, 2, true),
            ("wired", "ELV-Elvaco-CMa10" | "THI_cma10" | "elv_temp_humid")
                if !cfg!(feature = "plaintext-before-extension") =>
            {
                (1, 1, true)
            }
            ("wired", "manual_frame2" | "sen_pollusonic_2") => (0, 0, false),
            _ => (reference_records, 0, true),
        };
        assert_eq!(
            (work.records, work.record_errors, work.has_records),
            (expected_records, expected_errors, has_records),
            "{name}/{}: workload changed; investigate before comparing timings",
            fixture.name
        );
        if work.record_errors == 0 {
            complete.push(fixture);
        } else {
            eprintln!("{name}/{}: partial decode {work:?}", fixture.name);
            partial.push(fixture);
        }
    }
    eprintln!(
        "{name}: {} link-layer inputs, {} complete semantic inputs, {} partial inputs",
        fixtures.len(),
        complete.len(),
        partial.len()
    );
    for (stage, subset) in [
        ("frame", fixtures.iter().collect::<Vec<_>>()),
        ("semantic", complete),
        ("partial", partial),
    ] {
        if subset.is_empty() {
            continue;
        }
        let mut group = c.benchmark_group(format!("{name}/{stage}"));
        group
            .sample_size(30)
            .warm_up_time(Duration::from_millis(300))
            .measurement_time(Duration::from_secs(1));
        for fixture in &subset {
            group.throughput(Throughput::Bytes(fixture.bytes.len() as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(&fixture.name),
                fixture,
                |b, fixture| {
                    b.iter(|| {
                        if stage == "frame" {
                            fixture.frame().unwrap();
                        } else {
                            black_box(fixture.decode().unwrap());
                        }
                    });
                },
            );
        }
        // One iteration visits each input exactly once in stable filename order.
        group.throughput(Throughput::Elements(subset.len() as u64));
        group.bench_function("mixed_corpus", |b| {
            b.iter(|| {
                for fixture in &subset {
                    if stage == "frame" {
                        fixture.frame().unwrap();
                    } else {
                        black_box(fixture.decode().unwrap());
                    }
                }
            })
        });
        group.finish();
    }
}

#[cfg(feature = "std")]
fn bench_xml(c: &mut Criterion, fixtures: &[Fixture]) {
    use m_bus_parser::{render_bytes, OutputFormat, RenderOptions};
    let options = RenderOptions::default();
    let mut group = c.benchmark_group("wired/xml");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1));
    for fixture in fixtures {
        let actual = render_bytes(&fixture.bytes, OutputFormat::Xml, &options).unwrap();
        let expected = fixture.xml.as_ref().unwrap();
        let known_mismatch = matches!(fixture.name.as_str(), "ELS_Elster-F96-Plus" | "abb_f95")
            || (!cfg!(feature = "plaintext-before-extension")
                && matches!(
                    fixture.name.as_str(),
                    "ELV-Elvaco-CMa10" | "THI_cma10" | "elv_temp_humid"
                ));
        if known_mismatch {
            assert_ne!(
                &actual, expected,
                "remove resolved XML mismatch: {}",
                fixture.name
            );
            eprintln!(
                "XML comparison excludes known semantic mismatch: {}",
                fixture.name
            );
            continue;
        }
        assert_eq!(&actual, expected, "XML parity: {}", fixture.name);
        group.throughput(Throughput::Bytes(fixture.bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(&fixture.name),
            fixture,
            |b, fixture| {
                b.iter(|| {
                    black_box(
                        render_bytes(black_box(&fixture.bytes), OutputFormat::Xml, &options)
                            .unwrap(),
                    );
                })
            },
        );
    }
    group.finish();
}

// Reference encoder used only during setup, never in a timed closure.
fn crc16(bytes: &[u8]) -> [u8; 2] {
    let mut crc = 0u16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x3d65
            } else {
                crc << 1
            };
        }
    }
    (crc ^ 0xffff).to_be_bytes()
}

fn bench_format_a(c: &mut Criterion) {
    use m_bus_parser::FormatAFrame;
    use wireless_mbus_link_layer::strip_format_a_crcs;
    let mut group = c.benchmark_group("wireless/format_a");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1));
    for length in [0, 1, 15, 16, 17, 31, 32, 128, 240] {
        let payload: Vec<u8> = (0..length).map(|n| (n * 17) as u8).collect();
        let header = [0, 0x44, 0x49, 0x6a, 0x31, 0, 1, 0x55, 0x14, 0x37];
        let mut encoded = header.to_vec();
        encoded.extend(crc16(&header));
        for chunk in payload.chunks(16) {
            encoded.extend(chunk);
            encoded.extend(crc16(chunk));
        }
        let mut expected = vec![(length + 9) as u8];
        expected.extend(&header[1..]);
        expected.extend(&payload);
        let mut output = vec![0; encoded.len()];
        assert_eq!(
            strip_format_a_crcs(&encoded, &mut output),
            Some(expected.as_slice())
        );
        assert!(FormatAFrame::new(&encoded)
            .unwrap()
            .bytes()
            .eq(expected.iter().copied()));
        group.throughput(Throughput::Bytes(encoded.len() as u64));
        group.bench_function(BenchmarkId::new("copy", length), |b| {
            b.iter(|| {
                // Output storage is reused; validation, CRC stripping, and copying
                // are timed, allocation and encoding are not.
                black_box(
                    strip_format_a_crcs(black_box(&encoded), black_box(&mut output)).unwrap(),
                );
            })
        });
        group.bench_function(BenchmarkId::new("view", length), |b| {
            b.iter(|| {
                let view = FormatAFrame::new(black_box(&encoded)).unwrap();
                for byte in view.bytes() {
                    black_box(byte);
                }
            })
        });
    }
    group.finish();
}

fn benchmarks(c: &mut Criterion) {
    let wired = wired_fixtures();
    bench_corpus(c, "wired", &wired);
    bench_corpus(c, "wireless", &wireless_fixtures());
    bench_format_a(c);
    #[cfg(feature = "std")]
    bench_xml(c, &wired);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
