#![cfg(feature = "std")]
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

const DOCS_HTML: &str = include_str!("../docs/index.html");
const RUST_WORKFLOW: &str = include_str!("../.github/workflows/rust.yml");

#[test]
fn docs_use_the_rust_wasm_highlighter_without_an_npm_toolchain() {
    assert!(DOCS_HTML.contains("m_bus_highlight"));
    assert!(!DOCS_HTML.contains("highlighter.js"));
    assert!(!RUST_WORKFLOW.contains("npm ci"));
    assert!(!RUST_WORKFLOW.contains("npm install"));
    assert!(!RUST_WORKFLOW.contains("docs-ui"));

    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "docs-ui/package.json",
        "docs-ui/package-lock.json",
        "docs/package.json",
        "docs/assets/highlighter.js",
    ] {
        assert!(
            !repository.join(path).exists(),
            "{path} must not be part of the Rust-only docs build"
        );
    }
}

#[test]
fn docs_line_layout_and_palette_match_the_rust_contract() {
    assert!(
        !DOCS_HTML.contains(".code-source code .line"),
        "syntax lines must use their native newline separators"
    );
    assert!(DOCS_HTML.contains("flex: 0 0 auto;"));

    for color in [
        "#0d1117", "#4b5563", "#0a3069", "#953800", "#a0111f", "#0349b4", "#622cbc", "#024c1a",
        "#f0f3f6", "#c7cdd5", "#a5d6ff", "#ffb77c", "#ff9492", "#71b7ff", "#cb9eff", "#6de080",
        "#005a5a", "#704800", "#3b3f99", "#5eead4", "#f8d866", "#b6c7ff",
    ] {
        assert!(
            DOCS_HTML.contains(color),
            "missing audited token color {color}"
        );
    }
}

#[test]
fn docs_hex_view_uses_serialized_annotation_names() {
    for identifier in [
        "layer === 'frame'",
        "layer === 'app_header'",
        "kind === 'dif'",
        "kind === 'vife'",
        "kind === 'plaintext_vif'",
        "kind === 'data_payload'",
        "'record_field': 'Data Records'",
        "seg.layer === 'record_field'",
    ] {
        assert!(
            DOCS_HTML.contains(identifier),
            "missing annotation identifier {identifier}"
        );
    }

    for stale_identifier in [
        "layer === 'Frame'",
        "layer === 'AppHeader'",
        "kind === 'Dif'",
        "kind === 'DataPayload'",
        "seg.layer === 'RecordField'",
    ] {
        assert!(
            !DOCS_HTML.contains(stale_identifier),
            "stale annotation identifier {stale_identifier}"
        );
    }
}

#[test]
fn checked_in_wasm_stays_within_the_docs_budget() {
    let wasm = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/m_bus_parser_wasm_pack_bg.wasm");
    let bytes = fs::metadata(wasm).unwrap().len();
    assert!(
        bytes <= 2 * 1024 * 1024,
        "docs WASM is {bytes} bytes; budget is 2 MiB"
    );
}
