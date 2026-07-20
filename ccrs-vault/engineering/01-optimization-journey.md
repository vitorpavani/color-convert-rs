---
title: "Optimization Journey — 10 Waves, 33 Kept / 7 Dropped"
date: 2026-07-20
tags: [optimization, simd, lut, cbrt, rayon, gpu, benchmark, self-improvement]
status: complete
prerequisites: "[[00-benchmark-methodology]]"
---

# 🚀 Optimization Journey — 10 Waves of Measured Improvement

**Date:** July 2026
**Status:** ✅ Complete — CPU optimization surface exhausted (SIMD + multi-core)
**Host:** NVIDIA RTX 2000 Ada laptop, NixOS, 28 cores

---

## The arc

Starting from a faithful scalar `[f64;3]` port of `color-convert`, 10 waves of autonomous
`improvement-dev` cycles pushed the headline `rgb→lab` route from **10.8 → 111.3 MP/s single-core**
(10.3×) and **164.0 MP/s multi-core** (15.2×). Every change was measured against both the JS baseline
and the previous Rust iteration; changes that didn't beat both were dropped and recorded as negative
results.

## rgb→lab single-core journey

| Wave | Issue | Optimization | MP/s @50M | Cumulative |
|------|-------|-------------|-----------|------------|
| — | #23 | f64x4 scalar baseline | 10.8 | 1.0× |
| 1 | #51 | f32x8 SIMD (wide crate) | 22.1 | 2.0× |
| 2 | #61 | Fused rgb→xyz→lab single pass | 24.1 | 2.2× |
| 2 | #65 | Vectorized srgb/LAB transfer | 31.9 | 3.0× |
| T1 | #113 | sRGB inverse-gamma LUT (exact 256-entry) | ~68 | 6.3× |
| T2 | #117 | Fast cbrt (bit-hack + Newton-Raphson) | **111.3** | **10.3×** |

## Wave summary

### Waves 1–5: Forward SIMD (rgb→X)

Every numeric RGB-*source* route got a vectorized `f32x8` path via `wide` crate mask-blend. **10
routes, all KEPT** (1.69×–3.94× each). Highlights: rgb→hsl 3.8×, rgb→oklab 3.51×, rgb→hsv 3.72×.

Dropped: SoA memory layout (#25, transpose overhead), GPU workgroup sweep (#24, transfer-bound).

### Waves 6–8: Inverse SIMD (X→rgb)

The mirror image — `xyz→rgb`, `lab→xyz`, `oklab→rgb`, `hsv→rgb`, `hcg→rgb`. **5 routes, all KEPT**
(1.70×–3.94×). Same mask-blend pattern, different direction. First inverse route (#97) also shipped
a forward-gamma helper (`srgb_fwd_f32x8`) reused by the others.

### Wave 9: Multi-core parallelism (rayon)

Generic `simd_parallel::par_batch(input, f)` wrapping `rayon::par_chunks(65536).flat_map(f).collect()`.
Nests SIMD lanes (8-wide per core) × cores (28). **13/16 routes KEPT** (1.09×–4.49×); 3 reverted
(cmyk/apple/lab→xyz — memory-bandwidth-bound, adding cores starves the bus).

Key finding: after the LUT + fast cbrt optimizations (Tier 1–2), rgb→lab became memory-bound too —
multi-core only gives **1.42×** instead of the expected 4×.

### Tier 1–3: Algorithmic optimizations

| Tier | Issue | Optimization | Result |
|------|-------|-------------|--------|
| 1 | #113 | sRGB gamma LUT (exact 256-entry, compile-time const) | ✅ KEPT — 3.28× on xyz |
| 1 | #114 | GPU double-buffering | ❌ DROPPED — transfer-bound |
| 2 | #117 | Fast cbrt (bit-hack + 2 Newton-Raphson iterations) | ✅ KEPT — +63.7% on lab |
| 2 | #118 | Fused multi-hop convert_batch (chains SIMD hops) | ✅ KEPT — 2.93× on lab→cmyk |
| 3 | #122 | Rayon chunk-size tuning | ❌ DROPPED — all 12 trials negative |
| 3 | #123 | GPU-tier parity (3 CubeCL kernels) | ✅ Implemented — GPU still < CPU |

## Key architectural decisions

### sRGB LUT (exact, lossless) — #113

The `srgb_inv_f32x8` function's `powf(2.4)` is the dominant cost in xyz/lab/oklab routes. The input
is `u8_channel / 255.0` — only **256 possible values**. A `const SRGB_INV_LUT: [f32; 256]` computed
at compile time maps each byte to its exact linear value. **Zero approximation error** — bit-identical
to the powf path, just dramatically faster (array lookup vs transcendental).

### Fast cbrt — #117

Replaced `wide::f32x8::cbrt()` with a bit-hack initial guess (reinterpret f32 as u32, divide
exponent by 3) + 2 Newton-Raphson iterations (`x = (2x + t/x²) / 3`). Accuracy: <1e-6 vs builtin
cbrt — far tighter than the LAB_TOLERANCE of 1e-3. The NR iterations are fully vectorized (f32x8
arithmetic); only the initial bit-hack is scalarized via `to_array()`.

### Memory-bandwidth wall

After the LUT + cbrt optimizations, the per-pixel compute dropped so much that rgb→lab shifted from
compute-bound to **memory-bandwidth-bound**. Multi-core (rayon) only gives 1.42× because the 28 cores
share one memory bus. This is the same wall the reverted routes (cmyk/apple) hit — it's fundamental,
not fixable by algorithmic changes.

## What's left

The CPU optimization surface is genuinely exhausted (SIMD + multi-core + algorithmic). The only
remaining high-leverage direction is orthogonal:

- **GPU memory staging** (pinned/zero-copy buffers to attack PCIe) — but the #24 analysis showed
  the GPU kernel is transfer-bound (compute=0.01ms, upload=78%), and #114 double-buffering
  confirmed there's nothing to overlap with.
- **Lookups/quantizers** (hex, keyword, ansi16/256, gray) — correctly excluded; SIMD gives nothing
  for table lookups.

See [[index]] for the full vault, or the [benchmark rollup](../benchmarks/README.md) for per-route
numbers.
