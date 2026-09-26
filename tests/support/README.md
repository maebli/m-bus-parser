# rscada fixture checks

`tests/test.rs` exercises every `.hex` file in the three rscada directories:
73 ordinary frames, 20 error frames and 7 historically unsupported frames.
Directory traversal and missing XML files fail the test; fixed-data blocks and
unknown application blocks cannot silently bypass assertions.

For ordinary frames, the existing `.xml` files are the reference for header
fields, record order/count, function, storage number, tariff, device, unscaled
value, quantity, unit and decimal scale. The two fixed-data fixtures check both
counters and the packed medium/unit field. `rscada_units.rs` translates libmbus's
unit vocabulary; XML does not describe orthogonal VIFE modifiers, so it checks
the base quantity and unit prefix. ProductName is a libmbus database lookup and
has no corresponding parser field. `.norm.xml` contains scaled libmbus output
and is not the reference for the parser's unscaled record values.

Run both layouts (also covered by CI):

```sh
cargo test --test test
cargo test --test test --features plaintext-before-extension
```

The tests explicitly account for these existing limitations and reference
format differences rather than discarding failed records:

- `ELV-Elvaco-CMa10`, `THI_cma10` and `elv_temp_humid` require
  `plaintext-before-extension` to decode all 13 records. Without it, assert the
  first record, the precise error on the next record and iterator exhaustion.
- Records 4/5 of `ELS_Elster-F96-Plus` and 2/3 of `abb_f95` currently return
  `InvalidValueInformation`. Later records are still compared with XML.
- The error-status telegrams use CI=70, whose decoding currently returns
  `Unimplemented`. The malformed DIF/VIF fixtures assert the exact successful
  prefix, terminal error and exhaustion. `too_many_dife` currently accepts its
  eleven extensions; its decoded value and metadata are pinned explicitly.
- `manual_frame4/5/6` in unsupported-frames now parse as control frames, and
  `svm_f22_telegram2` parses as an opaque continuation record. Check successful
  results and payload preservation rather than expecting every file to fail.
- Signed BCD is numeric in this API but rendered as `Fxxxxx` in some XML files.
  XML rounds floating-point values, so numeric comparisons use a 1e-6 relative
  tolerance with a 1e-6 absolute floor. Wildcard date components are represented
  as enums instead of libmbus's numeric bit patterns. Whitespace-only customer
  IDs are trimmed by the XML reader and compared with the ten space bytes.
- `REL-Relay-Padpuls2` record 1 has a stale XML timestamp. The test uses the
  capture's `21 15 E9 17` bytes (2015-07-09T21:33:00) as the reference.
- Three XML medium labels contain a mojibake degree sign; both spellings map to
  the same warm-water medium.

The storage-bit and unit expectations follow the [M-Bus application-layer
layout](https://m-bus.com/documentation-wired/06-application-layer) and
[VIF tables](https://m-bus.com/documentation-wired/08-appendix). The expanded
comparisons exposed and now guard fixes to DIFE storage-bit placement, duration
units and extended energy units.
