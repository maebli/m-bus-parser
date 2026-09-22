use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=LIBMBUS_SOURCE");
    println!("cargo:rerun-if-changed=revision.txt");
    println!("cargo:rerun-if-changed=bridge.c");
    let source = PathBuf::from(
        env::var_os("LIBMBUS_SOURCE")
            .expect("set LIBMBUS_SOURCE to the pinned checkout; see benches/README.md"),
    );
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(&source)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "cannot verify libmbus checkout");
        String::from_utf8(output.stdout).unwrap()
    };
    let revision = include_str!("revision.txt").trim();
    assert_eq!(
        git(&["rev-parse", "HEAD"]).trim(),
        revision,
        "wrong libmbus revision"
    );
    assert!(
        git(&["status", "--porcelain", "--untracked-files=no"])
            .trim()
            .is_empty(),
        "libmbus checkout must be unmodified"
    );
    println!("cargo:rustc-env=LIBMBUS_REVISION={revision}");
    println!("cargo:rerun-if-changed={}", source.display());
    // The whole corpus includes a known unsupported VIF. Disable only the
    // diagnostic macro in a generated copy, so timing does not include stderr
    // I/O. Leave the pinned checkout and all parser logic untouched.
    let aux = fs::read_to_string(source.join("mbus/mbus-protocol-aux.c")).unwrap();
    let diagnostic = "#define MBUS_ERROR(...) fprintf (stderr, __VA_ARGS__)";
    assert_eq!(aux.matches(diagnostic).count(), 1);
    let quiet_aux = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("mbus-protocol-aux.c");
    fs::write(
        &quiet_aux,
        aux.replace(diagnostic, "#define MBUS_ERROR(...) ((void)0)"),
    )
    .unwrap();
    let mut build = cc::Build::new();
    build
        .include(source.join("mbus"))
        .file("bridge.c")
        .opt_level(3)
        .warnings(false);
    for name in ["mbus-protocol.c", "mbus-serial.c", "mbus-tcp.c"] {
        build.file(source.join("mbus").join(name));
    }
    build.file(quiet_aux);
    build.compile("mbus_comparison");
    println!("cargo:rustc-link-lib=m");
}
