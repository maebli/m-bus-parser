# Parser resource benchmark

This benchmark eagerly parses one representative wired M-Bus frame and consumes
all nine application-layer data records. It uses a pinned compiler and
dependency lockfile and does not use QEMU.

## Metrics

- Full-parse stack, its nested setup and record paths, and each local frame on
  the deepest path, compiled for
  `thumbv7em-none-eabi` and read from LLVM `.stack_sizes` metadata.
- `DataRecord`, `ValueInformationBlock`, and `ValueInformation` value sizes on the same Thumb build.
- Linked eager-parser text and data size for `thumbv7em-none-eabi`, using
  `opt-level=z`, fat LTO, and one codegen unit.
- The same eager full-frame decode latency, run natively with Criterion.

Frame/application setup and record iteration are sequential, so the full stack
uses the larger nested path rather than adding both:

```text
parse_full_wired_frame + max(
  MbusData::try_from -> frame/application setup,
  DataRecords::next -> DataRecord::try_from -> DataRecord::parse ->
    DataRecordHeader::try_from -> ProcessedDataRecordHeader::try_from ->
    ValueInformation::try_from + max(
      head_vif_info,
      OrthogonalVifes::fold -> OrthogonalVifes::next -> orthogonal_vife_info
    )
)
```

LLVM stack sizes are deterministic static lower bounds. They exclude interrupt
handlers, dynamic dispatch, call bookkeeping, and code outside this path.

## Run locally

Install the pinned nightly toolchain, LLVM tools, and Thumb target, then run:

```console
rustup component add llvm-tools-preview --toolchain nightly-2026-05-16
rustup target add thumbv7em-none-eabi --toolchain nightly-2026-05-16
PARSER_RESOURCES_TOOLCHAIN=nightly-2026-05-16 python3 benches/parser-resources/measure.py \
  --output parser-resources.json
cargo +nightly-2026-05-16 bench --bench bench -- parse_full_frame_eager --exact
```

Pull requests show the current values in the Actions summary and artifact.
Pushes to `main` add them to the
[parser resource trend dashboard](https://maebli.github.io/m-bus-parser/dev/bench/).

## Compare speed and size optimization

Run the same eager nine-record decode under `opt-level=z`, `s`, and `3`:

```console
PARSER_RESOURCES_TOOLCHAIN=nightly-2026-05-16 python3 benches/parser-resources/compare.py \
  --output optimization-comparison.json
```

Each setting uses fat LTO and one codegen unit, with isolated build directories.
Both the native Criterion benchmark and the Thumb footprint build use this
package's pinned dependency graph and the same fixture. The script reports:

- Native mean latency in ns/frame, with a 95% confidence interval (50 samples,
  one-second warm-up and three-second measurement).
- Thumb flash footprint (`text + data`) of the linked eager-parser executable.
- Thumb static RAM (`data + bss`), excluding stack and heap.

Native benchmarks use unwinding; the embedded executable uses `panic=abort`.
Native timings are not MCU cycle measurements. The existing stack estimate
remains a separate measurement; it is not extrapolated across optimization levels.

CI records all three settings. The dashboard overlays **Size (z)**, **Size (s)**,
and **Speed (3)** on the same latency chart and the same flash-size chart, with
static RAM on its own chart. Missing historical settings appear as gaps. Existing
stack and decode-speed histories remain available. Local timings should only be
compared within the same machine/run; CI tracks its own history.

The comparison JSON uses the same benchmark history format as `measure.py`.
Chart grouping, incomplete histories, tooltips, and commit links can be checked
with `node --test .github/scripts/test-bench-charts.cjs` from the repository root.
