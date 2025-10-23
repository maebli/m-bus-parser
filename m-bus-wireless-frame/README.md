# m-bus-wireless-frame

Wireless M-Bus (WM-Bus) frame parser implementing EN 13757-4.

## Features

- ✅ Format A and Format B frame parsing
- ✅ CRC-16/EN-13757 validation
- ✅ Manufacturer ID decoding
- ✅ Device address parsing
- ✅ Control field parsing
- ✅ Transmission mode types (S, T, C, R, N, F)
- ✅ `no_std` compatible
- ✅ Optional serde support

## Usage

```rust
use m_bus_wireless_frame::{Frame, WirelessMBusData};

// Parse a wireless M-Bus frame
let frame_data = /* your wireless M-Bus frame bytes */;

// Try parsing (attempts Format A first, then Format B)
let frame = Frame::try_from(frame_data)?;

// Or specify format explicitly
let frame = Frame::try_format_a(frame_data)?;

// Access frame fields
println!("Manufacturer: {:?}", frame.manufacturer.code);
println!("Device ID: {:08X}", frame.address.identification);
println!("CI Field: 0x{:02X}", frame.ci_field);

// Check if frame is encrypted
if frame.is_encrypted() {
    println!("Frame contains encrypted data");
}

// Get clean application layer data - requires std feature
// For encrypted frames: returns encrypted payload
// For unencrypted frames: returns data with CRC bytes removed
let user_data_clean = frame.user_data_clean();

// Or get raw data - for no_std environments
let user_data_raw = frame.user_data_raw();
```

## Frame Structure

Wireless M-Bus frames consist of:

```text
[Block 0 - Header]
Byte 0:      L-field (length)
Byte 1:      C-field (control: function, accessibility, synchronous)
Byte 2-3:    M-field (manufacturer ID, little-endian)
Byte 4-7:    A-field device ID (4 bytes, little-endian BCD)
Byte 8:      A-field version
Byte 9:      A-field device type/medium
Byte 10-11:  CRC-16 of bytes 0-9 (big-endian!)
Byte 12:     CI-field (control information)
Byte 13+:    User data (structure depends on CI-field)
```

### User Data Structure

The structure of user data (bytes 13+) depends on whether the frame is encrypted:

**For UNENCRYPTED frames (CI < 0x72 or CI > 0x7F):**
```text
[Block 1+ - User Data]
Byte 13-28:  User data (16 bytes)
Byte 29-30:  CRC-16 of bytes 13-28 (big-endian!)

[Subsequent blocks]
... 16 bytes data + 2 bytes CRC ...

[Last block]
... ((L-9) MOD 16) bytes data + 2 bytes CRC
```

**For ENCRYPTED frames (CI 0x72-0x7F):**
```text
Byte 13+:    Encrypted payload (no CRC blocks)
```

**Important:**
- CRC bytes are stored in **big-endian** format (MSB first), unlike wired M-Bus which uses little-endian
- Encrypted frames (CI-field 0x72-0x7F) contain encrypted payloads without multi-block CRC structure
- Use `frame.is_encrypted()` to check if a frame is encrypted

### Format A vs Format B

- **Format A**: Length excludes itself and CRC bytes (IEC 60870-5-1 FT3)
- **Format B**: Length excludes only itself

## Transmission Modes

- **S-mode**: Stationary (walk-by reading), 32.768 kHz
- **T-mode**: Frequent transmission (fixed network), 32.768 kHz
- **C-mode**: Compact, low power, bidirectional, 100 kHz
- **R-mode**: Frequent bidirectional, 32.768 kHz
- **N-mode**: Narrowband, optimized for long range
- **F-mode**: Frequent transmission, longer range

## CRC-16 Algorithm

Implements CRC-16/EN-13757:
- Polynomial: 0x3D65
- Init: 0x0000
- RefIn: false
- RefOut: false
- XorOut: 0xFFFF
- Check: 0xC2B7 (for "123456789")

## Integration with Application Layer

The wireless frame's user data contains application layer information that can be parsed using the `m-bus-application-layer` crate.

**Important:** The `data` field contains **raw data with CRC bytes interleaved**. For clean data without CRC bytes, use the `user_data_clean()` method (requires `std` feature):

```rust
use m_bus_wireless_frame::Frame;
use m_bus_application_layer::user_data::UserDataBlock;

let frame = Frame::try_from(wireless_data)?;

// Get clean user data (CRC bytes removed)
let clean_data = frame.user_data_clean();

// Parse application layer with clean data
let user_data_block = UserDataBlock::try_from(&clean_data[..])?;
```

For `no_std` environments, use `user_data_raw()` and manually skip CRC bytes:

```rust
// In no_std: raw data includes CRC bytes
// Structure: [16 bytes data][2 CRC][16 bytes data][2 CRC]...
let raw_data = frame.user_data_raw();
// You must manually extract data bytes and skip CRC bytes
```

## License

MIT
