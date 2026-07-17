# M-Bus application layer

This crate parses EN 13757-3 application-layer data without requiring a wired
or wireless link-layer frame.

Use `parse_application_layer` when the input begins with a CI field, or
`parse_data_records` when the CI and transport headers have already been
removed:

```rust
use m_bus_application_layer::parse_data_records;

let data = [0x03, 0x13, 0x15, 0x31, 0x00];
for record in parse_data_records(&data) {
    let record = record?;
    println!("value: {:?}", record.value());
    println!("value information: {:?}", record.value_information());
}
```

Run the complete data-record example from the workspace root:

```console
cargo run -p m-bus-application-layer --example parse_data_records
```

The parser is allocation-free and supports `no_std`; formatting and standard
error traits are enabled by the optional `std` feature.
