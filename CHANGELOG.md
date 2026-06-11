# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
