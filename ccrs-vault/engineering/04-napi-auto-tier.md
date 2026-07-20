---
tags: [engineering, napi, performance, auto-tier, benchmark]
updated: 2026-07-20
---

# napi-rs Migration + JS Fast-Path + Auto-Tier

## Overview

v0.1.0 used wasm-pack for the npm package. Single-color calls were 7-25× slower
than JS. v0.2.0 replaces wasm with a three-layer architecture: pure JS for
single-color, napi-rs for batch SIMD, and auto-tier routing.

## What changed

### Removed
- `src/wasm.rs` (deleted)
- `wasm` feature, `wasm-bindgen`/`js-sys` deps
- `js/color_convert_rs.js`, `js/color_convert_rs_bg.wasm` (wasm artifacts)
- wasm CI job

### Added
- `src/napi.rs` — native Node.js addon via `#[napi]` macro
- `js/js-routes.js` — pure JS conversion functions (9 hot routes)
- `napi` feature in Cargo.toml
- Auto-tier detection in `js/index.js`
- Float64Array `.into()` API (6 routes)
- Stride-aware batch API (`src/batch.rs`)
- `image` crate integration (feature-gated)
- `flake.nix` dev shell

## Architecture

```
User calls convert.rgb.lab(input)
              │
              ▼
    ┌─ Auto-tier detection ─┐
    │                       │
    │  Uint8Array?          │──→ napi rgbToLabBatch() → Float32Array
    │  Array > 300?         │──→ napi rgbToLabBatch() → Float32Array
    │  3 numbers?           │──→ JS rgbLab(r,g,b) → [l,a,b]
    │  String?              │──→ napi convertFromString()
    └───────────────────────┘
```

## src/napi.rs exports

| Function | Purpose | Speed |
|----------|---------|-------|
| `convertRoute(from, to, vec)` | Generic single-color (all 272 routes) | ~450ns/call |
| `convertRouteRaw` | Same without rounding | ~450ns/call |
| `rgbHsl/Hsv/Lab/Xyz/Oklab/Cmyk` | Typed fast paths (no string parse) | ~411ns/call |
| `rgbHslInto/...Into` | Float64Array mutation (zero-alloc) | ~256ns/call |
| `rgbToLabBatch/...` | Batch SIMD (Uint8Array → Float32Array) | 2-8ns/px |
| `convertFromString/ToString/ToNumber` | String/number routing | varies |

## js/js-routes.js

9 routes ported from color-convert's conversion math to pure JS:
- `rgbHsl`, `rgbHsv`, `rgbHwb`, `rgbCmyk`, `rgbXyz`, `rgbLab`, `rgbOklab`
- `hslRgb`, `hsvRgb`

Each uses internal unrounded helpers (`_rgbHslRaw`, `_rgbXyzRaw`) for
intermediate precision, then rounds output to match color-convert exactly.

Verified against 272/272 parity test with color-convert@3.1.3.

## Benchmark results

### Single-color (1M iterations)

| Route | color-convert | color-convert-rs | Ratio |
|-------|-------------|-----------------|-------|
| rgb→hsv | 41.1M ops/s | 43.0M ops/s | 1.05× |
| rgb→lab | 8.8M | 9.0M | 1.03× |
| hsl→rgb | 32.8M | 33.1M | 1.01× |
| rgb→oklab | 10.8M | 10.6M | 0.98× |

### Batch (100k-1M pixels, auto-tier)

| Route | JS loop | auto-tier (napi SIMD) | Speedup |
|-------|---------|----------------------|---------|
| rgb→lab | 7.6M | 103-117M | **13.5-15.3×** |
| rgb→oklab | 10.0M | 74-99M | **7.4-10.2×** |
| rgb→xyz | 15.1M | 97-119M | **6.5-7.7×** |
| rgb→cmyk | 52.7M | 97-147M | **1.9-2.8×** |
| rgb→hsl | 72.1M | 96-112M | **1.4-1.6×** |

## Build

```bash
# NixOS:
nix develop --command cargo build --features napi --release
cp target/release/libcolor_convert_rs.so js/color_convert_rs.node

# Standard Linux:
cargo build --features napi --release
cp target/release/libcolor_convert_rs.so js/color_convert_rs.node

# Test:
cd js && npm install && npm test
```

## Cross-platform deployment

Platform-specific `.node` binaries needed:
- `linux-x64-gnu` — libcolor_convert_rs.so
- `linux-arm64-gnu` — same, cross-compiled
- `darwin-arm64` — libcolor_convert_rs.dylib
- `darwin-x64` — same
- `win32-x64-msvc` — color_convert_rs.dll

For now: ship the Linux build. CI matrix build for cross-platform is a
follow-up (see `@napi-rs/cli` patterns).

## Commits

- `d1b218e` feat(napi): replace wasm with native Node.js addon via napi-rs
- `e0d2906` feat(napi): add Float64Array mutation variants — .into() API
- `f17553d` feat(napi): hybrid JS fast-path for single-color
- `5fb686b` feat: auto-tier — automatic JS/napi route selection
