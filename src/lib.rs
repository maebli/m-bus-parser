//! m-bus-parse
//! * is a library for parsing M-Bus frames and user data.
//! * aims to be a modern, open source decoder for wired m-bus portocol decoder for EN 13757-2 physical and link layer, EN 13757-3 application layer of m-bus
//! * was implemented using the publicily available documentation available at https://m-bus.com/
//! # Example
//! ```
//! use m_bus_parser::{frames::{parse_frame, FrameType}, user_data::{parse_user_data, UserDataBlock}};
//!
//! let frame = vec![0x68, 0x0e, 0x0e, 0x68, 0x08, 0x00, 0x72, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x16, 0x16];
//! let frame_type = parse_frame(&frame);
//! assert_eq!(frame_type, FrameType::Short);
//!
//! let user_data = vec![0x08, 0x00, 0x72, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x16];
//! let user_data_block = parse_user_data(&user_data);
//! assert_eq!(user_data_block, UserDataBlock::Short);
//! ```
pub mod frames;
pub mod user_data;
