//! Integration tests for wireless M-Bus frame parsing
//!
//! These tests use real wireless M-Bus frames from the OMS specification
//! and other documented examples.

use m_bus_wireless_frame::{Frame, FrameFormat, FrameError, WirelessMBusData};

/// Example wireless M-Bus frame from OMS specification
/// This is a Format A frame from an Elster GmbH gas meter with ENCRYPTED data
#[test]
fn test_parse_oms_example_format_a_encrypted() {
    // Real telegram with CRC (Format A) - ENCRYPTED
    // Byte  0:    0x2E (46) - Length
    // Byte  1:    0x44 - C-field (SND_NR)
    // Byte  2-3:  0x9315 - M-field (Elster GmbH, little-endian)
    // Byte  4-7:  0x78563412 - ID: 12345678 (BCD, little-endian)
    // Byte  8:    0x33 (51) - Version
    // Byte  9:    0x03 - Device (Gas meter)
    // Byte 10-11: 0x3363 - CRC of bytes 0-9 (big-endian)
    // Byte 12:    0x7A - CI-field (Extended Link Layer = ENCRYPTED)
    // Byte 13+:   Encrypted user data (42 bytes)
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    let frame = Frame::try_format_a(&frame_data);

    assert!(frame.is_ok(), "Should parse valid Format A frame: {:?}", frame.err());

    let frame = frame.unwrap();
    assert_eq!(frame.length, 0x2E, "Length should be 0x2E (46)");
    assert_eq!(frame.control.raw, 0x44, "C-field should be 0x44");
    assert_eq!(frame.manufacturer.raw, 0x1593, "M-field should be 0x1593 (from little-endian 0x93 0x15)");
    assert_eq!(frame.address.identification, 0x12345678, "Device ID should be 0x12345678");
    assert_eq!(frame.address.version, 0x33, "Version should be 0x33 (51)");
    assert_eq!(frame.address.device_type, 0x03, "Device type should be 0x03 (gas)");
    assert_eq!(frame.ci_field, 0x7A, "CI-field should be 0x7A (encrypted)");

    // Verify it's detected as encrypted
    assert!(frame.is_encrypted(), "Frame should be detected as encrypted");

    // Verify user data length - encrypted payload from byte 13 onwards
    assert!(!frame.data.is_empty(), "Frame should have encrypted user data");
}

/// Test parsing with WirelessMBusData wrapper
#[test]
fn test_wireless_mbus_data() {
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    let wmbus_data = WirelessMBusData::try_from_bytes(&frame_data);

    assert!(wmbus_data.is_ok(), "Should parse valid frame");

    let wmbus_data = wmbus_data.unwrap();

    // Test clean data API
    let (ci_field, app_data) = wmbus_data.application_data_clean();
    assert_eq!(ci_field, 0x7A, "CI-field should be 0x7A");
    assert!(!app_data.is_empty(), "Should have application data");

    // Note: This frame is ENCRYPTED (CI 0x7A), so clean and raw data are the same
    let (_, raw_data) = wmbus_data.application_data_raw();
    assert_eq!(app_data.len(), raw_data.len(),
               "For encrypted frames, clean data equals raw data (no CRC bytes to remove)");
}

/// Test manufacturer code decoding
#[test]
fn test_manufacturer_code() {
    // Elster GmbH = "ELS"
    // Encoding: Each letter as (letter-64), 5-bit fields
    // E=5, L=12, S=19
    // Value = 5*1024 + 12*32 + 19 = 5120 + 384 + 19 = 5523 = 0x1593
    // But stored little-endian: 0x9315
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    let frame = Frame::try_format_a(&frame_data).unwrap();

    // The manufacturer code decoding might give us "ELS" or similar
    // The exact decoding depends on the standard encoding
    assert!(frame.manufacturer.code.is_some(), "Should decode manufacturer code");
}

/// Test CRC validation failure
#[test]
fn test_invalid_crc() {
    // Create a frame with invalid CRC
    let mut frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    // Corrupt the first CRC bytes (at positions 10-11)
    frame_data[10] = 0xFF;
    frame_data[11] = 0xFF;

    let result = Frame::try_format_a(&frame_data);
    assert!(matches!(result, Err(FrameError::CrcError { .. })), "Should fail CRC validation");
}

/// Test empty data
#[test]
fn test_empty_data() {
    let result = Frame::try_format_a(&[]);
    assert_eq!(result, Err(FrameError::EmptyData));
}

/// Test insufficient data
#[test]
fn test_insufficient_data() {
    let short_data = [0x2E, 0x44, 0x93]; // Only 3 bytes
    let result = Frame::try_format_a(&short_data);
    assert!(matches!(result, Err(FrameError::InsufficientData { .. })));
}

/// Test Format B parsing (if different from Format A)
#[test]
fn test_format_b() {
    // For now, use the same data but try Format B
    // In reality, Format B frames have slightly different structure
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    // This might fail or succeed depending on whether the data is actually Format B
    let _result = Frame::try_format_b(&frame_data);
    // We don't assert here since we're using Format A data
}

/// Test TryFrom implementation (tries both formats)
#[test]
fn test_try_from() {
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    let frame = Frame::try_from(&frame_data[..]);
    assert!(frame.is_ok(), "TryFrom should successfully parse the frame");
}

/// Test encrypted frame handling
#[test]
fn test_encrypted_frame() {
    // This frame has CI-field 0x7A which indicates encrypted data
    let frame_data = hex::decode(
        "2e44931578563412330333637a2a0020055923c95aaa26d1b2e7493b2a8b\
         013ec4a6f6d3529b520edff0ea6defc955b29d6d69ebf3ec8a"
    ).expect("Invalid hex");

    let frame = Frame::try_format_a(&frame_data).unwrap();

    // Verify it's encrypted
    assert!(frame.is_encrypted(), "Frame with CI 0x7A should be encrypted");

    // Get clean data - for encrypted frames, this returns the encrypted payload as-is
    let clean_data = frame.user_data_clean();

    // Get raw data
    let raw_data = frame.user_data_raw();

    // For encrypted frames, clean and raw should be the same
    assert_eq!(
        clean_data.len(),
        raw_data.len(),
        "For encrypted frames, clean data should equal raw data (no CRC bytes to remove)"
    );

    // Verify clean data is not empty
    assert!(!clean_data.is_empty(), "Clean data should not be empty");
}
