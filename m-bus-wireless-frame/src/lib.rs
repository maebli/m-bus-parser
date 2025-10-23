//! M-Bus Wireless Frame Parser
//!
//! This crate implements the M-Bus wireless protocol frame parsing (EN 13757-4).
//! It handles the radio link layer for wireless M-Bus communication.
//!
//! Wireless M-Bus (WM-Bus) uses radio communication at 868 MHz (EU) or 433 MHz
//! for wireless meter reading.
//!
//! # Frame Structure
//!
//! Wireless M-Bus supports two frame formats:
//! - **Format A**: IEC 60870-5-1 format type FT3
//! - **Format B**: Modified format with different length field
//!
//! # Transmission Modes
//!
//! - **S-mode**: Stationary meters (walk-by reading)
//! - **T-mode**: Frequent transmission (fixed network)
//! - **C-mode**: Compact, low power
//! - **R-mode**: Frequent bidirectional
//! - **N-mode**: Narrowband
//! - **F-mode**: Frequent transmission, longer range

#![cfg_attr(not(feature = "std"), no_std)]

mod crc;
mod frame;
mod wireless_data;

pub use frame::*;
pub use crc::calculate_crc16;
pub use wireless_data::WirelessMBusData;
