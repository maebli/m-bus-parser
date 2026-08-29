#!/usr/bin/env python3
"""Emit Thumb stack, type-size, and linked-footprint benchmark metrics."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


HERE = Path(__file__).resolve().parent
MANIFEST = HERE / "Cargo.toml"
TARGET = "thumbv7em-none-eabi"

CRITICAL_PATH = (
    ("record_iteration", "DataRecords::next"),
    ("data_record_try_from", "DataRecord::try_from"),
    ("data_record_parse", "DataRecord::parse"),
    ("record_header_parse", "DataRecordHeader::try_from"),
    ("processed_header_parse", "ProcessedDataRecordHeader::try_from"),
    ("value_information_parse", "ValueInformation::try_from"),
    ("vife_consumer", "consume_orthhogonal_vife"),
)

FRAME_SYMBOLS = {
    "full_parse": "m_bus_stack_usage::full_parse_fixture::parse_full_wired_frame",
    "mbus_parse": (
        "<m_bus_parser::mbus_data::MbusData<wired_mbus_link_layer::WiredFrame> as "
        "core::convert::TryFrom<&[u8]>>::try_from"
    ),
    "wired_frame_parse": (
        "<wired_mbus_link_layer::WiredFrame as "
        "core::convert::TryFrom<&[u8]>>::try_from"
    ),
    "checksum": "wired_mbus_link_layer::validate_checksum",
    "user_data_parse": (
        "<m_bus_application_layer::UserDataBlock as "
        "core::convert::TryFrom<&[u8]>>::try_from"
    ),
    "identification_parse": (
        "<m_bus_core::IdentificationNumber>::from_bcd_hex_digits"
    ),
    "bcd_parse": "m_bus_core::bcd_hex_digits_to_u32",
    "record_iteration": (
        "<m_bus_application_layer::DataRecords as "
        "core::iter::traits::iterator::Iterator>::next"
    ),
    "vif_block_parse": (
        "<m_bus_application_layer::value_information::ValueInformationBlock as "
        "core::convert::TryFrom<&[u8]>>::try_from"
    ),
    "record_header_parse": (
        "<m_bus_application_layer::data_record::DataRecordHeader as "
        "core::convert::TryFrom<&[u8]>>::try_from"
    ),
    "processed_header_parse": (
        "<m_bus_application_layer::data_record::ProcessedDataRecordHeader as "
        "core::convert::TryFrom<&m_bus_application_layer::data_record::"
        "RawDataRecordHeader>>::try_from"
    ),
    "value_information_parse": (
        "<m_bus_application_layer::value_information::ValueInformation as "
        "core::convert::TryFrom<&m_bus_application_layer::value_information::"
        "ValueInformationBlock>>::try_from"
    ),
    "data_record_parse": "<m_bus_application_layer::data_record::DataRecord>::parse",
}


def run(command: list[str], *, cwd: Path, env: dict[str, str]) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if completed.returncode:
        sys.stderr.write(completed.stdout)
        raise SystemExit(completed.returncode)
    return completed.stdout


def one_file(paths: list[Path], description: str) -> Path:
    if len(paths) != 1:
        raise SystemExit(f"expected one {description}, found {len(paths)}")
    return paths[0]


def parse_stack_sizes(output: str) -> dict[str, int]:
    entries = re.findall(
        r"Entry \{\s+Functions: \[(.*?)\]\s+Size: (0x[0-9A-Fa-f]+)\s+\}",
        output,
        flags=re.DOTALL,
    )
    sizes = {symbol.strip(): int(size, 16) for symbol, size in entries}

    frames: dict[str, int] = {}
    for name, symbol in FRAME_SYMBOLS.items():
        if symbol not in sizes:
            raise SystemExit(f"stack size not found for {symbol}")
        frames[name] = sizes[symbol]

    patterns = {
        "data_record_try_from": (
            "<m_bus_application_layer::data_record::DataRecord as "
            "core::convert::TryFrom<",
            ">::try_from",
        ),
        "vife_consumer": (
            "m_bus_application_layer::value_information::consume_orthhogonal_vife",
            "",
        ),
    }
    for name, (prefix, suffix) in patterns.items():
        matches = [
            size
            for symbol, size in sizes.items()
            if symbol.startswith(prefix) and symbol.endswith(suffix)
        ]
        if not matches:
            raise SystemExit(f"stack size not found for {name}")
        frames[name] = max(matches)

    return frames


def parse_type_size(output: str, pattern: str) -> int:
    match = re.search(
        rf"print-type-size type: `{pattern}`: ([0-9]+) bytes",
        output,
    )
    if match is None:
        raise SystemExit(f"type size not found for {pattern}")
    return int(match.group(1))


def parse_footprint(output: str) -> int:
    rows = [line.split() for line in output.splitlines() if line.strip()]
    if len(rows) < 2 or rows[0][:3] != ["text", "data", "bss"]:
        raise SystemExit("unexpected llvm-size output")
    return int(rows[1][0]) + int(rows[1][1])


def metric(name: str, value: int, extra: str) -> dict[str, object]:
    return {"name": name, "unit": "bytes", "value": value, "extra": extra}


def measure_stack(temp: Path, base_env: dict[str, str]) -> tuple[dict[str, int], str]:
    target_dir = temp / "stack-target"
    env = base_env | {
        "CARGO_TARGET_DIR": str(target_dir),
        "RUSTFLAGS": " ".join(
            (
                "-Z emit-stack-sizes",
                "-Z print-type-sizes",
                "-C link-dead-code=yes",
                "-C embed-bitcode=no",
            )
        ),
    }
    build_output = run(
        [
            "cargo",
            "build",
            "--manifest-path",
            str(MANIFEST),
            "--locked",
            "--release",
            "--lib",
            "--target",
            TARGET,
        ],
        cwd=HERE,
        env=env,
    )

    deps = target_dir / TARGET / "release" / "deps"
    objects_dir = temp / "objects"
    objects_dir.mkdir()
    objects: list[Path] = []
    for crate in (
        "m_bus_stack_usage",
        "m_bus_parser",
        "wired_mbus_link_layer",
        "m_bus_application_layer",
        "m_bus_core",
    ):
        archive = one_file(sorted(deps.glob(f"lib{crate}-*.rlib")), f"{crate} archive")
        crate_objects = objects_dir / crate
        crate_objects.mkdir()
        run(["rust-ar", "x", str(archive)], cwd=crate_objects, env=env)
        objects.extend(sorted(crate_objects.glob("*.o")))
    if not objects:
        raise SystemExit("no object files found in parser archives")
    stack_output = run(
        ["rust-readobj", "--stack-sizes", "--demangle", *map(str, objects)],
        cwd=objects_dir,
        env=env,
    )
    return parse_stack_sizes(stack_output), build_output


def measure_footprint(temp: Path, base_env: dict[str, str]) -> int:
    target_dir = temp / "footprint-target"
    env = base_env | {
        "CARGO_TARGET_DIR": str(target_dir),
        "CARGO_PROFILE_RELEASE_OPT_LEVEL": "z",
        "CARGO_PROFILE_RELEASE_LTO": "true",
        "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "1",
        "CARGO_PROFILE_RELEASE_PANIC": "abort",
    }
    env.pop("RUSTFLAGS", None)
    run(
        [
            "cargo",
            "build",
            "--manifest-path",
            str(MANIFEST),
            "--locked",
            "--release",
            "--bin",
            "parser-footprint",
            "--target",
            TARGET,
        ],
        cwd=HERE,
        env=env,
    )

    host_libdir = Path(
        run(["rustc", "--print", "target-libdir"], cwd=HERE, env=base_env).strip()
    )
    llvm_size = host_libdir.parent / "bin" / "llvm-size"
    if not llvm_size.is_file():
        raise SystemExit(f"llvm-size not found at {llvm_size}")
    binary = target_dir / TARGET / "release" / "parser-footprint"
    output = run(
        [str(llvm_size), "--format=berkeley", str(binary)],
        cwd=HERE,
        env=base_env,
    )
    return parse_footprint(output)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("parser-resources.json"))
    args = parser.parse_args()

    toolchain = os.environ.get("STACK_USAGE_TOOLCHAIN", "nightly")
    base_env = os.environ.copy()
    base_env["RUSTUP_TOOLCHAIN"] = toolchain

    with tempfile.TemporaryDirectory(prefix="m-bus-parser-resources-") as temp:
        temp_path = Path(temp)
        frames, build_output = measure_stack(temp_path, base_env)
        footprint = measure_footprint(temp_path, base_env)

    record_stack = sum(frames[name] for name, _ in CRITICAL_PATH)
    wired_setup_stack = frames["wired_frame_parse"] + frames["checksum"]
    application_setup_stack = (
        frames["user_data_parse"]
        + frames["identification_parse"]
        + frames["bcd_parse"]
    )
    setup_stack = frames["mbus_parse"] + max(
        wired_setup_stack,
        application_setup_stack,
    )
    full_stack = frames["full_parse"] + max(setup_stack, record_stack)
    data_record_size = parse_type_size(build_output, r"data_record::DataRecord<'_>")
    vif_block_size = parse_type_size(
        build_output,
        r"value_information::ValueInformationBlock(?:<'_>)?",
    )
    context = f"target={TARGET}; toolchain={toolchain}"
    setup_context = (
        f"{context}; mbus={frames['mbus_parse']} B; "
        f"wired={wired_setup_stack} B; application={application_setup_stack} B"
    )
    metrics = [
        metric(
            "Eager full wired-frame parse stack",
            full_stack,
            (
                f"{context}; fixture={frames['full_parse']} B; "
                f"max(setup={setup_stack} B,records={record_stack} B)"
            ),
        ),
        metric("Frame and application setup nested stack", setup_stack, setup_context),
        metric("Record decode nested stack", record_stack, context),
        metric(
            "Eager full-frame fixture local stack frame",
            frames["full_parse"],
            context,
        ),
        *[
            metric(f"{label} local stack frame", frames[name], context)
            for name, label in CRITICAL_PATH
        ],
        metric(
            "VIF block parser local stack frame",
            frames["vif_block_parse"],
            context,
        ),
        metric("DataRecord value size", data_record_size, context),
        metric("VIF block value size", vif_block_size, context),
        metric(
            "Linked eager full parser text + data size",
            footprint,
            (
                f"{context}; profile=opt-level=z,lto=fat,codegen-units=1; "
                "sections=text+data"
            ),
        ),
    ]
    args.output.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(metrics)} metrics to {args.output}")


if __name__ == "__main__":
    main()
