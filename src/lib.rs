//! Brief summary
//! * is a library for parsing M-Bus frames and user data.
//! * aims to be a modern, open source decoder for wired m-bus protocol decoder for EN 13757-2 physical and link layer, EN 13757-3 application layer of m-bus
//! * was implemented using the publicly available documentation available at <https://m-bus.com/>
//! # Example
//! ```rust
//! use m_bus_parser::frames::{ Address, Frame, Function };
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

use frames::FrameError;
use user_data::ApplicationLayerError;

pub mod frames;
pub mod mbus_data;
pub mod user_data;

#[cfg(feature = "std")]
pub use mbus_data::serialize_mbus_data;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum MbusError {
    FrameError(FrameError),
    ApplicationLayerError(ApplicationLayerError),
    DataRecordError(user_data::variable_user_data::DataRecordError),
}

#[cfg(feature = "std")]
impl std::fmt::Display for MbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MbusError::FrameError(e) => write!(f, "{}", e),
            MbusError::ApplicationLayerError(e) => write!(f, "{}", e),
            MbusError::DataRecordError(e) => write!(f, "{}", e),
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
