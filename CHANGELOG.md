# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.2] - 2026-08-10

### Changed

- Simplified canonical record units to a single `unit` string. It contains the
  standardized representation when available, without exposing a technical
  `ucum` label, and falls back to the human-readable symbol for units without a
  standard mapping. This change advances the canonical output schema to
  version 2.
- Aligned canonical frame functions, record functions, quantities, and data
  coding names with M-Bus terminology (`RSP_UD`, `Instantaneous value`,
  `Volume flow`, `6-digit BCD`) across JSON, YAML, CSV, tables, Mermaid, Python,
  and WebAssembly. Annotation identifiers now use `snake_case`, and standalone
  application-record decoding now uses the same canonical record contract.

## [0.4.1]

### Changed

- Replaced the npm-based documentation highlighter with `syntect` running
  inside the Rust WebAssembly package. JSON, YAML, CSV, and XML now use
  prefixed semantic token classes and an audited high-contrast light/dark
  palette.
- CSV renders one row per input frame, with data records represented as
  namespaced columns instead of repeating frame metadata for every record.

### Fixed

- Removed doubled source lines that introduced blank gaps in JSON, CSV, and
  XML and caused line-number gutters to end halfway through long outputs.
- Restored semantic grouping and the high-contrast eight-color data-record
  palette in Mermaid diagrams.

## [0.4.0]

### Added

- A versioned canonical output schema shared by Rust, CLI, Python, and WASM,
  with exact decimal values, source provenance, partial-decode diagnostics,
  stable error codes, and optional manufacturer enrichment.
- Typed `decode_*` and `render_*` Rust APIs, a native-object WASM decoder, a
  custom Python exception, strict hexadecimal input, and explicit render
  options.
- Responsive Unicode-aware tables that adapt between wide, compact, and
  vertical layouts without exceeding the requested terminal or container
  width.
- Self-hosted Shiki syntax highlighting for JSON, YAML, CSV, and XML in the
  WASM documentation, with light/dark high-contrast themes, line numbers,
  keyboard scrolling, forced-colors support, and automated WCAG AA checks.

### Changed

- JSON, YAML, CSV, table, Mermaid, annotated, XML, and binding outputs now use
  one product-owned contract. Retained legacy serializers are available under
  explicit legacy format names through this compatibility release.
- Wireless link frames preserve the raw C-field and expose the decoded
  function only when the value is known.

### Fixed

- XML rendering now supports clear, encrypted, short/long transport, and ELL
  wireless M-Bus frames while retaining byte-for-byte wired libmbus parity.
- Tables no longer wrap or mangle inside narrow terminals and browser output
  cards.
- Syntax tokens and line-number gutters meet the WCAG 2.2 AA 4.5:1 text
  contrast threshold in both supported themes.

## [0.3.0]

### Added

- New `xml` output format that reproduces the legacy rSCADA/libmbus
  `mbus_frame_data_xml_normalized()` output byte for byte, as a drop-in
  replacement for consumers of libmbus's normalized XML. Available through
  `serialize_mbus_data(hex, "xml", None)`, the CLI (`-t xml`), the Python
  bindings, and a "Parse to XML" button in the web app.
- Parity test (`tests/rscada_xml.rs`) diffing the `xml` output against the
  reference `.norm.xml` files in `tests/rscada`.

### Fixed

- VIF plaintext extension chains are now walked from the correct offset when
  the plaintext VIF precedes the VIFE bytes (`plaintext-before-extension`),
  so records that previously aborted with `InvalidValueInformation` now parse.
- A 6-byte data field carrying a date/time VIF (0x6D) is now decoded as a
  type I (CP48) timestamp instead of type F, matching EN 13757-3.

## [0.2.0]

Breaking changes for JSON/YAML consumers:

- Changed `summary.records[].value` from a table-style string (e.g.
  `"(2850427)e-2[m³](Volume)"`) to structured fields: `value` (number),
  `exponent`, `unit` and `quantity`, derived from the processed data record
  header. The human-readable string is still available as
  `summary.records[].display`. Table and CSV outputs are unchanged.
- Removed the top-level `manufacturer_info` key from JSON and YAML output.
  Migration: use `summary.manufacturer`, which carries `code`, `name`,
  `website` and `description`.
- Changed raw byte payloads (`frame.data` of wireless frames,
  `data_records[].raw_bytes` and manufacturer-specific record data) to
  serialize as compact uppercase hex strings instead of decimal byte arrays.
  Migration: decode the hex string instead of reading a JSON/YAML array
  (e.g. `"2F2F"` instead of `[47, 47]`). Serialize-only: parsing APIs and
  `Deserialize` implementations are unchanged.

## [0.1.4] - 2026-07-17

- Added direct application-layer parsing APIs, record accessors, and a crate-local data-record example.
- Moved full-frame application-layer coverage to the top-level crate.
- Fixed data-record iteration to report malformed records instead of silently stopping.
- Fixed mojibake in decrypted variable-length UTF-8 text while preserving ISO-8859-1 fallback decoding.

## [0.1.3] - 2026-06-11

- Added robust WASM docs hex view support for CI 0x78 frames with trailing CRC bytes.
- Improved the hex view renderer for common hex input forms, ASCII column alignment, and record field coloring.
- Added per-field copy buttons for the interactive hex view.

## [0.1.2] - 2026-06-09

- Added support for CI 0x78 frames without a transport layer header.
- Improved WASM docs hex view alignment and ASCII column spacing.
- Deduplicated byte hover labels in the WASM docs hex view.
- Updated RustCrypto and serde XML dependencies.

## [0.1.1] - 2026-05-14

- Added byte-level frame annotations and interactive hex view support for the WASM site.
- Added and corrected VIF/VIFE labels, units, extension handling, and special-function parsing.
- Improved LVAR text decoding with ISO/IEC 8859-1 handling.
- Added package metadata and versioned path dependencies for crate publishing.

## [0.1.0]

- wmbus parsing capabilities
- preperation for decryption
- breaking changes to API for using the lib
- refacatoring things into core to be shared by wireless and wired parsing parts
