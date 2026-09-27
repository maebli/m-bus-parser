use criterion::{criterion_group, criterion_main, Criterion};
use m_bus_parser::user_data::data_information::DataFieldCoding;
use m_bus_parser::WiredFrame;
use std::hint::black_box;

mod full_parse_fixture;
use full_parse_fixture::{parse_full_wired_frame, FULL_FRAME};

#[allow(clippy::unwrap_used)]
fn frame_parse_benchmark(c: &mut Criterion) {
    let data: Vec<u8> = vec![0x68, 0x04, 0x04, 0x68, 0x53, 0x01, 0x00, 0x00, 0x54, 0x16];
    c.bench_function("parse_frame_only", |b| {
        b.iter(|| {
            // Use black_box to prevent compiler optimizations from skipping the computation
            WiredFrame::try_from(black_box(data.as_slice())).unwrap();
        })
    });
}

#[allow(clippy::unwrap_used)]
fn m_bus_parser_benchmark(c: &mut Criterion) {
    c.bench_function("parse_full_frame_eager", |b| {
        b.iter(|| {
            black_box(parse_full_wired_frame(black_box(&FULL_FRAME)).unwrap());
        })
    });
}

fn bcd_parse_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bcd");
    for (name, coding, data) in [
        ("2_digits", DataFieldCoding::BCD2Digit, &[0x98][..]),
        (
            "8_digits",
            DataFieldCoding::BCD8Digit,
            &[0x98, 0x76, 0x54, 0x32][..],
        ),
        (
            "12_digits",
            DataFieldCoding::BCDDigit12,
            &[0x98, 0x76, 0x54, 0x32, 0x10, 0x98][..],
        ),
        (
            "18_digits",
            DataFieldCoding::VariableLength,
            &[0xC9, 0x98, 0x76, 0x54, 0x32, 0x10, 0x98, 0x76, 0x54, 0x32][..],
        ),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| black_box(black_box(coding).parse(black_box(data), None)))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    frame_parse_benchmark,
    m_bus_parser_benchmark,
    bcd_parse_benchmark
);
criterion_main!(benches);
