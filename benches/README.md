# Parser benchmarks

## Rust vs libmbus: one comparison

The website shows one table with Rust and libmbus columns and three rows:

- Average full decode time per frame across **all 73 wired test vectors**, each
  visited once per iteration. Timing includes record decoding and cleanup.
- Peak observed decoder stack across the same vectors on Cortex-M4.
- Linked Cortex-M4 flash size (`text + data`, excluding embedded test vectors).

Speed is measured on the native host; it is not a Cortex-M4 cycle measurement.
Stack is measured with a watermark under QEMU, not a static worst-case bound.
Flash includes the common wrapper and each decoder's required runtime. libmbus
also uses heap memory; its reservation is reported beneath the table. Rust's
embedded decoder uses no heap.

```sh
PARSER_RESOURCES_TOOLCHAIN=nightly-2026-05-16 python3 benches/measure-corpus.py \
  --output corpus-benchmarks.json --details corpus-benchmarks-details.json
```

Requirements: Git, native Clang or C compiler, the pinned Rust toolchain with
`thumbv7em-none-eabi`, Arm GNU Toolchain 14.2.Rel1 (GCC 14.2.1), and
`qemu-system-arm`. Set `ARM_GCC` to the embedded compiler if it is not on PATH.
The runner also searches `target/arm-toolchain/`. CI installs these tools.

The exporter writes exactly six benchmark values plus a details file with
compiler versions, corpus fingerprint, libmbus revision, per-vector outcomes,
and memory methodology. Fresh Criterion output directories prevent stale
measurements entering the export. CI publishes the `Parser library comparison`
history series; the website presents its latest run in one table, with earlier
benchmark charts folded away. Local timings should only be compared within the
same machine and run.

## Workload and comparability

The isolated [comparison package](libmbus-comparison/Cargo.toml) builds
[rscada/libmbus](https://github.com/rscada/libmbus) at the commit pinned in
[revision.txt](libmbus-comparison/revision.txt). Its own lockfile and C build
requirements do not affect the parser package. The runner clones into
`target/libmbus-source`; `--libmbus-source` accepts an existing clean checkout
at that revision. It never resets a modified checkout.

Both libraries run in the same Criterion executable over identical bytes in
deterministic order. Rust traverses all records, labels, and units through its
public typed API. libmbus parses the frame, application data, and each normalized
record through its public API, then frees allocated records. These APIs expose
different representations: libmbus includes string normalization and allocation;
Rust retains typed and borrowed values. The result compares these public decoder
workloads, not identical internal operations. XML rendering is not timed.

All 73 vectors remain in the timed corpus, including known error cases. Of 901
attempted records, Rust produces 897 records and four errors (two each in
`ELS_Elster-F96-Plus` and `abb_f95`); libmbus produces 900 records and one error
in `sen_pollutherm`. Preflight checks these exact counts and rejects unexpected
changes. Frame/checksum checks and XML parity checks also run outside timing;
[xml-exclusions.json](libmbus-comparison/xml-exclusions.json) documents known XML
differences and does not exclude inputs from the timed decode workload.

A generated copy of libmbus disables its stderr diagnostic macro so error cases
do not time terminal I/O. The embedded build omits serial/TCP transport code.
Decoder logic and the pinned source checkout remain unchanged.

Both implementations use optimization level 3 without LTO. The native Rust
harness enables `std,plaintext-before-extension`; the embedded static library
uses the shared decode function with `no_std,plaintext-before-extension`.
Input loading, hex decoding, and correctness checks occur before timing.
Criterion uses 50 samples, one second of warm-up, and three seconds of
measurement. Means and confidence intervals are divided by the actual 73-frame
iteration count to report time per frame.

## Optional detailed investigations

`bench.rs` retains the original single-frame benchmark. `corpus.rs` provides
individual wired and wireless cases, corpus aggregates, Format A CRC workloads,
and optional XML rendering. These do not add rows to the comparison table.

```sh
cargo bench --locked --bench corpus -- --test
cargo bench --locked --bench corpus -- mixed_corpus
cargo bench --locked --bench corpus
cargo bench --locked --bench corpus --features std -- wired/xml
cargo bench --locked --bench corpus --features plaintext-before-extension -- --test
```

Detailed aggregate latencies are per complete corpus pass. Default-feature
semantic cases separate known partial decodes; enabling compatibility features
changes those groups. Wireless cases include 23 link-layer inputs, three
unencrypted semantic inputs, and synthetic CRC boundary cases. They are not
part of the wired libmbus comparison.

See [parser-resources](parser-resources/README.md) for the older single-fixture
size and optimization experiments.

```sh
python3 -m unittest discover -s benches -p test_measure_corpus.py
BENCH_CORPUS_JSON=target/corpus-benchmarks.json node --test .github/scripts/test-bench-charts.cjs
```
