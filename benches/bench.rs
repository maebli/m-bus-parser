use criterion::{black_box, criterion_group, criterion_main, Criterion};
use m_bus_parser::frames::parse_frame;

fn frame_parse_benchmark(c: &mut Criterion) {
    let data = [0x68, 0x04, 0x04, 0x68, 0x53, 0x01, 0x00, 0x00, 0x54, 0x16];
    c.bench_function("parse_frame", |b| b.iter(|| {
        // Use black_box to prevent compiler optimizations from skipping the computation
        parse_frame(black_box(&data)).unwrap();
    }));
}

criterion_group!(benches, frame_parse_benchmark);
criterion_main!(benches);
