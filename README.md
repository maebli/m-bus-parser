# m-bus-decoder
A modern, open source decoder for wired m-bus portocol decoder for EN 13757-2 physical and link layer, EN 13757-3 application layer of m-bus

# Important links

-  https://m-bus.com/documentation


# Goals

- Use no or little libraries other than standard library
- Releases for x86, x86_64,WebAssembly, ARM Architectures, RISC-V
- Optimize code size over speed
- Follow the Rust API Guideline https://rust-lang.github.io/api-guidelines/ e.g.  Types eagerly implement common traits (C-COMMON-TRAITS) Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, Default
- Keep it simple

# Example of function 


Examples (taken from https://m-bus.com/documentation-wired/06-application-layer):

1. Set the slave to primary address 8 without changing anything else:
INPUT: 68 06 06 68 | 53 FE 51 | 01 7A 08 | 25 16

2. Set the complete identification of the slave (ID=01020304, Man=4024h (PAD), Gen=1, Med=4 (Heat):
INPUT: 68 0D 0D 68 | 53 FE 51 | 07 79 04 03 02 01 24 40 01 04 | 95 16 §

3. Set identification number of the slave to "12345678" and the 8 digit BCD-Counter (unit 1 kWh) to 107 kWh.
INPUT:68 0F 0F 68 | 53 FE 51| 0C 79 78 56 34 12 | 0C 06 07 01 00 00 | 55 16


# TODO

- Specifiy information format(s) of output e.g. JSON, YAML, TOML, csv
- See if there is a suitable design pattern that fits the task and aims
git 