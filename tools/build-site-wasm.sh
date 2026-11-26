#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../wasm/site"
site_toolchain=${SITE_RUST_TOOLCHAIN:-nightly-2026-05-16}
rustup component add rust-src --toolchain "$site_toolchain"
for module in parser syntax; do
    rustup run "$site_toolchain" wasm-pack build --target web --out-dir "pkg-$module" --out-name "$module" . -- \
        -Zbuild-std=std,panic_abort -Zbuild-std-features=optimize_for_size --features "$module"
    cp "pkg-$module/$module.js" "pkg-$module/${module}_bg.wasm" ../../docs/
done

cd ../../docs
python3 - <<'PYTHON'
from hashlib import sha256
from pathlib import Path
import re

page = Path("index.html")
html = page.read_text()
for name in ("parser.js", "parser_bg.wasm", "syntax.js", "syntax_bg.wasm"):
    version = sha256(Path(name).read_bytes()).hexdigest()[:16]
    prefix = f"./{name}?v="
    html = re.sub(re.escape(prefix) + r"[\w.-]+", prefix + version, html)
page.write_text(html)
PYTHON
