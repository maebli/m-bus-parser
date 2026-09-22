use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use m_bus_parser::{render_bytes, OutputFormat, RenderOptions, WiredFrame};
use std::{
    collections::BTreeMap,
    ffi::{c_char, CStr},
    fs,
    hint::black_box,
    path::PathBuf,
    time::Duration,
};

#[path = "../../full_decode.rs"]
mod full_decode;

extern "C" {
    fn comparison_decode(bytes: *const u8, length: usize) -> full_decode::Decoded;
    fn comparison_frame(bytes: *const u8, length: usize) -> i32;
    fn comparison_xml(bytes: *const u8, length: usize) -> *mut c_char;
    fn comparison_free(xml: *mut c_char);
}

// libmbus uses static formatting buffers: this harness is deliberately single-threaded.
struct Xml(*mut c_char);
impl Xml {
    fn parse(bytes: &[u8]) -> Self {
        // SAFETY: a valid slice lives for the complete call; libmbus only reads
        // it. The returned malloc-owned, NUL-terminated string is freed in Drop.
        let pointer = unsafe { comparison_xml(bytes.as_ptr(), bytes.len()) };
        assert!(!pointer.is_null(), "libmbus XML parsing failed");
        Self(pointer)
    }
    fn bytes(&self) -> &[u8] {
        // SAFETY: ownership is retained until Drop, and libmbus terminates XML.
        unsafe { CStr::from_ptr(self.0).to_bytes() }
    }
}
impl Drop for Xml {
    fn drop(&mut self) {
        // SAFETY: allocated by libmbus and released once, using the same C allocator.
        unsafe { comparison_free(self.0) }
    }
}
struct Fixture {
    name: String,
    bytes: Vec<u8>,
    expected_records: u32,
}

fn benchmarks(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths: Vec<_> = fs::read_dir(root.join("tests/rscada/test-frames"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "hex"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let fixtures: Vec<_> = paths
        .iter()
        .map(|p| Fixture {
            name: p.file_stem().unwrap().to_str().unwrap().into(),
            expected_records: fs::read_to_string(p.with_extension("norm.xml"))
                .unwrap()
                .matches("<DataRecord ")
                .count() as u32,
            bytes: hex::decode(
                fs::read_to_string(p)
                    .unwrap()
                    .split_whitespace()
                    .collect::<String>(),
            )
            .unwrap(),
        })
        .collect();
    let options = RenderOptions::default();
    let exclusions: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../xml-exclusions.json")).unwrap();
    let mut mismatches = Vec::new();
    let mut matching = Vec::new();
    for fixture in &fixtures {
        WiredFrame::try_from(fixture.bytes.as_slice()).unwrap();
        // SAFETY: input is valid for the duration of this synchronous call.
        assert_eq!(
            unsafe { comparison_frame(fixture.bytes.as_ptr(), fixture.bytes.len()) },
            0,
            "{}",
            fixture.name
        );
        // Guard checksum work in both link parsers before collecting timings.
        let mut corrupt = fixture.bytes.clone();
        let checksum = corrupt.len().checked_sub(2).unwrap();
        corrupt[checksum] ^= 1;
        assert!(WiredFrame::try_from(corrupt.as_slice()).is_err());
        // SAFETY: corrupt is still a valid byte slice; failure is expected.
        assert_ne!(
            unsafe { comparison_frame(corrupt.as_ptr(), corrupt.len()) },
            0
        );
        let rust = render_bytes(&fixture.bytes, OutputFormat::Xml, &options).unwrap();
        let libmbus = Xml::parse(&fixture.bytes);
        if rust.as_bytes() != libmbus.bytes() {
            let cpp = String::from_utf8_lossy(libmbus.bytes());
            let differences: Vec<_> = rust
                .lines()
                .zip(cpp.lines())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .take(3)
                .map(|(line, (a, b))| serde_json::json!({"line":line+1,"rust":a,"libmbus":b}))
                .collect();
            mismatches.push(serde_json::json!({"fixture":fixture.name,"differences":differences}));
        } else {
            assert!(
                !exclusions.contains_key(&fixture.name),
                "resolved XML mismatch: {}",
                fixture.name
            );
            // This pinned libmbus version logs a VIF 0x7B normalization error
            // to stderr on every render. Keep I/O out of the timed workload.
            if fixture.name != "sen_pollutherm" {
                matching.push(fixture);
            }
        }
    }
    let report = serde_json::json!({
        "libmbus_revision": env!("LIBMBUS_REVISION"),
        "frame_fixtures": fixtures.iter().map(|f| &f.name).collect::<Vec<_>>(),
        "xml_fixtures": matching.iter().map(|f| &f.name).collect::<Vec<_>>(),
        "xml_mismatches": mismatches,
        "xml_diagnostic_exclusions": {"sen_pollutherm": "libmbus logs an unsupported VIF 0x7B error on every XML render"},
        "xml_exclusion_reasons": exclusions,
    });
    if let Some(path) = std::env::var_os("LIBMBUS_COMPARISON_REPORT") {
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    let actual: Vec<_> = mismatches
        .iter()
        .map(|m| m["fixture"].as_str().unwrap())
        .collect();
    let expected: Vec<_> = exclusions.keys().map(String::as_str).collect();
    assert_eq!(
        actual, expected,
        "XML mismatch set changed; inspect the preflight report"
    );
    assert!(!matching.is_empty(), "no equivalent XML workloads");
    eprintln!(
        "libmbus {}: {} frame inputs, {} equivalent XML inputs, {} XML mismatches plus one diagnostic exclusion",
        env!("LIBMBUS_REVISION"),
        fixtures.len(),
        matching.len(),
        mismatches.len()
    );

    let mut counts = Vec::new();
    for fixture in &fixtures {
        let rust = full_decode::decode(&fixture.bytes);
        // SAFETY: the pinned parser reads a valid slice; result is a repr(C) pair.
        let c = unsafe { comparison_decode(fixture.bytes.as_ptr(), fixture.bytes.len()) };
        let rust_errors = if matches!(fixture.name.as_str(), "ELS_Elster-F96-Plus" | "abb_f95") {
            2
        } else {
            0
        };
        let c_errors = if fixture.name == "sen_pollutherm" {
            1
        } else {
            0
        };
        assert_eq!(
            (rust.records + rust.errors, rust.errors),
            (fixture.expected_records, rust_errors),
            "Rust {}",
            fixture.name
        );
        assert_eq!(
            (c.records + c.errors, c.errors),
            (fixture.expected_records, c_errors),
            "libmbus {}",
            fixture.name
        );
        counts.push(serde_json::json!({"fixture":fixture.name,"rust_records":rust.records,"rust_errors":rust.errors,"libmbus_records":c.records,"libmbus_errors":c.errors}));
    }
    if let Some(path) = std::env::var_os("LIBMBUS_DECODE_REPORT") {
        fs::write(path, serde_json::to_string_pretty(&counts).unwrap()).unwrap();
    }
    let mut group = c.benchmark_group("comparison/decode");
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Elements(fixtures.len() as u64));
    group.bench_function("rust", |b| {
        b.iter(|| {
            for fixture in &fixtures {
                black_box(full_decode::decode(black_box(&fixture.bytes)));
            }
        })
    });
    group.bench_function("libmbus", |b| {
        b.iter(|| {
            for fixture in &fixtures {
                let bytes = black_box(fixture.bytes.as_slice());
                // SAFETY: valid, immutable input for the complete C call.
                black_box(unsafe { comparison_decode(bytes.as_ptr(), bytes.len()) });
            }
        })
    });
    group.finish();
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
