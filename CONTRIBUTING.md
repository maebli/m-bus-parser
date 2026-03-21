# Contributing

Contributions are welcome and appreciated — whether it's a bug report, a new feature, a test frame, or a documentation fix. Don't hesitate to open an issue or ask questions.

---

## Table of Contents

- [Getting oriented](#getting-oriented)
- [Setting up](#setting-up)
- [Project structure](#project-structure)
- [Making changes](#making-changes)
- [Testing](#testing)
- [CI checks](#ci-checks)
- [Opening a pull request](#opening-a-pull-request)
- [Good first contributions](#good-first-contributions)
- [Releasing](#releasing)
- [Code of conduct](#code-of-conduct)

---

## Getting oriented

Before diving into the code, build some intuition for what the parser does:

1. Open the [live parser](https://maebli.github.io/m-bus-parser/) in your browser.
2. Go to `tests/rscada/test-frames/` in this repo and open any `.hex` file — for example:
   ```
   68 3C 3C 68 08 08 72 78 03 49 11 77 04 0E 16 0A 00 00 00 0C 78 78 03 49 11
   04 13 31 D4 00 00 42 6C 00 00 44 13 00 00 00 00 04 6D 0B 0B CD 13 02 27 00
   00 09 FD 0E 02 09 FD 0F 06 0F 00 01 75 13 D3 16
   ```
3. Paste it into tmbus and explore the decoded output.
4. Compare with what this parser produces:
   ```bash
   cargo run -p m-bus-parser-cli -- parse -d "68 3C 3C 68 ..." -t table
   ```

The corresponding `.xml` file in the same folder shows the expected parsed values.

Useful background reading:
- [M-Bus application layer documentation](https://m-bus.com/documentation-wired/06-application-layer) — good starting point even though it's not the latest spec
- [OMS specification](https://oms-group.org/en/open-metering-system/oms-specification) — the current reference for wMBus

---

## Setting up

**Prerequisites:** Rust toolchain, git.

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repo
git clone https://github.com/maebli/m-bus-parser.git
cd m-bus-parser

# Run all tests to confirm everything works
cargo test --all-features

# Build the CLI
cargo build -p m-bus-parser-cli

# Try parsing a frame
cargo run -p m-bus-parser-cli -- parse \
  -d "68 3C 3C 68 08 08 72 78 03 49 11 77 04 0E 16 0A 00 00 00 0C 78 78 03 49 11 04 13 31 D4 00 00 42 6C 00 00 44 13 00 00 00 00 04 6D 0B 0B CD 13 02 27 00 00 09 FD 0E 02 09 FD 0F 06 0F 00 01 75 13 D3 16" \
  -t table
```

Optional — rebuild the WebAssembly package:
```bash
cargo install wasm-pack
wasm-pack build wasm --target web --out-dir ../docs
```

---

## Project structure

```
m-bus-parser/
├── src/                        # Main library (serialize_mbus_data, output formats)
│   ├── mbus_data.rs            # All output format functions (table, json, yaml, csv, mermaid)
│   └── manufacturers.rs        # FLAG manufacturer code lookup (std-only)
├── crates/
│   ├── m-bus-core/             # Shared types (ManufacturerCode, FrameError, …)
│   ├── m-bus-application-layer/# Application layer parsing (DIF/VIF, data records)
│   ├── wired-mbus-link-layer/  # Wired frame parsing (long/short/control frames)
│   └── wireless-mbus-link-layer/ # Wireless frame parsing + Format A CRC stripping
├── cli/                        # CLI binary (m-bus-parser-cli)
├── wasm/                       # WebAssembly bindings (wasm-bindgen)
├── python/                     # Python bindings (PyO3)
├── docs/                       # GitHub Pages web app + compiled WASM
├── tests/
│   └── rscada/test-frames/     # Real-world .hex test frames + expected .xml output
└── examples/
    └── cortex-m/               # Embedded no_std example
```

The `src/` crate is the public API. It ties together the inner crates and provides the `serialize_mbus_data(data, format, key)` function used by the CLI, WASM, and Python bindings.

---

## Making changes

1. **Create a branch** from `main`:
   ```bash
   git checkout -b fix/your-fix
   # or
   git checkout -b feature/your-feature
   ```

2. **Make your changes.** Some common areas:
   - Adding a missing VIF/DIF code → `crates/m-bus-application-layer/`
   - Fixing a parsing bug → relevant inner crate
   - Adding a manufacturer → `src/manufacturers.rs` (add a new `match` arm)
   - Improving an output format → `src/mbus_data.rs`
   - Adding a CI field type → `crates/m-bus-application-layer/`

3. **Add a test** if applicable. For a new device frame, add a `.hex` file to `tests/rscada/test-frames/` and a corresponding test.

---

## Testing

```bash
# Run all tests
cargo test --all-features

# Run tests for a specific crate
cargo test -p m-bus-application-layer

# Run a specific test by name
cargo test test_mermaid_expected_output
```

Test frames in `tests/rscada/test-frames/` are real-world captures from devices. The `.hex` file is the raw frame; the `.xml` file is the expected parsed output.

---

## CI checks

The CI pipeline runs these checks — make sure they all pass before opening a PR:

```bash
# Formatting
cargo fmt --check

# Lints (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all-features

# no_std build (embedded target)
cargo build --target thumbv7m-none-eabi --no-default-features
```

To auto-fix formatting and simple clippy suggestions:
```bash
cargo fmt
cargo clippy --fix --all-features
```

---

## Opening a pull request

1. Push your branch and open a PR against `main`
2. Fill in the PR description — what changed and why
3. Make sure CI is green
4. A maintainer will review and merge

Commit message convention (loosely followed):
```
feat: add VIF code 0x3E
fix: handle missing CRC in Format A frames
docs: improve contributing guide
```

---

## Good first contributions

- **Add a missing manufacturer** — check `src/manufacturers.rs`. If you encounter a 3-letter code that returns `None`, look it up in the [DLMS FLAG ID directory](https://www.dlms.com/flag-id-directory/) and add it.
- **Add a test frame** — capture a real device frame, add it to `tests/rscada/test-frames/` with the expected output, and wire it into the test suite.
- **Implement a missing CI type** — the list of unimplemented CI fields is in the README. Pick one and implement it in `crates/m-bus-application-layer/`.
- **Improve error messages** — many parse errors are just `Unimplemented`; a more descriptive message helps users.

---

## Releasing

Releases bump the version in all four crates:

```bash
cargo release version patch --execute
cd python  && cargo release version patch --execute && cd ..
cd wasm    && cargo release version patch --execute && cd ..
cd cli     && cargo release version patch --execute && cd ..
```

Then push the tags and let CI publish to crates.io / npm / PyPI.

---

## Code of conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be kind and constructive.
