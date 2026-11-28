use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    Engine,
};
use clap::ValueEnum;
use m_bus_parser::decode_hex_bytes;

#[derive(Clone, Copy, ValueEnum)]
pub enum InputFormat {
    Auto,
    Hex,
    Base64,
}

pub fn normalize_input(input: &str, format: InputFormat) -> Result<String, String> {
    if matches!(format, InputFormat::Hex)
        || matches!(format, InputFormat::Auto)
            && (decode_hex_bytes(input).is_ok()
                || input
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .all(|line| decode_hex_bytes(line).is_ok()))
    {
        return Ok(input.to_string());
    }
    let compact: String = input
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    let decoded = STANDARD
        .decode(&compact)
        .or_else(|_| STANDARD_NO_PAD.decode(&compact));
    match decoded {
        Ok(bytes) => Ok(hex::encode(bytes)),
        Err(_) if matches!(format, InputFormat::Auto) => Ok(input.to_string()),
        Err(error) => Err(format!("[input.invalid_base64] {error}")),
    }
}
