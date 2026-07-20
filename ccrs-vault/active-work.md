---
tags: [dashboard]
aliases:
  - Active Work
  - Open Issues
updated: 2026-07-20
---

# Active Work

Live view of project state. The optimization drive is complete; the focus shifted to
publish-readiness, npm package, and image processing positioning.

---

## ✅ Project Status: Published

All 17 color models ported. 16 SIMD batch routes (f32x8). Multi-core parallelism (rayon).
GPU kernels (CubeCL). sRGB LUT + fast cbrt. 10-wave optimization drive complete.

**Published:**
- npm: [`color-convert-rs`](https://www.npmjs.com/package/color-convert-rs) — 272/272 routes
  match color-convert@3.1.3, batch SIMD 2–8× faster than JS loops
- crates.io: pending (see below)

**Headline:** rgb→lab **111.3 MP/s** single-core (10.3×), **164.0 MP/s** multi-core (15.2×).
Full HD frame (1920×1080) in <30ms across all routes. See [[01-optimization-journey]].

## What shipped this session

| Deliverable | Issue/PR | Status |
|-------------|----------|--------|
| Feature gating (gpu/wasm/image) | [#131](https://github.com/vitorpavani/color-convert-rs/issues/131) / PR [#136](https://github.com/vitorpavani/color-convert-rs/pull/136) | ✅ Merged |
| npm drop-in via wasm-pack | [#132](https://github.com/vitorpavani/color-convert-rs/issues/132) | ✅ Published to npm |
| Examples (Rust + JS) | [#133](https://github.com/vitorpavani/color-convert-rs/issues/133) | ✅ |
| CI (test + clippy + fmt + wasm + JS parity) | [#134](https://github.com/vitorpavani/color-convert-rs/issues/134) | ✅ |
| CHANGELOG + rustdoc | [#135](https://github.com/vitorpavani/color-convert-rs/issues/135) | ✅ |
| Batch SIMD wasm exports (2–8× JS) | `9a415be` | ✅ |
| Stride-aware batch API + image crate | `ca698a8` | ✅ |
| flake.nix dev shell | `dbfb3a0` | ✅ |
| README repositioned for image processing | `7c0a912` | ✅ |

## 🚨 Blockers

| Issue | Title | Waiting on |
|-------|-------|------------|
| _(none)_ | | |

## 📋 Next opportunities

| Opportunity | Effort | Value |
|-------------|--------|-------|
| Hybrid npm (JS single-color + wasm batch) | M | Eliminates the 7–25× single-color penalty |
| RGBA output with alpha pass-through | S | Compositing pipelines |
| In-place conversion (caller-provided buffer) | S | Avoid allocation for large images |
| YCbCr/YUV + Rec.709/Rec.2020 | L | Video pipeline support |
| `palette` crate interop | M | Bridge type-safe color to fast batch |

## 📊 Optimization Drive Summary (10 waves, 33 kept / 7 dropped)

| Wave | Scope | Kept | Dropped |
|------|-------|------|---------|
| 1–5 | Forward SIMD (rgb→X) | 10 | 2 (SoA #25, GPU sweep #24) |
| 6–8 | Inverse SIMD (X→rgb) | 5 | 0 |
| 9 | Multi-core (rayon) | 13 | 3 (cmyk/apple/lab→xyz) |
| T1–T3 | Algorithmic (LUT, cbrt, fused convert, GPU parity) | 5 | 2 (double-buffer #114, chunk tuning #122) |

See [[01-optimization-journey]] for the full per-wave breakdown.

---

See [[index]] to navigate the full vault.
