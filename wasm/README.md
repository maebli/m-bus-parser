# WASM package for m-bus-parser

Browser bindings for the wired and wireless M-Bus parser.

- `m_bus_decode(data, key?, includeEnrichment?)` returns the canonical schema
  as a native JavaScript object.
- `m_bus_render(data, format, key?, width?, includeEnrichment?)` renders
  `table`, `json`, `yaml`, `csv`, `mermaid`, `xml`, `annotated`, or
  `annotated-text`.
- `m_bus_highlight(source, language)` highlights JSON, YAML, CSV, or XML with
  the Rust-only `syntect` grammar bundle and returns escaped, prefixed
  span-only markup.
- Rejected promises/errors are `MbusParserError` objects with stable `code`,
  `layer`, and optional `byteOffset` properties.
- `m_bus_parse` and `m_bus_parse_with_key` remain compatibility wrappers.

```js
import init, { m_bus_decode, m_bus_highlight, m_bus_render } from 'm-bus-parser-wasm-pack'

await init()
const telegram = '68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16'
const decoded = m_bus_decode(telegram)
const narrowTable = m_bus_render(telegram, 'table', undefined, 48)
const highlighted = m_bus_highlight(JSON.stringify(decoded, null, 2), 'json')
```
