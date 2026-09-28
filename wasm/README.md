# WASM package for m-bus-parser

Browser bindings for the wired and wireless M-Bus parser.

- `m_bus_decode(data, key?, includeEnrichment?)` returns the canonical schema
  as a native JavaScript object.
- `m_bus_render(data, format, key?, width?, includeEnrichment?)` renders
  `table`, `json`, `yaml`, `csv`, `mermaid`, `xml`, `annotated`, or
  `annotated-text`.
- `m_bus_render_application(data, format, key?, width?, includeEnrichment?)` renders
  CI-prefixed application blocks or bare DIF/VIF records with no link frame.
  The website uses it only after wired and wireless frame detection fails.
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

## Online M-Bus parser

The website accepts full wired and wireless frames, then tries application-only
input as its final fallback: a CI-prefixed block (for example `78 03 13 15 31 00`)
or bare DIF/VIF records (`03 13 15 31 00`). It labels fallback results and supports
JSON, YAML, table, CSV, diagram, XML, and hex views. Recognized CI headers take
priority over interpreting the same bytes as DIF/VIF records. Invalid or partial
records fail the fallback rather than silently dropping bytes.

Failures and partial frame decodes offer prefilled GitHub issue links with
the captured payload, errors, version, selected format, browser, and replay URL.
Review the report before submitting it; AES keys are omitted. The toolbar report
button reuses the last failed attempt even if the input has since been edited.

The Pages workflow builds WASM from `wasm/` and copies the generated assets into
`docs/` before deployment. To build locally, run:

```sh
wasm-pack build wasm --target web --out-dir /tmp/m-bus-web-wasm
```

Serve a copy of `docs/` with the generated JS/WASM files over HTTP. Website logic
regressions run with `node --test .github/scripts/test-parser-ui.mjs`.
