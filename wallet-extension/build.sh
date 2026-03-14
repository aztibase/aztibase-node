#!/bin/bash
# Build the Aztibase WASM module and copy it into the wallet extension.
# Prerequisites: wasm-pack (cargo install wasm-pack)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WASM_CRATE="$PROJECT_ROOT/crates/aztibase-wasm"
OUT_DIR="$SCRIPT_DIR/wasm"

echo "Building aztibase-wasm for browser..."
cd "$WASM_CRATE"
wasm-pack build --target web --out-dir "$OUT_DIR" --out-name aztibase_wasm

# wasm-pack generates some files we don't need in the extension
rm -f "$OUT_DIR/.gitignore" "$OUT_DIR/package.json" "$OUT_DIR/README.md"

echo ""
echo "WASM module built and copied to wallet-extension/wasm/"
echo "Load the extension in Chrome: chrome://extensions -> Load unpacked -> select wallet-extension/"
