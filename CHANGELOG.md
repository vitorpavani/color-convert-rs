# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-20

### Changed
- **BREAKING**: Removed `wasm` feature, `src/wasm.rs`, `wasm-bindgen`/`js-sys` deps.
  The npm package now uses a native napi-rs addon instead of WebAssembly.
- **BREAKING**: `napi` feature replaces `wasm` feature for Node.js bindings.

### Added
- **napi-rs native addon** (`src/napi.rs`): replaces wasm-pack. Enables rayon
  multi-core and GPU (CubeCL/wgpu) access from Node.js that wasm couldn't provide.
- **Pure JS fast-path** (`js/js-routes.js`): 9 hottest single-color routes
  (rgb→hsl/hsv/cmyk/xyz/lab/oklab/hwb, hsl→rgb, hsv→rgb) ported to hand-optimized
  JS. Runs at V8 JIT speed — at parity with color-convert (0.8-1.05×).
- **Auto-tiering**: `convert.rgb.lab(input)` automatically detects input type and
  routes to JS (single-color) or napi SIMD (typed array/pixel data). No API change
  needed — users get the fastest path automatically.
- **Float64Array `.into()` API**: 6 routes accept a pre-allocated `Float64Array`
  output buffer for zero-allocation tight loops. 1.6× faster than Vec return.
- **Stride-aware batch API** (`src/batch.rs`): `batch::rgb_to_lab(&[u8], stride)`
  handles any pixel layout (RGB stride=3, RGBA stride=4). No re-packing needed.
- **`image` crate integration** (feature-gated): `batch::image::to_lab(&DynamicImage)`
  — one-liner for any `image::DynamicImage`.
- **`flake.nix`**: NixOS dev shell with gcc, wasm-pack, nodejs pre-configured.
- **CI workflow for crates.io publishing** (triggers on `v*` tags).

### Performance
- Single-color: **at parity** with color-convert (was 7-25× slower with wasm)
- Batch (100k+ pixels): **10-15× faster** than JS loop (rgb→lab: 103M ops/s)
- `.into()` tight loops: 3.9M ops/s (1.6× faster than Vec return)
- Full HD frame (1920×1080): all routes under 30ms — real-time video speed

### Fixed
- Oracle review fixes from v0.1.0: exclude list, CI parity test, CHANGELOG gap,
  doctest `no_run` removed.

## [0.1.0] - 2026-07-20

### Added
- **17 color models ported**: RGB, HSL, HSV, HWB, CMYK, XYZ, LAB, LCH, OkLab,
  OkLCh, HCG, Apple, Gray, Hex, Keyword, Ansi16, Ansi256 — all conversions
  validated against JS-generated reference vectors from `color-convert@3.1.3`.
- **BFS route graph**: any-to-any conversion via shortest path, mirroring
  `color-convert`'s routing logic.
- **CPU SIMD batch path** (`wide::f32x8`): 16 vectorized routes (rgb↔xyz↔lab,
  rgb→hsl/hsv/cmyk/hwb/hcg/oklab/apple, and inverses).
- **Multi-core parallelism** (rayon): auto-parallelizes batch routes for
  >4096 pixels, 1.42× speedup on memory-bandwidth-bound routes.
- **GPU compute path** (CubeCL/wgpu, feature-gated): 4 GPU kernels
  (rgb→lab/hsl/hsv/cmyk) with runtime capability probe that degrades to
  CPU-SIMD on GPU-less hosts.
- **npm drop-in replacement** via wasm-pack: `color-convert-rs` npm package
  exposing the exact `convert.rgb.hsl(r,g,b)` API — 272/272 routes match
  `color-convert@3.1.3`.
- **3-tier benchmark harness**: JS baseline vs Rust-CPU-SIMD vs Rust-GPU,
  with append-only results ledger (`benchmarks/results.jsonl`).
- **10-wave optimization drive** (33 kept / 7 dropped): f32x8 SIMD, fused
  rgb→xyz→lab, sRGB inverse-gamma LUT, fast cbrt, multi-core rayon, fused
  multi-hop convert. Headline: rgb→lab at 111.3 MP/s single-core (10.3×
  cumulative speedup vs JS baseline).
- **GitHub Actions CI**: test + clippy + fmt on both CPU-only and GPU
  feature configurations, plus wasm-pack build check.
- **Feature gating**: `gpu` feature (CubeCL/wgpu/pollster/bytemuck),
  `wasm` feature (wasm-bindgen/js-sys). Default = CPU-only (no GPU deps).

### Performance
| Route | JS baseline | Rust single-core SIMD | Speedup |
|-------|-------------|-----------------------|---------|
| rgb→lab | ~6 MP/s | 111.3 MP/s | 10.3× |
| rgb→xyz | ~11 MP/s | 152.2 MP/s | 13.8× |
| rgb→hsl | ~14 MP/s | 142.1 MP/s | 10.1× |
| rgb→oklab | ~8 MP/s | 65.5 MP/s | 8.2× |

### Agentic build process
The entire library was built through an autonomous Red/Green/Blue TDD loop
driven from GitHub issues — an orchestrator spawns `red-dev` (writes failing
test), `green-dev` (minimal code to pass), and `blue-dev` (refactor/review)
per issue. Every performance change is measured on all 3 tiers and kept only
if it beats both the JS baseline and the previous Rust iteration.

### Known limitations
- **`gray.lch` hue**: for achromatic grayscale inputs (chroma=0), the hue is
  arbitrary. The Rust `lab→lch` function returns 180° (matching the JS
  `Math.atan2(-0, 0)` behavior preserved through JSON serialization), while
  `color-convert` returns 0° for genuine positive-zero inputs. Both values
  produce the same visible color since chroma is 0. Affects 1 of 272 routes.
