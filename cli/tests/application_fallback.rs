use std::process::{Command, Output};

const APPLICATION_INPUTS: [&str; 4] = [
    "03 13 15 31 00",
    "78 03 13 15 31 00",
    "51 03 13 15 31 00",
    "72 00 51 20 02 82 4D 02 04 00 00 00 00 03 13 15 31 00",
];

fn parse(input: &str, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_m-bus-parser-cli"))
        .args(["parse", "--data", input, "--format", format])
        .output()
        .expect("CLI must run")
}

#[test]
fn application_payloads_render_in_every_cli_format() {
    for input in APPLICATION_INPUTS {
        for format in [
            "table",
            "json",
            "yaml",
            "csv",
            "mermaid",
            "xml",
            "annotated",
            "annotated-text",
        ] {
            let output = parse(input, format);
            assert!(
                output.status.success(),
                "{input} / {format}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!output.stdout.is_empty(), "{input} / {format}");
            if format == "json" {
                let json = String::from_utf8(output.stdout).expect("JSON must be UTF-8");
                assert!(json.contains("\"protocol\": \"application\""));
                assert!(json.contains("\"value\": \"12.565\""));
                assert!(json.contains("\"unit\": \"m3\""));
            }
        }
    }
}

#[test]
fn full_frames_keep_their_detected_protocol() {
    for (input, protocol) in [
        ("68 06 06 68 53 FE 51 01 7A 08 25 16", "wired"),
        ("1444AE0C7856341201078C2027780B134365877AC5", "wireless"),
    ] {
        let output = parse(input, "json");
        assert!(output.status.success());
        let json = String::from_utf8(output.stdout).expect("JSON must be UTF-8");
        assert!(json.contains(&format!("\"protocol\": \"{protocol}\"")));
    }
}

#[test]
fn invalid_inputs_fail_in_every_cli_format() {
    for (input, error_code) in [
        ("zz", "input.invalid_hex"),
        ("", "input.empty"),
        ("78 03 13 15", "application.records_invalid"),
        ("03 13 15", "application.records_invalid"),
        ("72 00", "application.invalid"),
    ] {
        for format in [
            "table",
            "json",
            "yaml",
            "csv",
            "mermaid",
            "xml",
            "annotated",
            "annotated-text",
        ] {
            let output = parse(input, format);
            assert!(!output.status.success(), "{input} / {format}");
            assert!(output.stdout.is_empty(), "{input} / {format}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(error_code), "{input} / {format}: {error}");
        }
    }
}
