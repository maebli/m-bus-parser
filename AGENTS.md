# AGENTS Guide for OpenAI Codex

This file explains how AI agents, including OpenAI Codex, should work with this repository.

## Project Overview

- The Rust library lives under `src/` and contains the main parsing logic.
- The command line interface is under `cli/`.
- WebAssembly bindings are in `wasm/`.
- Python bindings are located in `python/`.
- Tests live under `tests/`.
- Static resources and documentation in `resources/` and `docs/` should not be modified automatically.

## Coding Conventions

- Write all new code in **Rust** and follow the existing formatting style.
- Use meaningful variable and function names and add comments for complex logic.

## Programmatic Checks

Before committing code, run the same checks that the CI runs:

```bash
# Format check for each crate
cargo fmt --all -- --check
# Library tests
cargo test --verbose
cargo test --verbose -F std
cargo test --verbose -F plaintext-before-extension
# Lint
rustup component add clippy
cargo clippy -- -D warnings

# CLI crate checks
cd cli && cargo fmt -- --check && cargo build --verbose && cargo test --verbose && cargo clippy -- -D warnings && cd ..

# WASM crate checks
cd wasm && cargo fmt -- --check && cargo build --verbose && cargo test --verbose && cargo clippy -- -D warnings && cd ..
```

For Python bindings see `python/README_python.md`. Building wheels uses `maturin`, as shown in `.github/workflows/python.yml`.

## Pull Request Guidelines

- Keep PRs focused and provide a clear description of the changes.
- Ensure all checks above pass before submitting a PR.
