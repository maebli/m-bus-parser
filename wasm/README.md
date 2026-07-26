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
const decoded = m_bus_decode('68 3D 3D 68 ...')
const narrowTable = m_bus_render('68 3D 3D 68 ...', 'table', undefined, 48)
const highlighted = m_bus_highlight(JSON.stringify(decoded, null, 2), 'json')
```
