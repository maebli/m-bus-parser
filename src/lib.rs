//! M-Bus Parser - Unified wired and wireless M-Bus parsing
//!
//! This library supports parsing of M-Bus (Meter Bus) protocol data for both:
//! - **Wired M-Bus** (EN 13757-2) - always available
//! - **Wireless M-Bus** (EN 13757-4) - available with `wireless` feature
//!
//! The M-Bus protocol is a European standard for remote reading of water, gas,
//! electricity, and heating meters.
//!
//! # Architecture
//!
//! This crate is organized into separate components:
//! - **Application Layer** (`user_data`, `mbus_data`) - Shared by both protocols
//! - **Wired Frame Layer** (`frames`) - Wired M-Bus specific
//! - **Wireless Frame Layer** (`wireless`) - Wireless M-Bus specific (opt-in)
//!
//! # Example - Wired M-Bus
//!
//! ```rust
//! use m_bus_parser::frames::{Address, Frame, Function};
//! use m_bus_parser::user_data::{DataRecords, UserDataBlock};
//! use m_bus_parser::mbus_data::MbusData;
//!
//! fn try_parse() -> Result<(), m_bus_parser::MbusError> {
//!     let example = vec![
//!         0x68, 0x4D, 0x4D, 0x68, 0x08, 0x01, 0x72, 0x01,
//!         0x00, 0x00, 0x00, 0x96, 0x15, 0x01, 0x00, 0x18,
//!         0x00, 0x00, 0x00, 0x0C, 0x78, 0x56, 0x00, 0x00,
//!         0x00, 0x01, 0xFD, 0x1B, 0x00, 0x02, 0xFC, 0x03,
//!         0x48, 0x52, 0x25, 0x74, 0x44, 0x0D, 0x22, 0xFC,
//!         0x03, 0x48, 0x52, 0x25, 0x74, 0xF1, 0x0C, 0x12,
//!         0xFC, 0x03, 0x48, 0x52, 0x25, 0x74, 0x63, 0x11,
//!         0x02, 0x65, 0xB4, 0x09, 0x22, 0x65, 0x86, 0x09,
//!         0x12, 0x65, 0xB7, 0x09, 0x01, 0x72, 0x00, 0x72,
//!         0x65, 0x00, 0x00, 0xB2, 0x01, 0x65, 0x00, 0x00,
//!         0x1F, 0xB3, 0x16
//!     ];
//!
//!     // Parse the frame
//!     let frame = Frame::try_from(example.as_slice())?;
//!
//!     if let Frame::LongFrame { function, address, data } = frame {
//!         assert_eq!(function, Function::RspUd { acd: false, dfc: false });
//!         assert_eq!(address, Address::Primary(1));
//!         if let Ok(UserDataBlock::VariableDataStructure { fixed_data_header, variable_data_block }) = UserDataBlock::try_from(data) {
//!             let data_records = DataRecords::from((variable_data_block, &fixed_data_header));
//!             println!("data_records: {:#?}", data_records.collect::<Result<Vec<_>, _>>()?);
//!         }
//!     }
//!
//!     // Parse everything at once
//!     let parsed_data = MbusData::try_from(example.as_slice())?;
//!     println!("parsed_data: {:#?}", parsed_data);
//!     Ok(())
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

// Re-export application layer user_data module (shared by wired and wireless)
pub use m_bus_application_layer::user_data;

// High-level data aggregation (combines frames + application layer)
pub mod mbus_data;

// Wired M-Bus frame parsing (always available)
pub mod frames {
    //! Wired M-Bus frame parsing (EN 13757-2)
    pub use m_bus_wired_frame::*;
}

// Wireless M-Bus frame parsing (feature-gated)
#[cfg(feature = "wireless")]
pub mod wireless {
    //! Wireless M-Bus frame parsing (EN 13757-4)
    pub use m_bus_wireless_frame::*;
}

// Re-export commonly used types
pub use m_bus_application_layer::{
    ApplicationLayerError,
    UserDataBlock,
    DataRecords,
};

pub use mbus_data::MbusData;

pub use m_bus_wired_frame::FrameError;

#[cfg(feature = "std")]
pub use mbus_data::serialize_mbus_data;

// Convenience type aliases
pub use frames::Frame as WiredFrame;

#[cfg(feature = "wireless")]
pub use wireless::Frame as WirelessFrame;

/// Unified error type for M-Bus parsing
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum MbusError {
    /// Error in wired frame parsing
    FrameError(FrameError),
    /// Error in application layer parsing
    ApplicationLayerError(ApplicationLayerError),
    /// Error in data record parsing
    DataRecordError(user_data::variable_user_data::DataRecordError),
    /// Error in wireless frame parsing (when wireless feature is enabled)
    #[cfg(feature = "wireless")]
    WirelessFrameError(m_bus_wireless_frame::FrameError),
}

#[cfg(feature = "std")]
impl std::fmt::Display for MbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MbusError::FrameError(e) => write!(f, "{e}"),
            MbusError::ApplicationLayerError(e) => write!(f, "{e}"),
            MbusError::DataRecordError(e) => write!(f, "{e}"),
            #[cfg(feature = "wireless")]
            MbusError::WirelessFrameError(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MbusError {}

impl From<FrameError> for MbusError {
    fn from(error: FrameError) -> Self {
        Self::FrameError(error)
    }
}

impl From<ApplicationLayerError> for MbusError {
    fn from(error: ApplicationLayerError) -> Self {
        Self::ApplicationLayerError(error)
    }
}

impl From<user_data::variable_user_data::DataRecordError> for MbusError {
    fn from(error: user_data::variable_user_data::DataRecordError) -> Self {
        Self::DataRecordError(error)
    }
}

#[cfg(feature = "wireless")]
impl From<m_bus_wireless_frame::FrameError> for MbusError {
    fn from(error: m_bus_wireless_frame::FrameError) -> Self {
        Self::WirelessFrameError(error)
    }
}
