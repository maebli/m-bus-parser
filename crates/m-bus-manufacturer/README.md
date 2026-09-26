# Manufacturer-specific M-Bus decoders

A `no_std`, allocation-free extension point for DIF `0x0F`/`0x1F` tails. There are
no registered vendors yet. The [runnable example](examples/decoder.rs) implements a
fictional layout with status flags, conditional temperature, and known-answer tests.
It is deliberately excluded from `BUILTIN`.

## Adding a requested decoder

1. Establish the byte layout from a vendor specification and reviewed sample frames.
2. Copy the example's imports, `Decoder` implementation and `tests` module into
   `decoders/<code>_<slug>.rs` (for example `abc_heat_meter.rs`). Change imports from
   `m_bus_manufacturer` to `crate`; omit `main` and the example's introductory comment.
3. Supply the real manufacturer code, inclusive version range, device restriction,
   name and specification URL/section in a static `DecoderDescriptor`.
4. Replace the synthetic layout and fixtures. Include success, truncation and relevant
   branch cases. Keep `tests::FIXTURES` visible to its parent with `pub(super)`.
5. Run `cargo test -p m-bus-manufacturer --all-targets` and format the contribution
   with `rustfmt --edition 2021 decoders/<code>_<slug>.rs`.

The build script discovers `.rs` files in sorted order and generates modules,
`builtin::ABC_HEAT_METER` and `BUILTIN`. Generated tests check the filename's code and
run `FIXTURES`; registry tests reject invalid/overlapping selectors and empty sources.
No manifest or manual registry edits are needed when adding a decoder. Full-frame
regression tests live in the root crate, avoiding a dependency cycle with the parser.

## Decoder contract

`decode(meter, tail, emit)` receives only the bytes after the manufacturer DIF and
returns how many it consumed. Fields include their tail-relative byte range, exact
value, decimal exponent, units, and optional flag/enum labels. The visitor consumes
borrowed fields synchronously; it can copy or render them without allocating in the
library. Decoders must terminate and must not panic on malformed input.

`Cursor` checks every read and leaves its position unchanged on failure. It supports
signed/unsigned 1–8-byte integers in either byte order, exact packed BCD (2–12 even
numbers of digits), date G, datetime F, slices, skipping, and the remaining bytes.
Named integer helpers widen to `u64`/`i64`; `u8`/`i8` return their native types.
BCD rejects nondecimal nibbles; any vendor-specific sign/sentinel handling belongs in
the decoder. Dates retain the protocol parser's invalid/every components.

Use `Field::unsigned`, `Field::signed`, `Field::bytes`, or `Field::new` with a date.
Attach scaling with `.exponent(-2)` and units with
`.units(&[Unit { name: UnitName::Celsius, exponent: 1 }])`. `.flags` uses bit indices;
`.enumeration` uses exact `Integer::Signed`/`Unsigned` keys. Neither uses floating point.

## Dispatch from firmware

```rust
use m_bus_manufacturer::{MeterInfo, Registry};
let meter = MeterInfo { manufacturer: Some(*b"ABC"), version: Some(1), device: None };
let result = Registry::default().decode(&meter, &[0x01, 0x02], &mut |field| {
    // Consume the reading immediately, e.g. forward it to your application.
    let _ = field;
});
assert!(result.is_none()); // no built-ins have been added yet
```

`Registry::new(&custom)` searches custom decoders before built-ins. `Registry::only`
uses exactly the supplied list, allowing firmware to link just selected statics or
callers to disable decoding. The first matching selector wins; failures never retry
another decoder. Missing required metadata does not match. `None` means no match;
`Some(Err(...))` retains preceding fields and carries a tail-relative error offset.
On success, leftover bytes are emitted as a raw `unparsed` field. The caller retains
the original tail. Reported field ranges and consumed lengths are checked, but this
is not a sandbox for arbitrary Rust: termination and bounds handling require review.

## Full-frame output

Enable the root crate's `manufacturer-decoders` feature (independent of `std`). Its
`std` output API automatically uses built-ins. Existing `DecodeOptions` and functions
are unchanged. For custom decoders use `decode_bytes_with_decoders` or
`decode_hex_with_decoders`, passing a borrowed `Registry`; then call
`output::render_decoded` for canonical JSON/YAML, table or CSV output.

Identity selection follows the existing output API: long TPL first, otherwise the
wireless link identity. Encrypted records are dispatched only after decryption.
Original records and raw bytes remain intact. Matched records add optional
`manufacturer_decoder`, `manufacturer_fields`, and `manufacturer_error` fields.
A decoder failure marks the output partial and adds a diagnostic, while preserving
previously decoded fields. Integers and scaled decimals serialize as exact strings.
Table output shows child readings and errors. CSV preserves one row per frame and
adds `record_N_manufacturer_*` columns only for matched records; field indices keep
duplicate names unambiguous. XML, annotations and diagrams retain their existing
standard-record representations. The CLI enables built-ins; browser code is unchanged.

## Validation

```sh
cargo test -p m-bus-manufacturer --all-targets
cargo test -p m-bus-parser --features std,manufacturer-decoders --test manufacturer_decoders
cargo build -p m-bus-manufacturer --no-default-features --target thumbv7em-none-eabi
cargo run -p m-bus-manufacturer --example decoder
```

The structured `testing::Fixture` table asserts every field, range, unit, label,
consumed length and error. It requires at least one successful nonempty expectation;
additional ordinary Rust tests may cover more complex behavior. No browser editor,
compiler service, dynamic loader or automatic reverse engineering is included.
