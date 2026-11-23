#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../wasm/site"
for module in parser syntax; do
    wasm-pack build --target web --out-dir "pkg-$module" --out-name "$module" -- --features "$module"
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
