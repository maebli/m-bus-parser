#!/usr/bin/env python3
"""Compare native latency and Thumb footprint for Rust optimization levels."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import tempfile

from measure import HERE, MANIFEST, TARGET, measure_sections, run

OPT_LEVELS = ("z", "s", "3")


def measure_latency(temp: Path, base_env: dict[str, str], opt_level: str) -> dict:
    target_dir = temp / "native-target"
    env = base_env | {
        "CARGO_TARGET_DIR": str(target_dir),
        "CRITERION_HOME": str(target_dir / "criterion"),
        "CARGO_PROFILE_BENCH_OPT_LEVEL": opt_level,
        "CARGO_PROFILE_BENCH_LTO": "true",
        "CARGO_PROFILE_BENCH_CODEGEN_UNITS": "1",
    }
    run(
        [
            "cargo", "bench", "--manifest-path", str(MANIFEST), "--locked",
            "--bench", "optimization", "--", "--noplot",
            "--sample-size", "50", "--warm-up-time", "1", "--measurement-time", "3",
            "--nresamples", "10000",
        ],
        cwd=HERE,
        env=env,
    )
    estimates = target_dir / "criterion/parse_full_frame_eager/new/estimates.json"
    return json.loads(estimates.read_text())["mean"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("optimization-comparison.json"))
    args = parser.parse_args()
    toolchain = os.environ.get("PARSER_RESOURCES_TOOLCHAIN", "nightly")
    env = os.environ.copy()
    # Host flags/targets must not silently change the comparison or Thumb build.
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET"):
        env.pop(key, None)
    env["RUSTUP_TOOLCHAIN"] = toolchain
    compiler = run(["rustc", "-Vv"], cwd=HERE, env=env).strip()
    host = next(line.removeprefix("host: ") for line in compiler.splitlines() if line.startswith("host: "))
    metrics = []
    print("opt-level | native mean (ns/frame) | Thumb text+data (B) | static RAM (B)", flush=True)
    with tempfile.TemporaryDirectory(prefix="m-bus-optimization-") as tmp:
        for level in OPT_LEVELS:
            directory = Path(tmp) / level
            directory.mkdir()
            text, data, bss = measure_sections(directory, env, level)
            estimate = measure_latency(directory, env, level)
            context = f"toolchain={toolchain}; opt-level={level}; lto=fat; codegen-units=1"
            latency = estimate["point_estimate"]
            interval = estimate["confidence_interval"]
            metrics.append({
                "name": f"Eager full-frame decode latency [opt-level={level}]",
                "unit": "ns/frame", "value": latency,
                "range": f"{interval['confidence_level']:.0%} CI {interval['lower_bound']:.2f}–{interval['upper_bound']:.2f}",
                "extra": f"{context}; target={host}; native Criterion mean; nine records/frame; panic=unwind",
            })
            for name, value, sections in (
                ("Linked eager parser flash", text + data, "text+data"),
                ("Linked eager parser static RAM", data + bss, "data+bss; excludes stack/heap"),
            ):
                metrics.append({
                    "name": f"{name} [opt-level={level}]", "unit": "bytes", "value": value,
                    "extra": f"{context}; target={TARGET}; panic=abort; sections={sections}",
                })
            print(f"{level:>9} | {latency:>22.2f} | {text + data:>19} | {data + bss:>14}", flush=True)
    args.output.write_text(json.dumps(metrics, indent=2) + "\n")
    print(f"Wrote {len(metrics)} metrics to {args.output}")


if __name__ == "__main__":
    main()
