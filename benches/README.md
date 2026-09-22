# Parser benchmarks

The website compares Rust and pinned [libmbus](libmbus-comparison/revision.txt)
in one table: average decode time, peak stack, and flash size.

```sh
python3 benches/measure-corpus.py \
  --output corpus-benchmarks.json --details corpus-benchmarks-details.json
```

Requires Git, a native C compiler, Rust `nightly-2026-05-16` with
`thumbv7em-none-eabi`, Arm GCC 14.2.1 (`ARM_GCC` or PATH), and `qemu-system-arm`.
CI installs these and publishes six metrics plus measurement details. libmbus is
cloned automatically; `--libmbus-source` accepts a clean checkout at the pinned revision.

- **Speed:** native Criterion timing across all 73 wired vectors, including
  known error cases and cleanup. Rust yields 897 records / 4 errors; libmbus
  yields 900 / 1. XML exclusions apply only to untimed correctness checks.
- **Stack:** highest observed Cortex-M4 watermark under QEMU, not a worst-case bound.
- **Flash:** linked Cortex-M4 `text + data`, including runtime but excluding fixtures.
  libmbus heap reservation is reported separately; Rust uses no heap.

Both builds use O3 without LTO and enable `plaintext-before-extension` for Rust.
Native Rust enables `std`; embedded Rust uses `no_std`. Rust returns typed,
borrowed values; libmbus includes normalization and allocation. Diagnostic I/O
is disabled in generated C sources. Compare timings on the same host.

## Detailed benchmarks and checks

The optional corpus suite covers individual wired/wireless inputs, aggregates,
and CRC cases. Aggregate times are per corpus pass, not per frame.

```sh
cargo bench --locked --bench corpus -- --test  # smoke check
cargo bench --locked --bench corpus -- mixed_corpus
cargo bench --locked --bench corpus --features std -- wired/xml
python3 -m unittest discover -s benches -p test_measure_corpus.py
BENCH_CORPUS_JSON=corpus-benchmarks.json node --test .github/scripts/test-bench-charts.cjs
```

See [parser-resources](parser-resources/README.md) for single-frame size and
optimization benchmarks, and [xml-exclusions.json](libmbus-comparison/xml-exclusions.json)
for known XML differences.
