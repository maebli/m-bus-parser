#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../wasm/site"
for module in parser syntax; do
    wasm-pack build --target web --out-dir "pkg-$module" --out-name "$module" -- --features "$module"
    cp "pkg-$module/$module.js" "pkg-$module/${module}_bg.wasm" ../../docs/
done
