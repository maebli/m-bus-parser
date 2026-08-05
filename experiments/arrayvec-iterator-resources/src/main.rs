use arrayvec_iterator_resources::{
    eager_probe, iterator_probe, parse_arrayvec, parse_iter, DemoRecordBuffer, Numbers,
    ParsedNumbers, CAPACITY,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    mem::{align_of, size_of},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
        System.realloc(ptr, old, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn allocation_measurement<T>(work: impl FnOnce() -> T) -> (T, usize, usize) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    let value = work();
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    let bytes = ALLOCATED_BYTES.load(Ordering::SeqCst);
    (value, allocations, bytes)
}

fn benchmark(name: &str, iterations: u32, mut work: impl FnMut() -> u32) {
    let start = Instant::now();
    let mut checksum = 0_u32;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(black_box(work()));
    }
    let elapsed = start.elapsed();
    let nanos_per_parse = elapsed.as_nanos() as f64 / f64::from(iterations);
    println!(
        "time.{name}: total={elapsed:?}, iterations={iterations}, ns_per_parse={nanos_per_parse:.1}, checksum={checksum}"
    );
}

fn main() {
    const INPUT: &str = "1 2 3 4 5 6 7 8 9 10";

    println!("target.pointer_width_bits: {}", usize::BITS);
    println!("capacity: {CAPACITY}");
    println!(
        "type.arrayvec: size={} align={}",
        size_of::<ParsedNumbers>(),
        align_of::<ParsedNumbers>()
    );
    println!(
        "type.iterator: size={} align={}",
        size_of::<Numbers<'static>>(),
        align_of::<Numbers<'static>>()
    );
    println!(
        "type.demo_record_arrayvec: size={} align={}",
        size_of::<DemoRecordBuffer>(),
        align_of::<DemoRecordBuffer>()
    );

    let (eager, eager_allocs, eager_bytes) =
        allocation_measurement(|| parse_arrayvec(INPUT).unwrap());
    let (lazy_sum, lazy_allocs, lazy_bytes) = allocation_measurement(|| {
        parse_iter(INPUT)
            .map(Result::unwrap)
            .map(u32::from)
            .sum::<u32>()
    });

    println!(
        "heap.eager: allocations={eager_allocs} requested_bytes={eager_bytes} result_len={}",
        eager.len()
    );
    println!(
        "heap.iterator: allocations={lazy_allocs} requested_bytes={lazy_bytes} sum={lazy_sum}"
    );

    let probe_eager = unsafe { eager_probe(INPUT.as_ptr(), INPUT.len()) };
    let probe_iterator = unsafe { iterator_probe(INPUT.as_ptr(), INPUT.len()) };
    println!("probe.eager_sum: {probe_eager}");
    println!("probe.iterator_sum: {probe_iterator}");

    let iterations = 1_000_000;
    benchmark("eager", iterations, || unsafe {
        eager_probe(INPUT.as_ptr(), INPUT.len())
    });
    benchmark("iterator", iterations, || unsafe {
        iterator_probe(INPUT.as_ptr(), INPUT.len())
    });
}
