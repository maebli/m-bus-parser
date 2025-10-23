//! Example: Parse a wireless M-Bus frame
//!
//! Run with: cargo run --example parse_wireless

use m_bus_wireless_frame::{Frame, FrameFormat, WirelessMBusData};

fn main() {
    // Example wireless M-Bus frame from OMS specification
    // This is a Format A frame from an Elster GmbH gas meter
    let frame_hex = "\
        2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
        013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a";

    let frame_data = hex::decode(frame_hex.replace(" ", ""))
        .expect("Invalid hex string");

    println!("Parsing wireless M-Bus frame...");
    println!("Frame length: {} bytes\n", frame_data.len());

    // Parse the frame
    match Frame::try_from(&frame_data[..]) {
        Ok(frame) => {
            println!("✓ Frame parsed successfully!");
            println!();
            println!("Frame Details:");
            println!("  Format: {:?}", frame.format);
            println!("  Length field: 0x{:02X} ({} bytes)", frame.length, frame.length);
            println!();

            println!("Control Field:");
            println!("  Raw: 0x{:02X}", frame.control.raw);
            println!("  Function: 0x{:X}", frame.control.function);
            println!("  Accessibility: {}", frame.control.accessibility);
            println!("  Synchronous: {}", frame.control.synchronous);
            println!();

            println!("Manufacturer:");
            println!("  Raw ID: 0x{:04X}", frame.manufacturer.raw);
            if let Some(code) = frame.manufacturer.code {
                println!("  Code: {}{}{}", code[0], code[1], code[2]);
            }
            println!();

            println!("Device Address:");
            println!("  Identification: 0x{:08X}", frame.address.identification);
            println!("  Version: {}", frame.address.version);
            println!("  Device Type: 0x{:02X}", frame.address.device_type);
            println!();

            println!("Application Layer:");
            println!("  CI Field: 0x{:02X}", frame.ci_field);
            println!("  User Data: {} bytes", frame.data.len());
            println!();

            // Show first few bytes of user data
            if !frame.data.is_empty() {
                print!("  First bytes: ");
                for (i, byte) in frame.data.iter().take(16).enumerate() {
                    print!("{:02X} ", byte);
                    if i == 7 {
                        print!(" ");
                    }
                }
                if frame.data.len() > 16 {
                    print!("...");
                }
                println!("\n");
            }

            // Also demonstrate WirelessMBusData wrapper
            println!("Using WirelessMBusData wrapper:");
            let wmbus_data = WirelessMBusData::try_from(&frame_data[..])
                .expect("Should parse");

            if let Some(mfr) = wmbus_data.manufacturer_code() {
                println!("  Manufacturer code: {}", mfr);
            }

            let (ci, app_data) = wmbus_data.application_data();
            println!("  CI: 0x{:02X}, App data: {} bytes", ci, app_data.len());
        }
        Err(e) => {
            eprintln!("✗ Error parsing frame: {:?}", e);
        }
    }
}
