//! Example: Parse both wired and wireless M-Bus frames
//!
//! This example demonstrates parsing both protocols with the unified API.
//!
//! Run with: cargo run --example parse_both_protocols --features wireless,std

fn main() {
    println!("=== M-Bus Parser: Wired and Wireless Support ===\n");

    // Parse wired M-Bus frame
    parse_wired_frame();

    println!("\n{}\n", "=".repeat(60));

    // Parse wireless M-Bus frame (if wireless feature is enabled)
    #[cfg(feature = "wireless")]
    parse_wireless_frame();

    #[cfg(not(feature = "wireless"))]
    println!("Wireless M-Bus support not enabled. Run with --features wireless");
}

fn parse_wired_frame() {
    use m_bus_parser::frames::{Frame, Address, Function};

    println!("Parsing WIRED M-Bus frame:");
    println!("-".repeat(60));

    // Example wired M-Bus frame (LongFrame)
    let wired_data = hex::decode(
        "684d4d680801720100000096150100180000000c78560000000\
         1fd1b0002fc03485225744 40d22fc03485225744f10c12fc034852257\
         4463110265b409226586091265b70901720072650000b2016500001fb316"
    ).expect("Invalid hex");

    match Frame::try_from(&wired_data[..]) {
        Ok(frame) => {
            println!("✓ Wired frame parsed successfully!");

            if let m_bus_parser::frames::Frame::LongFrame { function, address, data } = frame {
                println!("  Frame type: LongFrame");
                println!("  Function: {:?}", function);
                println!("  Address: {:?}", address);
                println!("  Data length: {} bytes", data.len());

                // Parse application layer
                if let Ok(user_data) = m_bus_parser::user_data::UserDataBlock::try_from(data) {
                    println!("  Application layer: {:?}", user_data);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {:?}", e);
        }
    }
}

#[cfg(feature = "wireless")]
fn parse_wireless_frame() {
    use m_bus_parser::wireless::{Frame, WirelessMBusData};

    println!("Parsing WIRELESS M-Bus frame:");
    println!("-".repeat(60));

    // Example wireless M-Bus frame (Format A)
    let wireless_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    match Frame::try_from(&wireless_data[..]) {
        Ok(frame) => {
            println!("✓ Wireless frame parsed successfully!");
            println!("  Format: {:?}", frame.format);
            println!("  Manufacturer: {:?}", frame.manufacturer.code);
            println!("  Device ID: 0x{:08X}", frame.address.identification);
            println!("  CI Field: 0x{:02X}", frame.ci_field);
            println!("  Data length: {} bytes", frame.data.len());

            // Application layer data can be parsed the same way as wired
            let (ci_field, app_data) = (frame.ci_field, frame.data);
            println!("  App layer: CI=0x{:02X}, {} bytes", ci_field, app_data.len());
        }
        Err(e) => {
            eprintln!("✗ Error: {:?}", e);
        }
    }
}
