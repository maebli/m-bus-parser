#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn vendor_commands_use_the_installed_backend_and_report_its_failures() {
    let directory = std::env::temp_dir().join(format!("mbus-vendor-test-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_m-bus-parser-cli"))
            .env("PATH", &directory)
            .args(args)
            .output()
            .unwrap()
    };
    let backend = directory.join("wmbusmeters");
    let decode = ["vendor", "--data", "AB CD", "--driver", "meter1"];
    let missing = run(&decode);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Install wmbusmeters"));

    fs::write(
        &backend,
        r#"#!/bin/sh
if [ "$*" = "--listdrivers" ]; then
    echo meter1
elif [ "$*" = "--silent --format=json stdin:jsonl" ]; then
    IFS= read -r request || exit 2
    echo "{\"_\":\"telegram\",\"driver\":\"meter1\",\"request\":$request}"
else
    exit 2
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&backend, fs::Permissions::from_mode(0o700)).unwrap();
    let list = run(&["drivers"]);
    assert!(list.status.success());
    assert_eq!(list.stdout, b"meter1\n");
    let result = run(&decode);
    assert!(result.status.success());
    let reading: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        reading["request"],
        serde_json::json!({"_":"decode","driver":"meter1","telegram":"abcd"})
    );

    fs::write(&backend, "#!/bin/sh\nexit 2\n").unwrap();
    assert!(!run(&["drivers"]).status.success());
    assert!(!run(&decode).status.success());
    fs::remove_file(&backend).unwrap();
    fs::remove_dir(&directory).unwrap();
}
