# Parser benchmarks

```sh
python3 benches/measure-corpus.py
```

Writes `corpus-benchmarks.json` and `corpus-benchmarks-details.json`. CI publishes
one Rust/libmbus table: native average decode time, Cortex-M4 peak stack, and flash.

Requires Git, a C compiler, Rust `nightly-2026-05-16` with `thumbv7em-none-eabi`,
Arm GCC 14.2.1 (`ARM_GCC` or PATH), and `qemu-system-arm`. The runner clones
[pinned libmbus](libmbus-comparison/revision.txt) automatically.

All 73 wired vectors are timed, including known errors (Rust: 4; libmbus: 1).
[XML exclusions](libmbus-comparison/xml-exclusions.json) affect only untimed checks.
Both use O3 without LTO. Rust returns typed values; libmbus includes normalization,
allocation, and cleanup. Diagnostic I/O is disabled. Compare speed on the same host.
Stack is an observed watermark, not a worst-case bound. Flash includes runtime,
excludes fixtures; heap is reported separately.

For detailed cases, run `cargo bench --locked --bench corpus` (add `-- --test`
for a smoke check). See [parser-resources](parser-resources/README.md) for
single-frame size and optimization benchmarks.
