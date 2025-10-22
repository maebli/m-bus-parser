//! M-Bus Wired Frame Parser
//!
//! This crate implements the M-Bus wired protocol frame parsing (EN 13757-2).
//! It handles the physical and data link layer for wired M-Bus communication.
//!
//! Wired M-Bus uses a two-wire bus for communication between a master and
//! multiple slave meters.

#![cfg_attr(not(feature = "std"), no_std)]

mod frames;

// Re-export all frame types
pub use frames::*;
