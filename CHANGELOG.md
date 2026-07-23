# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
