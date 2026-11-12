use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_m-bus-parser-cli"))
}

#[cfg(not(feature = "decryption-mode-13"))]
#[test]
fn session_flag_is_unavailable_without_mode13() {
    let help = cli().args(["parse", "--help"]).output().unwrap();
    assert!(!String::from_utf8_lossy(&help.stdout).contains("--tls-session"));
    let output = cli()
        .args(["parse", "--data", "e5", "--tls-session", "session.json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[cfg(feature = "decryption-mode-13")]
mod enabled {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const OPENSSL_AES128_RECORD: &str =
        "170303001ba0a1a2a3a4a5a6a79bbaad16c9503d86625a3a66e44e7f876bf808";

    const SESSION: &str = r#"{
        "name": "Test meter",
        "cipher_suite": "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
        "direction": "client_to_server",
        "key_hex": "000102030405060708090a0b0c0d0e0f",
        "fixed_iv_hex": "01020304",
        "sequence_number": "5"
    }"#;

    struct Fixture {
        directory: PathBuf,
    }

    impl Fixture {
        fn new(session: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "mbus-cli-tls-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("session.json"), session).unwrap();
            fs::write(directory.join("frame.hex"), Self::frame()).unwrap();
            Self { directory }
        }

        fn frame() -> String {
            format!("2f4493157856341233037a0100ff0d01{}", OPENSSL_AES128_RECORD)
        }

        fn command(&self) -> Command {
            let mut command = cli();
            command.current_dir(&self.directory).args([
                "parse",
                "--file",
                "frame.hex",
                "--tls-session",
                "session.json",
                "--format",
                "json",
            ]);
            command
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    #[test]
    fn session_file_decrypts_file_and_inline_frames() {
        let fixture = Fixture::new(SESSION);
        let from_file = fixture.command().output().unwrap();
        assert!(
            from_file.status.success(),
            "{}",
            String::from_utf8_lossy(&from_file.stderr)
        );
        let output = String::from_utf8(from_file.stdout).unwrap();
        assert!(output.contains("decrypted_authenticated"));
        assert!(output.contains("01132A"));
        assert!(!output.contains("000102030405060708090a0b0c0d0e0f"));

        let inline = cli()
            .current_dir(&fixture.directory)
            .args([
                "parse",
                "--data",
                &Fixture::frame(),
                "--tls-session",
                "session.json",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert!(inline.status.success());
        assert_eq!(String::from_utf8(inline.stdout).unwrap(), output);
    }

    #[test]
    fn invalid_session_and_wrong_counter_produce_no_plaintext() {
        for (session, code) in [
            (
                SESSION.replace("000102030405060708090a0b0c0d0e0f", "SECRET-INVALID-KEY"),
                "option.invalid",
            ),
            (
                SESSION.replace("\"5\"", "\"6\""),
                "security.authentication_failed",
            ),
        ] {
            let fixture = Fixture::new(&session);
            let output = fixture.command().output().unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(code), "{error}");
            assert!(!error.contains("SECRET-INVALID-KEY"));
            assert!(!error.contains("000102030405060708090a0b0c0d0e0f"));
        }
    }

    #[test]
    fn conflicting_keys_and_missing_session_files_are_rejected() {
        let fixture = Fixture::new(SESSION);
        let conflict = fixture
            .command()
            .args(["--key", "000102030405060708090a0b0c0d0e0f"])
            .output()
            .unwrap();
        assert!(!conflict.status.success());
        assert!(conflict.stdout.is_empty());
        assert!(String::from_utf8_lossy(&conflict.stderr).contains("cannot be used with"));

        fs::remove_file(fixture.directory.join("session.json")).unwrap();
        let missing = fixture.command().output().unwrap();
        assert!(!missing.status.success());
        assert!(missing.stdout.is_empty());
        assert!(String::from_utf8_lossy(&missing.stderr).contains("input.tls_session"));
    }
}
