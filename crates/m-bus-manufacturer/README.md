# Manufacturer-specific M-Bus decoders

A standalone `no_std`, allocation-free crate for decoding manufacturer-specific
DIF `0x0F`/`0x1F` tails. No vendors are implemented yet; the parser, CLI and browser
are not connected to it. See the [runnable, tested example](examples/decoder.rs).

A decoder reads bytes and reports named readings. For example:

```rust
let temperature = cursor.read(Cursor::i16_le)?;
let field = temperature.signed("temperature").exponent(-2);
// A raw value of -250 represents -2.50; the byte range is recorded automatically.
```

Use `reading.value` for conditions and calculations, and `.units(...)` or `.flags(...)`
for metadata. `Cursor` also supports BCD, dates and byte slices. Failed reads leave it
unchanged. Emitted fields are borrowed and must be consumed within the callback.
Add a `Decoder` table entry with the function, manufacturer/version/device restrictions
and a specification reference; the example shows the complete setup.

Pass the table to `decode(&decoders, &meter, tail, &mut emit)` with the bytes after
the manufacturer DIF.
The first matching decoder wins; missing required metadata does not match.
`None` means no match. Success returns the consumed byte count and matching entry,
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
