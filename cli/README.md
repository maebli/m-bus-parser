# M-Bus parser CLI

The CLI decodes wired and wireless M-Bus frames through the same canonical
output contract as the Rust, Python, and WebAssembly APIs.

```console
cargo install m-bus-parser-cli
m-bus-parser-cli parse --data "68 3D 3D 68 ..." --format json
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
