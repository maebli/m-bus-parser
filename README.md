# m-bus-parser
A modern, open source parser for wired m-bus portocol according to EN 13757-2 (contains physical and link layer specificatioin) and EN 13757-3 (contains application layer specification).

# Important links

-  https://m-bus.com/documentation
-  https://en.wikipedia.org/wiki/Parsing
-  Similar task can be used as inspiration: https://github.com/seanmonstar/httparse
-  Implementation in C https://github.com/rscada/libmbus

# Goals

- Written in rust with no_std 
- Releases for x86, x86_64,WebAssembly, ARM Architectures, RISC-V
- Optimize code size over speed
- Follow the Rust API Guideline https://rust-lang.github.io/api-guidelines/
- Keep it simple
- zero copy

# Example of function 

Examples taken from https://m-bus.com/documentation-wired/06-application-layer:

1. Set the slave to primary address 8 without changing anything else:

```INPUT: 68 06 06 68 | 53 FE 51 | 01 7A 08 | 25 16```

Parsing the frame using the library (the data is not yet parsable with the lib):

```rust
     
    use m_bus_parser::frames::{parse_frame, Address, FrameType, Function};

    let example = vec![ 
        0x68, 0x06, 0x06, 0x68, 
        0x53, 0xFE, 0x51, 
        0x01, 0x7A, 0x08, 
        0x25, 0x16,
    ];

    let frame = parse_frame(&example).unwrap();

    if let FrameType::ControlFrame { function, address, data } = frame {
        assert_eq!(address, Address::Broadcast { reply_required: true });
        assert_eq!(function, Function::SndUd { fcb: (false)});
        assert_eq!(data, &[0x51,0x01, 0x7A, 0x08]);
    }

```