#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building napi addon (cargo build --features napi --release)..."
cargo build --features napi --release

case "$(uname -s)" in
  Linux*)  cp target/release/libcolor_convert_rs.so js/color_convert_rs.node ;;
  Darwin*) cp target/release/libcolor_convert_rs.dylib js/color_convert_rs.node ;;
  MINGW*|MSYS*|CYGWIN*) cp target/release/color_convert_rs.dll js/color_convert_rs.node ;;
  *) echo "Unknown platform"; exit 1 ;;
esac

echo ""
echo "Native addon built and copied to js/color_convert_rs.node"
echo "Run tests with: cd js && npm install && npm test"
