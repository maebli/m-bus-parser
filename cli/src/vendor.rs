use std::io::{ErrorKind, Write};
use std::process::{Command, Stdio};

fn launch_error(error: std::io::Error) -> String {
    if error.kind() == ErrorKind::NotFound {
        return "Install wmbusmeters with stdin:jsonl support and add it to PATH".into();
    }
    format!("Could not run wmbusmeters: {error}")
}

pub fn list() -> Result<(), String> {
    let status = Command::new("wmbusmeters")
        .arg("--listdrivers")
        .status()
        .map_err(launch_error)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("wmbusmeters exited with {status}"))
    }
}

fn decode_request(
    input: &str,
    driver: Option<&str>,
    key: Option<&str>,
) -> Result<serde_json::Value, String> {
    let bytes = m_bus_parser::decode_hex_bytes(input).map_err(|error| error.to_string())?;
    let mut request = serde_json::json!({"_": "decode", "telegram": hex::encode(&bytes)});
    if matches!(bytes.as_slice(), [0x68, a, b, 0x68, ..] if a == b && bytes.len() == usize::from(*a) + 6)
    {
        request["format"] = "mbus".into();
    }
    if let Some(name) = driver {
        if name.is_empty()
            || !name
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
        {
            return Err("Expected a driver name from the drivers command".into());
        }
        request["driver"] = name.into();
    }
    if let Some(key) = key {
        if !matches!(key.len(), 16 | 32) || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Expected a 16- or 32-digit hexadecimal meter key".into());
        }
        request["key"] = key.into();
    }
    Ok(request)
}

pub fn decode(input: &str, driver: Option<&str>, key: Option<&str>) -> Result<(), String> {
    let request = decode_request(input, driver, key)?;
    let mut child = Command::new("wmbusmeters")
        .args(["--silent", "--format=json", "stdin:jsonl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(launch_error)?;
    let sent = match child.stdin.take() {
        Some(mut pipe) => writeln!(pipe, "{request}"),
        None => Err(std::io::Error::other("missing child stdin")),
    };
    let output = child.wait_with_output().map_err(launch_error)?;
    sent.map_err(|e| format!("Could not send telegram to wmbusmeters: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "wmbusmeters failed ({}); this command requires stdin:jsonl support",
            output.status
        ));
    }
    let result = decode_response(&output.stdout)?;
    println!("{result}");
    Ok(())
}

fn decode_response(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let result: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "wmbusmeters did not return a single JSON reading")?;
    if let Some(message) = result.get("error").and_then(|v| v.as_str()) {
        if result["_"] == "response"
            || message == "decoding failed"
            || message.starts_with("decryption failed")
        {
            return Err(format!("wmbusmeters: {message}"));
        }
    }
    if result["_"] != "telegram" || !result["driver"].is_string() || result["driver"] == "unknown" {
        return Err("No wmbusmeters driver decoded the telegram".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{decode_request, decode_response};
    use serde_json::json;

    #[test]
    fn requests_identify_wired_frames_and_preserve_driver_and_key() {
        let key = "0123456789abcdef";
        let request = decode_request("68010168000016", Some("x_1"), Some(key)).unwrap();
        assert_eq!(
            request,
            json!({"_":"decode", "telegram":"68010168000016", "format":"mbus", "driver":"x_1", "key":key})
        );
        for frame in ["68010268000016", "6801016800", "44ab"] {
            assert_eq!(
                decode_request(frame, None, None).unwrap(),
                json!({"_":"decode", "telegram":frame})
            );
        }
        assert!(decode_request("44ab", None, Some(&key.repeat(2))).is_ok());
    }

    #[test]
    fn rejects_invalid_input_before_running_a_driver() {
        assert!(decode_request("zz", None, None).is_err());
        for name in ["", "driver\n", "../driver"] {
            assert!(decode_request("44ab", Some(name), None).is_err());
        }
        for key in ["a", "aabbccddeeffgg00"] {
            assert!(decode_request("44ab", None, Some(key)).is_err());
        }
    }

    #[test]
    fn keeps_faults_reported_by_a_meter() {
        let reading = json!({"_":"telegram","driver":"em24","error":"OVER_VOLTAGE"});
        assert_eq!(
            decode_response(reading.to_string().as_bytes()).unwrap(),
            reading
        );
    }

    #[test]
    fn reports_backend_failures_and_unknown_drivers() {
        for sample in [
            json!({"_":"response","error":"failed to parse telegram header"}),
            json!({"_":"telegram","driver":"kamwater","error":"decryption failed, please check key"}),
            json!({"_":"telegram","driver":"kamwater","error":"decoding failed"}),
            json!({"_":"telegram","driver":"unknown"}),
            json!({"_":"telegram"}),
            json!({"_":"response","driver":"kamwater"}),
        ] {
            assert!(decode_response(sample.to_string().as_bytes()).is_err());
        }
        assert!(decode_response(b"invalid JSON").is_err());
    }
}
