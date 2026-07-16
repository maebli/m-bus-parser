# pymbusparser

Python bindings for the Rust [`m-bus-parser`](https://github.com/maebli/m-bus-parser)
library. The package parses wired and wireless M-Bus frames and supports AES
decryption for the security modes implemented by the Rust library.

## Installation

```console
python -m pip install pymbusparser
```

## API

`parse` is the main entry point. It accepts either hexadecimal text or raw bytes
and returns ordinary Python dictionaries and lists:

```python
from pymbusparser import parse

telegram = """
68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00
04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B
00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C
8C 11 02 27 37 0D 0F 60 00 67 16
"""

frame = parse(telegram)
print(frame["frame"])
print(frame.get("data_records"))
```

Use `parse_records` when the input contains application-layer records without a
link-layer frame:

```python
from pymbusparser import parse_records

records = parse_records("2f2f0413fce0f5052f2f2f2f2f2f2f2f")
```

Use `render` for text-oriented output. Supported formats are `json`, `yaml`,
`csv`, `table`, `mermaid`, `annotated`, `annotated-text`, and `hexview`:

```python
from pymbusparser import render

print(render(telegram, "table"))
```

## Decryption

The wheel is built with decryption support. Pass a 16-byte AES key as bytes or
as a 32-digit hexadecimal string:

```python
from pymbusparser import parse

decoded = parse(encrypted_telegram, key=bytes.fromhex("00112233445566778899aabbccddeeff"))
```

Malformed frames, keys, and output formats raise `ValueError`; unsupported input
types raise `TypeError`.

## Compatibility

`m_bus_parse` and `parse_application_layer` remain available for callers that
expect serialized strings. New code should use `parse`, `parse_records`, and
`render`.

## Development

From this directory:

```console
python -m pip install maturin
maturin develop
python -m unittest discover -s tests -v
```

Release artifacts are built and published by the `Python Bindings CI` workflow
when a matching `python-v*` tag is pushed.
