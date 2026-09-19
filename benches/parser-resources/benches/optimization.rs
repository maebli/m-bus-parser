use criterion::{criterion_group, criterion_main, Criterion};
use m_bus_parser_resources::{parse_full_wired_frame, FULL_FRAME};
use std::hint::black_box;

fn eager_decode(c: &mut Criterion) {
    assert_eq!(parse_full_wired_frame(&FULL_FRAME), Ok(9));
    c.bench_function("parse_full_frame_eager", |b| {
        b.iter(|| black_box(parse_full_wired_frame(black_box(&FULL_FRAME)).unwrap()));
    });
}

criterion_group!(benches, eager_decode);
criterion_main!(benches);
