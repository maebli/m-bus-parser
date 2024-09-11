# Python bindings [WIP]  

Rust lib aims to be accessible from a Python module. This is done by using the `PyO3` crate.

## Development

- `pipx install maturin` to install `maturin` globally.
- `maturin develop` to build the Rust lib and create a Python module in the current environment.
- Currently this creates a release in the target directory that is one hierachy up. This is not ideal and will be fixed in the future.
- after calling the maturin develop command cd one up and `pip install target/..` and then run `python` and `from pymbusparser import pymbus` to test the module.
- to test inside `REPL` run `python` and then `import p

## Publishing

- `maturin publish`
- username = __token__
- passwrod = api key from pypi

## Usage

`pip install pymbusparser`

```python
from pymbusparser import m_bus_parse,parse_application_layer

print(m_bus_parse("68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16","table"))

print(m_bus_parse("68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16","json"))

print(m_bus_parse("68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E )16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16","yml"))

# note this is not as pretty as the function before, still TODO, currently it just outputs structs of RUST in string
print(parse_application_layer("2f2f0413fce0f5052f2f2f2f2f2f2f2f"))

```