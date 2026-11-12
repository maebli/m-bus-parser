# M-Bus parser CLI

The CLI decodes wired and wireless M-Bus frames through the same canonical
output contract as the Rust, Python, and WebAssembly APIs.

```console
cargo install m-bus-parser-cli
m-bus-parser-cli parse --data "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16" --format json
```

Formats: `table` (default), `json`, `yaml`, `csv`, `mermaid`, `xml`,
`annotated`, and `annotated-text`.

Input accepts compact hexadecimal or complete `HH`/`0xHH` byte tokens separated
by whitespace, colons, or hyphens. Ambiguous or partial input is rejected with
a stable error code.

Tables detect the interactive terminal width automatically. For pipes, the
default is 100 columns; override it explicitly when needed:

```console
m-bus-parser-cli parse --file telegram.hex --format table --width 48
```

Pass a 32-digit AES-128 key with `--key`. Use `--no-enrichment` when only
protocol-derived data should be emitted.

## Mode 13: imported TLS session

From the repository root, decode a frame with session data exported by the
meter or gateway:

```sh
cargo run -p m-bus-parser-cli --no-default-features --features decryption-mode-13 -- \
  parse --file frame.hex --tls-session session.json --format json
```

The default CLI build also includes this feature. Use `--data "HEX"` instead of
`--file` for inline input. Omit `--tls-session` to inspect TLS record metadata.

`session.json` uses the same format as the browser. Replace the example values
with the values for the captured traffic:

```json
{
  "name": "Meter session",
  "cipher_suite": "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
  "direction": "client_to_server",
  "key_hex": "000102030405060708090a0b0c0d0e0f",
  "fixed_iv_hex": "01020304",
  "sequence_number": "5"
}
```

- `key_hex`: the selected direction's session encryption key, not the static meter key.
- `fixed_iv_hex`: its four-byte nonce salt, as eight hex digits.
- `sequence_number`: the first record's TLS counter, as a decimal string. It counts
  encrypted handshake records too. Repeated commands do not advance it.
- `direction`: `client_to_server` for meter → gateway, or `server_to_client` for
  gateway → meter.
- `cipher_suite`: the suite above requires a 16-byte key. The supported alternative,
  `TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384`, requires a 32-byte key.

Do not combine `--tls-session` with `--key`. Supported outputs are `json`, `yaml`,
`table`, `csv`, and `mermaid`. Input must contain complete TLS application records;
reassemble and remove AFL framing first. This command does not perform a TLS handshake.
Authentication failures return a nonzero exit code and no decoded output.
