---
adr: 003
title: Feature gating — gpu, wasm, image behind opt-in flags
status: accepted
date: 2026-07-20
tags: [adr, architecture, publish, features]
---

# ADR-003: Feature Gating

## Context

The crate has three distinct user profiles with different dependency needs:

1. **Rust image processing** — wants CPU SIMD (`wide`, `rayon`), no GPU deps
2. **GPU compute** — wants CubeCL/wgpu kernels
3. **npm/wasm** — wants wasm-bindgen exports, no system GPU deps

Before feature gating, `cargo build` pulled in `wgpu`, `cubecl`, `pollster`, `bytemuck`
unconditionally — 300+ transitive crates, Vulkan headers required, ~60s compile time. This
blocked crates.io publish (heavy deps deter adoption) and broke the "lightweight library" value
prop.

## Decision

Three independent feature flags, `default = []` (CPU-only):

```toml
[features]
default = []
gpu = ["dep:wgpu", "dep:pollster", "dep:cubecl", "dep:bytemuck"]
wasm = ["dep:wasm-bindgen", "dep:js-sys"]
image = ["dep:image"]
```

- **`gpu`**: gates `src/gpu.rs` and the wgpu-dependent half of `src/probe.rs`. Without it,
  `probe()` returns `Backend::CpuSimd` unconditionally — no wgpu import, no Vulkan needed.
- **`wasm`**: gates `src/wasm.rs`. Used only by `wasm-pack build --features wasm`.
- **`image`**: gates the `batch::image` submodule. Adds `image` crate for `DynamicImage` integration.

All 5 bench binaries and the `gpu_kernel` integration test use `required-features = ["gpu"]`.

## Consequences

**Positive:**
- `cargo build` = 4 deps (thiserror, wide, rayon, bytemuck-transitive), ~3s compile
- `cargo publish --dry-run` succeeds with a clean package
- Users opt into heavy deps only when needed
- CI runs two matrix lines: `--no-default-features` (CPU) and `--features gpu` (full)

**Negative:**
- `probe.rs` has `#[cfg]` duplication (two `probe()` implementations)
- Users wanting GPU + image need `--features gpu,image`
- The `cdylib` crate-type (for wasm-pack) produces an unused `.so` on native targets

## Verification

- `cargo build --no-default-features`: 4 deps, ~1s ✅
- `cargo build --features gpu`: full GPU path ✅
- `cargo build --features wasm`: wasm-bindgen exports ✅
- `cargo build --features image`: image crate integration ✅
- `cargo publish --dry-run --no-default-features`: 172 files, clean package ✅

## References

- Issue [#131](https://github.com/vitorpavani/color-convert-rs/issues/131)
- PR [#136](https://github.com/vitorpavani/color-convert-rs/pull/136)
- [[02-publish-readiness]]
