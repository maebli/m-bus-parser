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

    /// Get the application layer data
    ///
    /// This returns the CI-field and user data that can be parsed
    /// using the `m-bus-application-layer` crate.
    pub fn application_data(&self) -> (u8, &'a [u8]) {
        (self.frame.ci_field, self.frame.data)
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
