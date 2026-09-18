#!/usr/bin/env python3
"""Render the benchmark page's data files from the stored history.

Writes two files next to the chart page:

  data.js  the history the charts plot
  tags.js  release tags, so the charts can mark where a version shipped

A tag is attached to the first benchmarked commit that contains it, which is
usually the tagged commit itself but also covers tags cut on commits that were
never benchmarked. Tags older than the first point on a chart are dropped:
they sit outside the plotted window.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def tag_commits(pattern: str) -> dict[str, list[str]]:
    """Map commit sha -> tag names, dereferencing annotated tags."""
    out: dict[str, list[str]] = {}
    listing = git(
        "for-each-ref",
        "--sort=creatordate",
        "--format=%(refname:short)\t%(objectname)\t%(*objectname)",
        f"refs/tags/{pattern}",
    )
    for line in listing.splitlines():
        name, obj, deref = (line.split("\t") + ["", ""])[:3]
        sha = deref or obj
        if sha:
            out.setdefault(sha, []).append(name)
    return out


def commit_exists(sha: str) -> bool:
    return subprocess.run(
        ["git", "cat-file", "-e", f"{sha}^{{commit}}"], capture_output=True
    ).returncode == 0


def tags_for_series(commits: list[str], tags: dict[str, list[str]]) -> dict[str, list[str]]:
    """Attach each tag to the first plotted commit that contains it."""
    known = [c for c in commits if commit_exists(c)]
    labels: dict[str, list[str]] = {}
    for previous, current in zip(known, known[1:]):
        # Commits introduced between two neighbouring points on the chart.
        try:
            span = git("rev-list", f"{previous}..{current}").splitlines()
        except subprocess.CalledProcessError:
            continue
        for sha in span:
            for name in tags.get(sha, []):
                labels.setdefault(current, []).append(name)
    return labels


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--history", default="cache/benchmark-data.json")
    parser.add_argument("--out-dir", default="docs/dev/bench")
    parser.add_argument(
        "--tag-pattern",
        default=os.environ.get("BENCH_TAG_PATTERN", "v*"),
        help="glob under refs/tags to mark on the charts (default: library releases)",
    )
    args = parser.parse_args()

    history = json.loads(Path(args.history).read_text())
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    (out_dir / "data.js").write_text(
        "window.BENCHMARK_DATA = " + json.dumps(history, indent=2, ensure_ascii=False) + "\n"
    )

    tags = tag_commits(args.tag_pattern)
    labelled = {
        name: tags_for_series([e["commit"]["id"] for e in entries], tags)
        for name, entries in history["entries"].items()
    }
    (out_dir / "tags.js").write_text(
        "window.BENCHMARK_TAGS = " + json.dumps(labelled, indent=2, ensure_ascii=False) + "\n"
    )

    for name, marks in labelled.items():
        found = sorted({t for names in marks.values() for t in names})
        print(f"{name}: {len(marks)} tagged point(s) {found}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
