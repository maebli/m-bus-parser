# Parser resource benchmark

This benchmark eagerly parses one representative wired M-Bus frame and consumes
all nine application-layer data records. It uses a pinned compiler and
dependency lockfile and does not use QEMU.

## Metrics

- Full-parse stack, its nested setup and record paths, and each local frame on
  the deepest path, compiled for
  `thumbv7em-none-eabi` and read from LLVM `.stack_sizes` metadata.
- `DataRecord` and `ValueInformationBlock` value sizes on the same Thumb build.
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
    ValueInformation::try_from -> consume_orthhogonal_vife
)
```

LLVM stack sizes are deterministic static lower bounds. They exclude interrupt
handlers, dynamic dispatch, call bookkeeping, and code outside this path.

## Run locally

Install the pinned nightly toolchain, LLVM tools, and Thumb target, then run:

```console
rustup component add llvm-tools-preview --toolchain nightly-2026-05-16
rustup target add thumbv7em-none-eabi --toolchain nightly-2026-05-16
STACK_USAGE_TOOLCHAIN=nightly-2026-05-16 python3 benches/stack-usage/measure.py \
  --output parser-resources.json
cargo +nightly-2026-05-16 bench --bench bench -- parse_full_frame_eager --exact
```

Pull requests show the current values in the Actions summary and artifact.
Pushes to `main` add them to the
[parser resource trend dashboard](https://maebli.github.io/m-bus-parser/dev/bench/).
