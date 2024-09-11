# Python bindings [WIP]  

Rust lib aims to be accessible from a Python module. This is done by using the `PyO3` crate.

## Development

- `pipx install maturin` to install `maturin` globally.
- `maturin develop` to build the Rust lib and create a Python module in the current environment.
- Currently this creates a release in the target directory that is one hierachy up. This is not ideal and will be fixed in the future.
- after calling the maturin develop command cd one up and `pip install target/..` and then run `python` and `from pymbusparser import pymbus` to test the module.
- to test inside `REPL` run `python` and then `import p