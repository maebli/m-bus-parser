"""Callgrind counts for the full wired corpus; Linux x86-64 only."""
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess


def instruction_total(profile):
    events = re.findall(r"^events: (.+)$", profile, re.M)
    totals = re.findall(r"^summary: (.+)$", profile, re.M)
    if len(events) != 1 or len(totals) != 1:
        raise ValueError("missing or ambiguous Callgrind summary")
    names, values = events[0].split(), totals[0].split()
    if "Ir" not in names or len(names) != len(values):
        raise ValueError("missing instruction count")
    count = int(values[names.index("Ir")])
    if count <= 0:
        raise ValueError("empty instruction workload")
    return count


def measure(root, env, context, frames, output):
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise RuntimeError("instruction counting requires Linux x86-64 and Valgrind")
    if frames < 1:
        raise ValueError("empty corpus")
    output.mkdir(parents=True, exist_ok=True)
    env = env | {"RUSTFLAGS": "-C target-cpu=x86-64", "CFLAGS": "-march=x86-64 -mtune=generic"}
    version = subprocess.check_output(["valgrind", "--version"], env=env, text=True).strip()
    libc = subprocess.check_output(["ldd", "--version"], env=env, text=True).splitlines()[0]
    # libc can select different implementations from the CPU feature flags.
    cpu = re.search(r"^flags\s*: (.+)$", Path("/proc/cpuinfo").read_text(), re.M)
    if not cpu:
        raise RuntimeError("cannot fingerprint CPU features")
    cpu_hash = hashlib.sha256(" ".join(sorted(cpu[1].split())).encode()).hexdigest()
    environment = f"{context}; target=x86_64-linux; cpu=x86-64; cpu-features={cpu_hash}; {version}; {libc}; callgrind-scope=v1"
    fingerprint = hashlib.sha256(environment.encode()).hexdigest()
    build = subprocess.check_output([
        "cargo", "bench", "--locked", "--manifest-path", str(root / "benches/libmbus-comparison/Cargo.toml"),
        "--bench", "comparison", "--no-run", "--message-format=json",
    ], env=env, cwd=root, text=True)
    artifacts = [json.loads(line) for line in build.splitlines() if line.startswith("{")]
    binaries = [a["executable"] for a in artifacts if a.get("executable") and a.get("target", {}).get("name") == "comparison"]
    if len(binaries) != 1:
        raise ValueError("expected one comparison executable")
    metrics = []
    for library in ("rust", "libmbus"):
        profile = output / f"{library}.callgrind"
        subprocess.run([
            "valgrind", "--tool=callgrind", "--cache-sim=no", "--branch-sim=no",
            "--collect-atstart=no", f"--toggle-collect=instruction_{library}",
            f"--callgrind-out-file={profile}", binaries[0], "--noplot",
        ], env=env | {"LIBMBUS_INSTRUCTIONS": library}, cwd=root, check=True, timeout=120)
        total = instruction_total(profile.read_text())
        metrics.append({
            "name": f"Decoder instructions [implementation={library}]", "unit": "instructions/message",
            "value": total / frames,
            "extra": f"baseline={fingerprint}; {environment}; frames={frames}; total={total}; includes allocation/cleanup; excludes setup and XML",
        })
    return metrics


def regressions(metrics, history, threshold=1.05):
    runs = history.get("entries", {}).get("Parser instruction counts", [])
    if not runs:
        return [], "No instruction baseline yet."
    previous = {m["name"]: m for m in max(runs, key=lambda run: run["date"])["benches"]}
    failures, compared = [], 0
    for current in metrics:
        old = previous.get(current["name"])
        fingerprint = re.search(r"\bbaseline=([0-9a-f]{64});", current.get("extra", ""))
        if not old or not fingerprint or fingerprint.group(0) not in old.get("extra", ""):
            continue
        if old["value"] <= 0:
            raise ValueError("invalid instruction baseline")
        compared += 1
        ratio = current["value"] / old["value"]
        if ratio > threshold:
            failures.append(f"{current['name']}: +{(ratio - 1) * 100:.1f}% (limit +{(threshold - 1) * 100:.0f}%)")
    return failures, f"Compared {compared}/{len(metrics)} instruction metrics; changed environments start a new baseline."


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Check instruction counts against the latest CI baseline.")
    parser.add_argument("metrics", type=Path)
    parser.add_argument("history", type=Path)
    args = parser.parse_args()
    failures, status = regressions(json.loads(args.metrics.read_text()), json.loads(args.history.read_text()) if args.history.exists() else {})
    print(status)
    for failure in failures:
        print(f"::error::{failure}")
    raise SystemExit(bool(failures))
