//! M-Bus Application Layer Parser
//!
//! This crate implements the M-Bus application layer (EN 13757-3) parsing.
//! It is shared by both wired M-Bus (EN 13757-2) and wireless M-Bus (EN 13757-4).
//!
//! The application layer defines how meter data is structured and encoded,
//! independent of the physical transport mechanism (wired or wireless).

#![cfg_attr(not(feature = "std"), no_std)]

pub mod user_data;

// Re-export commonly used types
pub use user_data::{
    ApplicationLayerError,
    UserDataBlock,
    DataRecords,
    FixedDataHeader,
    ControlInformation,
};
