# rscada fixture checks

`tests/test.rs` checks all 100 captures: headers, records, fixed counters and
explicit error outcomes. XML supplies expected values; `rscada_units.rs` maps
unit names. Fixture exceptions are documented beside their assertions.

```sh
cargo test --test test
cargo test --test test --features plaintext-before-extension
```

Product names and orthogonal VIFE modifiers have no XML-to-parser comparison.
`.norm.xml` contains scaled values; these tests use the unscaled `.xml` files.
