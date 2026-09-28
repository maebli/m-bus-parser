# Online M-Bus parser

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
