# Manufacturer-specific M-Bus decoders

A standalone `no_std`, allocation-free crate for decoding manufacturer-specific
DIF `0x0F`/`0x1F` tails. No vendors are implemented yet; the parser, CLI and browser
are not connected to it. See the [runnable, tested example](examples/decoder.rs).

Implement `ManufacturerDecoder` with a descriptor (manufacturer, optional version
range/device, and specification reference) and a `decode` method. Use `Cursor` for
checked integer, BCD, date and slice reads, then emit named `Field`s with byte ranges,
exact values, scaling, units and optional flag/enum labels. Failed reads leave the
cursor unchanged. Fields are borrowed and must be consumed within the callback.

Pass an explicit list to `Registry::new(&[&MyDecoder])`, then call
`decode(&meter, tail, &mut emit)` with the bytes after the manufacturer DIF.
The first matching decoder wins; missing required metadata does not match.
`None` means no match. Success returns the consumed byte count and descriptor,
emitting any leftover bytes as `unparsed`. Errors carry a tail-relative offset and
retain earlier fields; another decoder is never tried after a failure. The caller
keeps the original bytes. Decoders are trusted Rust and must terminate without
panicking on malformed input.

When a request arrives, add a normal Rust module and ordinary tests using documented
sample readings, including truncation and conditional fields. Register the decoder
explicitly and add full-frame integration when connecting it to the parser.

```sh
cargo test -p m-bus-manufacturer --all-targets
cargo run -p m-bus-manufacturer --example decoder
cargo build -p m-bus-manufacturer --target thumbv7em-none-eabi
```
