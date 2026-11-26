#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../wasm/site/grammars"
grammar_tmp=$(mktemp -d)
trap 'rm -rf "$grammar_tmp"' EXIT
syntax_source=https://raw.githubusercontent.com/sublimehq/Packages/fa6b8629c95041bf262d4c1dab95c456a0530122
mkdir "$grammar_tmp/grammars"
for grammar in JavaScript/JSON YAML/YAML XML/XML; do
    curl --fail --location --silent --show-error "$syntax_source/$grammar.sublime-syntax" -o "$grammar_tmp/grammars/${grammar##*/}.sublime-syntax"
done
cargo install --locked --version 5.3.0 --no-default-features --features default-fancy --example gendata --root "$grammar_tmp/tools" syntect
"$grammar_tmp/tools/bin/gendata" synpack "$grammar_tmp/grammars" "$grammar_tmp/newlines.packdump" "$grammar_tmp/no-newlines.packdump"
cp "$grammar_tmp/newlines.packdump" syntax.packdump
