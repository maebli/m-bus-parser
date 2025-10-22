//! M-Bus Wireless Frame Parser (WORK IN PROGRESS)
//!
//! This crate will implement the M-Bus wireless protocol frame parsing (EN 13757-4).
//! It handles the radio link layer for wireless M-Bus communication.
//!
//! Wireless M-Bus (WM-Bus) uses radio communication at 868 MHz (EU) or 433 MHz
//! for wireless meter reading.
//!
//! **NOTE**: This crate is currently a stub and does not yet implement wireless M-Bus parsing.
//! It is part of the planned architecture to support both wired and wireless M-Bus.

#![cfg_attr(not(feature = "std"), no_std)]

/// Placeholder for wireless M-Bus frame types
///
/// This will be implemented in a future version to support:
/// - Format A and Format B frames
/// - S, T, C, and R transmission modes
/// - AES-128 encryption (Mode 5/7)
/// - CRC-16 validation
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Frame {
    /// Placeholder variant - not yet implemented
    Unimplemented,
}

/// Placeholder error type for wireless M-Bus frame parsing
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum FrameError {
    /// Feature not yet implemented
    NotImplemented,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wireless M-Bus parsing is not yet implemented")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FrameError {}
