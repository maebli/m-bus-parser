#!/usr/bin/env python3
"""Measure the shared full-corpus decoder's Cortex-M4 flash and stack watermark."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
TARGET = "thumbv7em-none-eabi"


def run(args, env, capture=False):
    print("+ " + " ".join(map(str, args)), flush=True)
    result = subprocess.run(list(map(str, args)), env=env, cwd=ROOT, check=True,
                            text=True, stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else ""


def fixtures(output):
    paths = sorted((ROOT / "tests/rscada/test-frames").glob("*.hex"))
    assert paths, "empty corpus"
    lines = ["struct fixture {const unsigned char *data; unsigned length;};"]
    for index, path in enumerate(paths):
        payload = bytes.fromhex(path.read_text())
        lines.append(f'__attribute__((section(".fixtures"))) static const unsigned char input_{index}[] = {{' + ','.join(map(str, payload)) + '};')
    lines.append('__attribute__((section(".fixtures"))) static const struct fixture fixtures[] = {' + ','.join(
        '{input_' + str(index) + ',sizeof(input_' + str(index) + ')}' for index in range(len(paths))) + '};')
    (output / "fixtures.h").write_text('\n'.join(lines) + '\n')
    return len(paths)


def memory_metrics(output, source, env, expected, cc=None):
    output.mkdir(parents=True, exist_ok=True)
    cc = cc or env.get("ARM_GCC") or shutil.which("arm-none-eabi-gcc")
    if not cc:
        installed = sorted((ROOT / "target/arm-toolchain").glob("arm-*/bin/arm-none-eabi-gcc"))
        cc = str(installed[0]) if installed else None
    if not cc:
        raise SystemExit("Install Arm GNU Toolchain 14.2.Rel1 and set ARM_GCC; also install qemu-system-arm")
    cc = Path(cc).resolve()
    size = cc.with_name("arm-none-eabi-size")
    compiler = run([cc, "--version"], env, True).splitlines()[0]
    if "14.2.1" not in compiler:
        raise SystemExit(f"Expected pinned Arm GCC 14.2.1, found {compiler}")
    qemu = env.get("QEMU_SYSTEM_ARM") or shutil.which("qemu-system-arm")
    if not qemu:
        raise SystemExit("qemu-system-arm is required for the Cortex-M4 stack watermark")
    qemu_version = run([qemu, "--version"], env, True).splitlines()[0]
    count = fixtures(output)
    if count != len(expected):
        raise ValueError("native and embedded fixture counts differ")
    # Remove only unreachable serial/TCP helpers so no host POSIX headers are
    # needed. The retained prefix contains every decoder/normalizer unchanged.
    original = (source / "mbus/mbus-protocol-aux.c").read_text()
    anchor = "mbus_handle *\nmbus_context_serial("
    if original.count(anchor) != 1:
        raise ValueError("pinned libmbus source layout changed")
    aux = original.split(anchor)[0].replace('#include "mbus-serial.h"', '').replace('#include "mbus-tcp.h"', '')
    diagnostic = "#define MBUS_ERROR(...) fprintf (stderr, __VA_ARGS__)"
    if aux.count(diagnostic) != 1:
        raise ValueError("pinned libmbus diagnostic macro changed")
    (output / "mbus-protocol-aux.c").write_text(aux.replace(diagnostic, "#define MBUS_ERROR(...) ((void)0)"))
    build_env = env | {"CARGO_PROFILE_RELEASE_OPT_LEVEL": "3", "CARGO_PROFILE_RELEASE_LTO": "false",
                       "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "1", "CARGO_PROFILE_RELEASE_PANIC": "abort"}
    run(["cargo", "build", "--locked", "--manifest-path", HERE / "Cargo.toml", "--release", "--target", TARGET], build_env)
    flags = [cc, "-mcpu=cortex-m4", "-mthumb", "-mfloat-abi=soft", "-O3", "-ffunction-sections", "-fdata-sections",
             "-I", source / "mbus", "-I", output]
    for path, name in [(HERE / "harness.c", "harness"), (ROOT / "benches/libmbus-comparison/bridge.c", "bridge"),
                       (source / "mbus/mbus-protocol.c", "protocol"), (output / "mbus-protocol-aux.c", "aux")]:
        run(flags + ["-c", path, "-o", output / f"{name}.o"], env)
    libraries = {
        "rust": [HERE / "target" / TARGET / "release/libm_bus_library_resources.a"],
        "libmbus": [output / f"{name}.o" for name in ("bridge", "protocol", "aux")],
    }
    metrics, details = [], {}
    for library, objects in libraries.items():
        binary = output / f"{library}.elf"
        run(flags[:4] + ["-nostartfiles", "-Wl,--gc-sections", "-T", HERE / "memory.ld", output / "harness.o"] + objects +
            ["-Wl,--start-group", "-lc", "-lm", "-lgcc", "-Wl,--end-group", "-o", binary], env)
        sections = run([size, "-A", binary], env, True)
        section_sizes = {name: int(amount) for name, amount in re.findall(r"^(\.[\w.]+)\s+(\d+)\s+\d+", sections, re.M)}
        # Match conventional text+data footprint, excluding embedded input data.
        summary = run([size, binary], env, True).splitlines()[-1].split()
        flash = int(summary[0]) + int(summary[1]) - section_sizes[".fixtures"]
        result = subprocess.run([qemu, "-M", "mps2-an386", "-nographic", "-semihosting-config", "enable=on,target=native",
                                 "-kernel", str(binary)], env=env, capture_output=True, text=True, timeout=30, check=True)
        measurements = {key: int(value) for key, value in re.findall(r"^(stack|heap_reserved|records|errors)=(\d+)$", result.stdout + result.stderr, re.M)}
        if set(measurements) != {"stack", "heap_reserved", "records", "errors"} or not 0 < measurements["stack"] < 65536:
            raise ValueError(f"invalid stack watermark: {result.stdout} {result.stderr}")
        for key in ("records", "errors"):
            if measurements[key] != sum(frame[f"{library}_{key}"] for frame in expected):
                raise ValueError(f"{library}: embedded {key} differ from validated native results")
        context = (f"target={TARGET}; Cortex-M4; O3; soft float; lto=off; {compiler}; {qemu_version}; frames={count}; "
                   f"heap_reserved={measurements['heap_reserved']}; includes common decoder wrapper/runtime; excludes fixtures")
        metrics.extend([
            {"name": f"Peak decoder stack [implementation={library}]", "value": measurements["stack"], "unit": "bytes",
             "extra": context + "; observed stack watermark, max over corpus, not a static worst-case bound"},
            {"name": f"Linked decoder flash [implementation={library}]", "value": flash, "unit": "bytes",
             "extra": context + "; linked text+data minus .fixtures"},
        ])
        details[library] = measurements | {"flash_bytes": flash, "sections": section_sizes}
    return metrics, {"target": TARGET, "c_compiler": compiler, "qemu": qemu_version, "libraries": details}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--libmbus-source", type=Path, required=True)
    parser.add_argument("--decode-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--details", type=Path, required=True)
    args = parser.parse_args()
    metrics, details = memory_metrics(ROOT / "target/library-resources", args.libmbus_source.resolve(), os.environ.copy(), json.loads(args.decode_report.read_text()))
    args.output.write_text(json.dumps(metrics, indent=2) + '\n')
    args.details.write_text(json.dumps(details, indent=2) + '\n')
