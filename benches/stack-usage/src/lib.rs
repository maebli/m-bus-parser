#![no_std]

//! Build-only fixture used by `measure.py`.

#[path = "../../full_parse_fixture.rs"]
mod full_parse_fixture;

pub use full_parse_fixture::{parse_full_wired_frame, FULL_FRAME};
