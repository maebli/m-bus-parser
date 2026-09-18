#!/usr/bin/env bash
# Restore the benchmark history that github-action-benchmark appends to.
#
# History lives as a release asset so that recording a measurement never needs
# a commit. On the first run the asset does not exist yet, so fall back to the
# data.js that was committed while the history was stored in the repository.
set -euo pipefail

tag="${BENCH_HISTORY_TAG:-bench-history}"
mkdir -p cache
target="cache/benchmark-data.json"

if gh release download "$tag" --pattern benchmark-data.json --dir cache --clobber 2>/dev/null; then
  echo "Restored history from release $tag"
elif [ -f docs/dev/bench/data.js ]; then
  sed 's/^window.BENCHMARK_DATA = //' docs/dev/bench/data.js > "$target"
  echo "Seeded history from docs/dev/bench/data.js"
else
  echo "No history found; starting a new series"
  exit 0
fi

# A truncated or malformed file would silently reset the charts.
jq empty "$target"
echo "History covers: $(jq -r '.entries | keys | join(", ")' "$target")"
