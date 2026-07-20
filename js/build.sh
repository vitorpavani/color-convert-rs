#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v wasm-pack &> /dev/null; then
  echo "Error: wasm-pack not installed. Install via:" >&2
  echo "  nix shell nixpkgs#wasm-pack" >&2
  echo "  # or: cargo install wasm-pack" >&2
  exit 1
fi

wasm-pack build --target nodejs --features wasm

cp pkg/color_convert_rs.js js/
cp pkg/color_convert_rs_bg.wasm js/
cp pkg/color_convert_rs.d.ts js/

echo ""
echo "wasm module built and copied to js/"
echo "Run tests with: cd js && npm install && npm test"
