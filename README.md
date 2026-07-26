# m-bus-parser

[![Discord](https://img.shields.io/badge/Discord-Join%20Now-blue?style=flat&logo=Discord)](https://discord.gg/FfmecQ4wua)
[![Crates.io](https://img.shields.io/crates/v/m-bus-parser.svg)](https://crates.io/crates/m-bus-parser)
[![Downloads](https://img.shields.io/crates/d/m-bus-parser.svg)](https://crates.io/crates/m-bus-parser)
[![License](https://img.shields.io/crates/l/m-bus-parser.svg)](https://crates.io/crates/m-bus-parser)
[![Documentation](https://docs.rs/m-bus-parser/badge.svg)](https://docs.rs/m-bus-parser)
[![Build Status](https://github.com/maebli/m-bus-parser/actions/workflows/rust.yml/badge.svg)](https://github.com/maebli/m-bus-parser/actions/workflows/rust.yml)

*For contributing see [CONTRIBUTING.md](./CONTRIBUTING.md), for change history see [CHANGELOG.md](./CHANGELOG.md).*

---

An open-source parser (decoder/deserializer) for the **wired** and **wireless** M-Bus protocol, written in Rust.

> M-Bus (Meter-Bus) is a European standard (EN 13757-2 physical and link layer, EN 13757-3 application layer) for remote reading of water, gas, electricity, and heat meters. — [Wikipedia](https://en.wikipedia.org/wiki/Meter-Bus)

- Try it live: **[maebli.github.io/m-bus-parser](https://maebli.github.io/m-bus-parser/)**
- Spec: [m-bus.com/documentation](https://m-bus.com/documentation) · [OMS specification](https://oms-group.org/en/open-metering-system/oms-specification)

---

## Features

- Parses **wired M-Bus** (EN 13757-2/-3) and **wireless M-Bus** (wMBus) frames
- **Eight harmonized output formats**: `table`, `json`, `yaml`, `csv`,
  `mermaid`, `xml`, `annotated`, and `annotated-text`
- A versioned canonical schema with exact decimal values, provenance,
  partial-decode diagnostics, and stable error codes
- Responsive, Unicode-aware tables for narrow terminals and browser cards
- **AES-128 decryption** for encrypted wMBus frames (mode 5 / mode 7)
- **`no_std` compatible** — runs on embedded targets (manufacturer lookup and output formats require `std`)
- Available as a **Rust library**, **CLI**, **WebAssembly (npm)** and **Python bindings**

---

## Deployments

### Web app (WebAssembly)
[![npm](https://img.shields.io/npm/dm/m-bus-parser-wasm-pack.svg)](https://www.npmjs.com/package/m-bus-parser-wasm-pack)
[![npm](https://img.shields.io/npm/v/m-bus-parser-wasm-pack.svg)](https://www.npmjs.com/package/m-bus-parser-wasm-pack)

Paste a hex frame at **[maebli.github.io/m-bus-parser](https://maebli.github.io/m-bus-parser/)** and get instant output in any format, including a rendered Mermaid diagram. Frames can be shared via URL.

Source: [`wasm/`](./wasm)

### CLI
[![Crates.io](https://img.shields.io/crates/v/m-bus-parser-cli.svg)](https://crates.io/crates/m-bus-parser-cli)
[![Downloads](https://img.shields.io/crates/d/m-bus-parser-cli.svg)](https://crates.io/crates/m-bus-parser-cli)

```bash
cargo install m-bus-parser-cli
```

Source: [`cli/`](./cli)

### Python bindings
[![PyPI version](https://badge.fury.io/py/pymbusparser.png)](https://badge.fury.io/py/pymbusparser)

```bash
pip install pymbusparser
```

Source: [`python/`](./python)

---

## CLI Usage

```
m-bus-parser-cli parse [OPTIONS]

Options:
  -d, --data <DATA>      Raw M-Bus frame as a hex string
  -f, --file <FILE>      File containing a hex frame
  -t, --format <FORMAT>  table, json, yaml, csv, mermaid, xml, annotated, annotated-text
  -k, --key <KEY>        AES-128 decryption key (32 hex characters)
      --width <WIDTH>    Table width (auto-detected on an interactive terminal)
      --no-enrichment    Omit manufacturer enrichment
```

Input hex is strict: use compact hexadecimal or complete byte tokens separated
by whitespace, colons, or hyphens.

```
68 3D 3D 68 ...      (space-separated)
683D3D68...          (plain hex)
0x68:0x3D:0x3D:0x68  (prefixed byte tokens)
```

### Table output (default)

The table automatically selects a wide, compact, or vertical-card layout and
never exceeds the detected terminal width. Use `--width 44` to request an exact
maximum explicitly.

### Other formats

```bash
# JSON
m-bus-parser-cli parse -d "..." -t json

# YAML
m-bus-parser-cli parse -d "..." -t yaml

# CSV (one row per input frame; record fields use namespaced columns)
m-bus-parser-cli parse -d "..." -t csv

# Colored, semantically grouped Mermaid diagram source (renders in the web app)
m-bus-parser-cli parse -d "..." -t mermaid

# Wired-compatible and wireless XML
m-bus-parser-cli parse -d "..." -t xml

# Byte annotations as JSON or human-readable text
m-bus-parser-cli parse -d "..." -t annotated
m-bus-parser-cli parse -d "..." -t annotated-text

# With AES-128 decryption key
m-bus-parser-cli parse -d "..." -k "000102030405060708090A0B0C0D0E0F"
```

---

## Library Usage

Add to `Cargo.toml`:

```toml
[dependencies]
m-bus-parser = { version = "0.4", features = ["std", "serde"] }
```

### Parse a wired frame

```rust
use m_bus_parser::{Address, WiredFrame, Function};
use m_bus_parser::mbus_data::MbusData;
use m_bus_parser::user_data::parse_application_layer;

let frame_bytes: Vec<u8> = vec![
    0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01,
    // ... rest of frame
];

let frame = WiredFrame::try_from(frame_bytes.as_slice())?;

if let WiredFrame::LongFrame { function, address, data } = frame {
    let application_layer = parse_application_layer(data)?;
    if let Some(records) = application_layer.data_records() {
        for record in records {
            println!("{:?}", record?.value());
        }
    }
}
```

### Parse application-layer data records

When the link and transport headers have already been removed, parse the DIF/VIF
records directly:

```rust
use m_bus_parser::user_data::parse_data_records;

let data = [0x03, 0x13, 0x15, 0x31, 0x00];
for record in parse_data_records(&data) {
    let record = record?;
    println!("value: {:?}", record.value());
    println!("value information: {:?}", record.value_information());
}
```

### Decode and render with typed APIs

```rust
use m_bus_parser::{
    DecodeOptions, OutputFormat, RenderOptions, decode_hex, render_hex,
};

let hex = "68 3D 3D 68 08 01 72 ...";
let decoded = decode_hex(hex, &DecodeOptions::default())?;
println!("schema v{}: {}", decoded.schema_version, decoded.protocol);

let table = render_hex(
    hex,
    OutputFormat::Table,
    &RenderOptions {
        table_width: Some(72),
        ..RenderOptions::default()
    },
)?;
```

`serialize_mbus_data` remains as a string compatibility wrapper. New code
should use the typed APIs so invalid input and unsupported options remain
machine-readable `OutputError` values.

### `no_std` usage

The core parsing types are `no_std` compatible. Disable default features:

```toml
[dependencies]
m-bus-parser = { version = "0.4", default-features = false }
```

An embedded example (Cortex-M) is in [`examples/cortex-m/`](./examples/cortex-m).

---

## Output Formats

| Format    | Flag    | Description                                      |
|-----------|---------|--------------------------------------------------|
| `table`         | default             | Width-aware human-readable table |
| `json`          | `-t json`           | Canonical schema as JSON |
| `yaml`          | `-t yaml`           | Canonical schema as YAML |
| `csv`           | `-t csv`            | One frame row with namespaced record columns |
| `mermaid`       | `-t mermaid`        | Colored, layer-oriented Mermaid flowchart |
| `xml`           | `-t xml`            | Wired libmbus-compatible and wireless XML |
| `annotated`     | `-t annotated`      | Byte-segment annotation envelope |
| `annotated-text`| `-t annotated-text` | Human-readable byte annotations |

---

## Protocol Coverage

### Frame types

| Type              | CI bytes               | Status      |
|-------------------|------------------------|-------------|
| Long frame        | 0x72, 0x76, 0x7A       | Supported   |
| Short frame       | —                      | Supported   |
| Control frame     | —                      | Supported   |
| Single character  | —                      | Supported   |
| Wireless frame    | wMBus link layer       | Supported   |

### CI field types

#### Implemented
- `ResponseWithVariableDataStructure` (CI: 0x72, 0x76, 0x7A)
- `ResponseWithFixedDataStructure` (CI: 0x73)
- `ApplicationLayerShortTransport` (CI: 0x7D)
- `ApplicationLayerLongTransport` (CI: 0x7E)
- `ExtendedLinkLayerI` (CI: 0x8A)
- `ResetAtApplicationLevel`

#### Not yet implemented
Returns `ApplicationLayerError::Unimplemented` for: `SendData`, `SelectSlave`, `SynchronizeSlave`, baud-rate commands, `ExtendedLinkLayerII/III`, COSEM/OBIS data, and various transport/network layer types.

Most common value information unit codes are supported. Contributions for additional CI types and VIF codes are welcome.

---

## Frame Structure

### Wireless Link Layer

![](./resources/wireless-frame.png)

### Wired Link Layer (Long Frame)

![](./resources/function.png)

### Application Layer

![](./resources/application-layer.png)

### Value Information Block

![](./resources/application-layer-valueinformationblock.png)

---

## Related Projects

| Language | Project |
|----------|---------|
| C        | [libmbus by rscada](https://github.com/rscada/libmbus) |
| Java     | [jMbus](https://github.com/qvest-digital/jmbus) |
| C#       | [Valley.Net.Protocols.MeterBus](https://github.com/sympthom/Valley.Net.Protocols.MeterBus/) |
| JS       | [tmbus](https://dev-lab.github.io/tmbus/) |
| Python   | [pyMeterBus](https://github.com/ganehag/pyMeterBus) |
