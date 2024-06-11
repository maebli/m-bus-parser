//! Brief summary
//! * is a library for parsing M-Bus frames and user data.
//! * aims to be a modern, open source decoder for wired m-bus portocol decoder for EN 13757-2 physical and link layer, EN 13757-3 application layer of m-bus
//! * was implemented using the publicily available documentation available at <https://m-bus.com/>
//! # Example
//! ```rust
//! use m_bus_parser::frames::{ Address, Frame, Function};
//!
//! let example = vec![
//!     0x68, 0x4D, 0x4D, 0x68,
//!     0x08, 0x01, 0x72, 0x01,
//!     0x00, 0x00,0x00, 0x96,
//!     0x15, 0x01, 0x00, 0x18,
//!     0x00, 0x00, 0x00, 0x0C,
//!     0x78, 0x56, 0x00, 0x00,
//!     0x00, 0x01, 0xFD, 0x1B,
//!     0x00, 0x02, 0xFC, 0x03,
//!     0x48, 0x52, 0x25, 0x74,
//!     0x44, 0x0D, 0x22, 0xFC,
//!     0x03, 0x48, 0x52, 0x25,
//!     0x74, 0xF1, 0x0C, 0x12,
//!     0xFC, 0x03, 0x48, 0x52,
//!     0x25, 0x74, 0x63, 0x11,
//!     0x02, 0x65, 0xB4, 0x09,
//!     0x22, 0x65, 0x86, 0x09,
//!     0x12, 0x65, 0xB7, 0x09,
//!     0x01, 0x72, 0x00, 0x72,
//!     0x65, 0x00, 0x00, 0xB2,
//!     0x01, 0x65, 0x00, 0x00,
//!     0x1F, 0xB3, 0x16,
//! ];
//!
//! // Parse the frame
//! let frame = Frame::try_from(example.as_slice()).unwrap();
//!
//! if let Frame::LongFrame { function, address, data :_} = frame {
//!     assert_eq!(function, Function::RspUd{acd: false, dfc:false});
//!     assert_eq!(address, Address::Primary(1));
//! }
//!
//! // Alternatively, parse the frame and user data in one go
//! let mbus_data = m_bus_parser::MbusData::try_from(example.as_slice()).unwrap();
//!
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use frames::FrameError;
use user_data::ApplicationLayerError;

#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
extern crate core;

pub mod frames;
pub mod user_data;

#[derive(Debug)]
pub struct MbusData<'a> {
    pub frame: frames::Frame<'a>,
    pub user_data: Option<user_data::UserDataBlock<'a>>,
    pub data_records: Option<user_data::DataRecords>,
}

#[derive(Debug)]
pub enum MbusError {
    FrameError(FrameError),
    ApplicationLayerError(ApplicationLayerError),
}

impl From<FrameError> for MbusError {
    fn from(error: FrameError) -> MbusError {
        MbusError::FrameError(error)
    }
}

impl From<ApplicationLayerError> for MbusError {
    fn from(error: ApplicationLayerError) -> MbusError {
        MbusError::ApplicationLayerError(error)
    }
}

impl<'a> TryFrom<&'a [u8]> for MbusData<'a> {
    type Error = MbusError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let frame = frames::Frame::try_from(data)?;
        let mut user_data = None;
        let mut data_records = None;
        match &frame {
            frames::Frame::LongFrame { data, .. } => {
                if let Ok(x) = user_data::UserDataBlock::try_from(*data) {
                    user_data = Some(x);
                    if let Ok(user_data::UserDataBlock::VariableDataStructure {
                        fixed_data_header: _,
                        variable_data_block,
                    }) = user_data::UserDataBlock::try_from(*data)
                    {
                        data_records = user_data::DataRecords::try_from(variable_data_block).ok();
                    }
                }
            }
            frames::Frame::SingleCharacter { .. } => (),
            frames::Frame::ShortFrame { .. } => (),
            frames::Frame::ControlFrame { .. } => (),
        };

        Ok(MbusData {
            frame,
            user_data,
            data_records,
        })
    }
}
