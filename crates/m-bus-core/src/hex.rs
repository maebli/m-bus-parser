//! Uppercase byte-to-hex formatting for human-facing output.

/// Encode bytes as uppercase hex, optionally separating each byte with a space.
/// The result uses at most one allocation regardless of the number of bytes.
#[must_use]
pub fn encode_upper(bytes: &[u8], spaced: bool) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let separator_count = if spaced {
        bytes.len().saturating_sub(1)
    } else {
        0
    };
    let mut output = String::with_capacity(bytes.len() * 2 + separator_count);
    for (index, &byte) in bytes.iter().enumerate() {
        if spaced && index != 0 {
            output.push(' ');
        }
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0F)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::encode_upper;

    #[test]
    fn uppercase_hex_matches_both_existing_formats() {
        assert_eq!(encode_upper(&[], false), "");
        assert_eq!(encode_upper(&[], true), "");
        let bytes: Vec<u8> = (u8::MIN..=u8::MAX).collect();
        let compact = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>();
        let spaced = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(encode_upper(&bytes, false), compact);
        assert_eq!(encode_upper(&bytes, true), spaced);
        assert_eq!(encode_upper(&[0, 0xA5, 0xFF], true), "00 A5 FF");
    }
}
