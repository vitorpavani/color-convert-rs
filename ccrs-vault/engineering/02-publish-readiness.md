---
tags: [engineering, publish, npm, wasm]
updated: 2026-07-20
---

# Publish Readiness — #131–#135, wasm-pack, npm

## Overview

Five issues shipped in PR [#136](https://github.com/vitorpavani/color-convert-rs/pull/136) to
make the crate publishable on both crates.io and npm. Oracle-verified with 4 follow-up fixes.

## What shipped

### #131 — Feature gating + Cargo.toml metadata

Made `wgpu`, `pollster`, `cubecl`, `bytemuck` optional behind `gpu` feature. Added `wasm` and
`image` features. Default install pulls 4 deps (thiserror, wide, rayon, bytemuck-transitive).

`cargo publish --dry-run --no-default-features` succeeds: 172 files, clean package.

See [[ADR-003]].

### #132 — npm drop-in replacement via wasm-pack

- `src/wasm.rs`: `convert_route` + `convert_route_raw` wasm-bindgen exports
- `js/index.js`: nested `convert.rgb.hsl(r,g,b)` API matching color-convert exactly
- `js/test-parity.mjs`: **272/272 routes match** color-convert@3.1.3
- Published to npm as [`color-convert-rs`](https://www.npmjs.com/package/color-convert-rs)

Known gap: `gray.lch` hue (180° vs 0°) — documented in CHANGELOG.

### #133 — Examples

- `examples/basic_convert.rs` — single-color demo
- `examples/batch_simd.rs` — 100k-pixel SIMD benchmark (26M px/s)
- `examples/image_to_lab.rs` — full image pipeline with throughput

### #134 — CI

`.github/workflows/ci.yml`: test + clippy + fmt on `--no-default-features` and `--features gpu`,
plus wasm-pack build check + JS parity test.

### #135 — CHANGELOG + rustdoc

CHANGELOG.md with v0.1.0 entry, Known Limitations section. Crate-level rustdoc with Quick Start
doctest, Features section, Public API listing.

## Oracle review findings (4 fixes applied)

1. **Exclude list incomplete** — `.github/`, `benchmarks/`, `js/` were leaking into the crate.
   Fixed: added to `exclude` in Cargo.toml.
2. **CI didn't run JS parity** — wasm-build job only checked compilation. Fixed: added
   `npm install && npm test` step.
3. **gray.lch undocumented** — CHANGELOG now has "Known limitations" section.
4. **Doctest `no_run`** — assertion never ran. Removed `no_run`; runs on CI.

## Batch SIMD exports (post-publish)

After npm publish, added 8 batch wasm exports for the routes where SIMD wins:

```js
convert.rgb.lab.batch(uint8Array)  // → Float32Array, 6.8× faster than JS loop
convert.rgb.xyz.batch(uint8Array)  // → Float32Array, 7.6× faster
```

See [[ADR-004]] for the full performance analysis.

## flake.nix

Added NixOS dev shell (`nix develop`) with gcc, wasm-pack, nodejs. Sets
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER` and `-fuse-ld=bfd` to bypass the broken
`ld-wrapper.sh` in the rustup toolchain.

## Commits

- `3df8e70` chore: Cargo.toml metadata + feature-gate GPU deps (#131)
- `e80b32b` feat: npm drop-in replacement via wasm-pack (#132)
- `a230b14` docs: examples (#133)
- `1528cd7` ci+docs: CI + CHANGELOG + rustdoc (#134, #135)
- `303eb79` fix: Oracle review fixes
- `9a415be` feat: batch SIMD exports
- `dbfb3a0` chore: flake.nix dev shell
