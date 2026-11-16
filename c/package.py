import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--profile", choices=["debug", "release"], default="release")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "c/Cargo.toml").read_text())["package"]["version"]
    if os.environ.get("GITHUB_REF_TYPE") == "tag":
        if os.environ["GITHUB_REF_NAME"] != f"c-v{version}":
            raise SystemExit(f"Expected release tag c-v{version}")
    library_dir = root / "target" / args.target / args.profile
    if sys.platform == "win32":
        libraries = ["mbus_parser.dll", "mbus_parser.dll.lib", "mbus_parser.lib"]
        executable = library_dir / "parse.exe"
        link_flags = [str(library_dir / "mbus_parser.dll.lib")]
    else:
        suffix = "dylib" if sys.platform == "darwin" else "so"
        libraries = [f"libmbus_parser.{suffix}", "libmbus_parser.a"]
        executable = library_dir / "parse"
        link_flags = [f"-L{library_dir}", "-lmbus_parser", f"-Wl,-rpath,{library_dir}"]
    compiler = os.environ.get("CC", "clang" if sys.platform == "win32" else "cc")
    subprocess.run([
        compiler, "-std=c11", "-Wall", "-Wextra", "-Werror",
        f"-I{root / 'c/include'}", str(root / "c/examples/parse.c"),
        *link_flags, "-o", str(executable),
    ], check=True)
    subprocess.run([str(executable)], check=True)
    if args.verify_only:
        return
    name = f"m-bus-parser-c-{version}-{args.target}"
    staging = root / "target" / "c-package" / name
    (staging / "lib").mkdir(parents=True, exist_ok=True)
    shutil.copytree(root / "c/include", staging / "include", dirs_exist_ok=True)
    shutil.copytree(root / "c/examples", staging / "examples", dirs_exist_ok=True)
    shutil.copy2(root / "LICENSE", staging)
    for library in libraries:
        shutil.copy2(library_dir / library, staging / "lib" / library)
    distribution = root / "target" / "c-dist"
    distribution.mkdir(parents=True, exist_ok=True)
    archive = Path(shutil.make_archive(str(distribution / name), "zip", staging.parent, name))
    with archive.open("rb") as source:
        checksum = hashlib.file_digest(source, "sha256").hexdigest()
    archive.with_suffix(".zip.sha256").write_text(f"{checksum}  {archive.name}\n")
    print(archive)


if __name__ == "__main__":
    main()
