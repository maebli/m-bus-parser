import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tomllib
import xml.etree.ElementTree as ET


PLATFORMS = {
    "x86_64-unknown-linux-gnu": ("linux-x86_64", "libmbus_parser.so"),
    "aarch64-unknown-linux-gnu": ("linux-aarch64", "libmbus_parser.so"),
    "x86_64-apple-darwin": ("macos-x86_64", "libmbus_parser.dylib"),
    "aarch64-apple-darwin": ("macos-aarch64", "libmbus_parser.dylib"),
    "x86_64-pc-windows-msvc": ("windows-x86_64", "mbus_parser.dll"),
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prebuilt", choices=["true", "false"], default="false")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    pom = ET.parse(root / "java/pom.xml")
    version = pom.findtext("{http://maven.apache.org/POM/4.0.0}version")
    native_version = tomllib.loads((root / "c/Cargo.toml").read_text())["package"]["version"]
    if version != native_version:
        raise SystemExit("The Java and native library versions must match")
    if os.environ.get("GITHUB_REF_TYPE") == "tag":
        if os.environ["GITHUB_REF_NAME"] != f"java-v{version}":
            raise SystemExit(f"Expected release tag java-v{version}")
    resources = root / "java/target/native-resources/native"
    if args.prebuilt == "true":
        for platform, library in PLATFORMS.values():
            if not (resources / platform / library).is_file():
                raise SystemExit(f"Missing native library for {platform}")
        return
    details = subprocess.check_output(["rustc", "-vV"], text=True)
    target = next(line.removeprefix("host: ") for line in details.splitlines() if line.startswith("host: "))
    if target not in PLATFORMS:
        raise SystemExit(f"Unsupported Rust target: {target}")
    subprocess.run([
        "cargo", "build", "--locked", "-p", "m-bus-parser-c", "--release", "--target", target,
    ], cwd=root, check=True)
    platform, library = PLATFORMS[target]
    destination = resources / platform
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(root / "target" / target / "release" / library, destination / library)


if __name__ == "__main__":
    main()
