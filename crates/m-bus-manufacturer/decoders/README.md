# Built-in decoders

No vendors are registered yet. Add `<code>_<slug>.rs` here when a request includes
an understood layout and test data. The build script discovers files in sorted order.
Copy the implementation and `tests` module from `../examples/decoder.rs`, replace its
synthetic descriptor with a cited vendor specification, and omit the example's `main`.

Each module must export a unit `Decoder` implementing `ManufacturerDecoder` and, under
`cfg(test)`, a `tests::FIXTURES` table visible to its parent (`pub(super)`). Imports are
ordinary crate imports; `use m_bus_manufacturer::...` in the example becomes `use crate::...`.
Generated tests run those fixtures and check filename/manufacturer agreement. Registry
tests reject missing metadata, invalid selectors and overlaps. The sample is not a
registered decoder and does not claim any actual manufacturer support.
