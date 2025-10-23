//! High-level wireless M-Bus data structures
//!
//! This module provides convenient access to wireless M-Bus frames
//! and their application layer data.

use crate::frame::{Frame, FrameError};

/// High-level wireless M-Bus data structure
///
/// This combines the wireless frame with a reference to extract
/// application layer data (which should be parsed using `m-bus-application-layer`).
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WirelessMBusData<'a> {
    /// The parsed wireless frame
    pub frame: Frame<'a>,
}

impl<'a> WirelessMBusData<'a> {
    /// Parse wireless M-Bus data from raw bytes
    pub fn try_from_bytes(data: &'a [u8]) -> Result<Self, FrameError> {
        let frame = Frame::try_from(data)?;
        Ok(Self { frame })
    }

    /// Get the application layer data (raw, including CRC bytes)
    ///
    /// This returns the CI-field and raw user data (with interleaved CRC bytes).
    ///
    /// **Warning:** The returned data includes CRC bytes! Every 16 bytes of data
    /// is followed by 2 CRC bytes.
    ///
    /// For clean data without CRC bytes, use `application_data_clean()` (requires `std` feature).
    ///
    /// The CI-field and user data can be parsed using the `m-bus-application-layer` crate.
    pub fn application_data_raw(&self) -> (u8, &'a [u8]) {
        (self.frame.ci_field, self.frame.data)
    }

    /// Get the application layer data (clean, CRC bytes removed)
    ///
    /// This returns the CI-field and clean user data without CRC bytes.
    /// The data is ready to be parsed by the `m-bus-application-layer` crate.
    ///
    /// **Requires `std` feature** - allocates a `Vec<u8>` to remove CRC bytes.
    #[cfg(feature = "std")]
    pub fn application_data_clean(&self) -> (u8, Vec<u8>) {
        (self.frame.ci_field, self.frame.user_data_clean())
    }

    /// Get the application layer data (deprecated, use application_data_clean or application_data_raw)
    ///
    /// **Deprecated:** This method returns raw data with CRC bytes.
    /// Use `application_data_clean()` for clean data or `application_data_raw()` for explicit raw access.
    #[deprecated(since = "0.1.0", note = "Use application_data_clean() or application_data_raw()")]
    pub fn application_data(&self) -> (u8, &'a [u8]) {
        self.application_data_raw()
    }

    /// Get manufacturer code as string (if valid)
    pub fn manufacturer_code(&self) -> Option<String> {
        self.frame.manufacturer.code.map(|chars| {
            chars.iter().collect()
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for WirelessMBusData<'a> {
    type Error = FrameError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(data)
    }
}
