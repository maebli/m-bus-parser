#!/usr/bin/env python3
"""Compare full-corpus Rust/libmbus speed, Cortex-M4 stack, heap, and flash: eight metrics."""
from __future__ import annotations

import argparse
import importlib.util
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
COMPARISON = ROOT / "benches/libmbus-comparison"
REVISION = (COMPARISON / "revision.txt").read_text().strip()


def run(args: list[str], env: dict[str, str], *, capture: bool = False) -> str:
    print("+ " + " ".join(args), flush=True)
    result = subprocess.run(args, cwd=ROOT, env=env, check=True, text=True,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else ""


def prepare_source(source: Path, env: dict[str, str]) -> None:
    if not source.exists():
        source.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", "--no-checkout", "https://github.com/rscada/libmbus.git", str(source)], env)
        run(["git", "-C", str(source), "checkout", "--detach", REVISION], env)
    actual = run(["git", "-C", str(source), "rev-parse", "HEAD"], env, capture=True)
    if actual != REVISION:
        raise SystemExit(f"libmbus must be {REVISION}, found {actual}; use a separate pinned checkout")
    if run(["git", "-C", str(source), "status", "--porcelain", "--untracked-files=no"], env, capture=True):
        raise SystemExit("libmbus checkout has tracked modifications")


def corpus_hash() -> str:
    digest = hashlib.sha256()
    paths = sorted((ROOT / "tests/rscada/test-frames").glob("*.hex"))
    paths.append(ROOT / "tests/wmbusmeters/test_vectors.json")
    for path in paths:
        digest.update(str(path.relative_to(ROOT)).encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def metric(benchmark: dict, estimates: dict, context: str) -> dict:
    identifier = benchmark["full_id"]
    if identifier not in ("comparison/decode/rust", "comparison/decode/libmbus"):
        raise ValueError(f"unexpected benchmark {identifier}")
    elements = benchmark.get("throughput", {}).get("Elements")
    if not isinstance(elements, int) or elements < 1:
        raise ValueError(f"invalid frame count: {identifier}")
    implementation = identifier.rsplit("/", 1)[-1]
    estimate = estimates["mean"]
    interval = estimate["confidence_interval"]
    value = estimate["point_estimate"] / elements
    if not math.isfinite(value) or value <= 0:
        raise ValueError(f"invalid latency: {identifier}")
    return {
        "name": f"Corpus decode latency [implementation={implementation}]", "unit": "ns/frame", "value": value,
        "range": f"{interval['confidence_level']:.0%} CI {interval['lower_bound']/elements:.2f}–{interval['upper_bound']/elements:.2f}",
        "extra": f"{context}; criterion={identifier}; frames/iteration={elements}; all wired inputs; allocation/cleanup included; full public decoder APIs, no XML",
    }


def collect(directory: Path, context: str) -> list[dict]:
    metrics = []
    for path in sorted(directory.glob("**/new/benchmark.json")):
        benchmark = json.loads(path.read_text())
        estimates = json.loads(path.with_name("estimates.json").read_text())
        metrics.append(metric(benchmark, estimates, context))
    expected = 2
    if len(metrics) != expected or len({m["name"] for m in metrics}) != expected:
        raise ValueError(f"expected {expected} distinct measurements, got {len(metrics)} in {directory}")
    return metrics


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instructions", type=Path, help="also write Linux x86-64 Callgrind metrics")
    parser.add_argument("--output", type=Path, default=Path("corpus-benchmarks.json"))
    parser.add_argument("--details", type=Path, default=Path("corpus-benchmarks-details.json"))
    parser.add_argument("--libmbus-source", type=Path, default=ROOT / "target/libmbus-source")
    args = parser.parse_args()
    env = os.environ.copy()
    # Pin build policy; do not silently inherit local CPU, feature or profile flags.
    for key in list(env):
        if key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR", "CRITERION_HOME") or key.startswith(("CARGO_PROFILE_", "CFLAGS", "CPPFLAGS")):
            env.pop(key)
    env["RUSTUP_TOOLCHAIN"] = env.get("PARSER_RESOURCES_TOOLCHAIN", "nightly-2026-05-16")
    env["LC_ALL"] = "C"
    env["TZ"] = "UTC"
    env["CC"] = shutil.which("clang") or shutil.which("cc") or "cc"
    source = args.libmbus_source.resolve()
    prepare_source(source, env)
    compiler = run(["rustc", "-Vv"], env, capture=True)
    c_compiler = run([env["CC"], "--version"], env, capture=True)
    context = f"corpus-sha256={corpus_hash()}; {compiler.replace(chr(10), '; ')}"
    with tempfile.TemporaryDirectory(prefix="mbus-corpus-") as temp:
        temp = Path(temp)
        comparison_dir = temp / "comparison"
        report_path = temp / "comparison-report.json"
        decode_report = temp / "decode-report.json"
        comparison_env = env | {"CRITERION_HOME": str(comparison_dir), "LIBMBUS_SOURCE": str(source),
                                "LIBMBUS_COMPARISON_REPORT": str(report_path), "LIBMBUS_DECODE_REPORT": str(decode_report),
                                "CARGO_PROFILE_BENCH_OPT_LEVEL": "3", "CARGO_PROFILE_BENCH_LTO": "false",
                                "CARGO_PROFILE_BENCH_CODEGEN_UNITS": "1"}
        run(["cargo", "bench", "--locked", "--manifest-path", str(COMPARISON / "Cargo.toml"),
             "--bench", "comparison", "--", "--noplot"], comparison_env)
        report = json.loads(report_path.read_text())
        decoded = json.loads(decode_report.read_text())
        comparison_context = (context + f"; rust=3; C=-O3; lto=off; features=std,plaintext-before-extension; "
                              f"libmbus={REVISION}; C compiler={c_compiler.splitlines()[0]}; libmbus diagnostic logging disabled")
        metrics = collect(comparison_dir, comparison_context)
        if args.instructions:
            from instructions import measure
            counts = measure(ROOT, comparison_env, comparison_context, len(decoded), ROOT / "target/instruction-counts")
            args.instructions.write_text(json.dumps(counts, indent=2) + "\n")
            report["instructions"] = counts
        spec = importlib.util.spec_from_file_location("library_resources", ROOT / "benches/library-resources/measure.py")
        memory = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(memory)
        memory_metrics, memory_details = memory.memory_metrics(ROOT / "target/library-resources", source, env, decoded)
        metrics.extend(memory_metrics)
        report.update({"rustc": compiler, "c_compiler": c_compiler, "corpus_sha256": corpus_hash(),
                       "decode_outcomes": decoded, "memory": memory_details})
        args.details.write_text(json.dumps(report, indent=2) + "\n")
    args.output.write_text(json.dumps(metrics, indent=2) + "\n")
    print(f"Wrote {len(metrics)} metrics to {args.output}; comparison details: {args.details}")


if __name__ == "__main__":
    main()
