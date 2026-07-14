# Architecture Review — color-convert-rs

**Author:** `improvement-dev` agent
**Date:** 2026-07-14
**Reviewed commit:** `a353aa6` (staging)
**Scope:** Post-MVP self-improvement review — survey the architecture, propose multiple
performance improvements, drive each through RED→GREEN→BLUE TDD, benchmark, and keep only what
beats both the JS baseline and the previous Rust iteration.

This is the standalone review artifact backing the self-improvement waves. The concrete
outcomes are tracked in GitHub issues #24, #25, #58, #61, #64, #65, #71, #72, #78, #79 and recorded in
`benchmarks/results.jsonl`; the rollup lives in [`benchmarks/README.md`](../benchmarks/README.md).

---

## 1. Architecture as surveyed

The crate is a behavior-faithful Rust port of npm `color-convert`, with two acceleration tiers
behind a runtime capability probe.

### 1.1 Layers

| Layer | Files | Role |
|-------|-------|------|
| **Scalar routes** | `rgb.rs`, `hsl.rs`, `hsv.rs`, `hwb.rs`, `cmyk.rs`, `xyz.rs`, `lab.rs`, `lch.rs`, `oklab.rs`, `oklch.rs`, `hcg.rs`, `apple.rs`, `gray.rs`, `hex.rs`, `ansi16.rs`, `ansi256.rs`, `keyword.rs`, `color_name.rs` | Per-pixel `[f64;3]`-array faithful JS ports. One module per source color model. |
| **Routing** | `convert.rs` | Public `convert(from, to, input)` API over a BFS route graph (`Graph` with 50 edges), memoized `path_cache`, held in a `thread_local` singleton. `Model` enum = the 17 color models. |
| **CPU-SIMD** | `simd.rs`, `simd_hsl.rs` | `wide::f32x8` batch paths for the hot matrix-heavy routes (rgb↔xyz↔lab, rgb↔hsl). |
| **GPU** | `gpu.rs` | CubeCL/wgpu batch compute kernel for rgb→lab; per-phase timing (upload/compute/readback). |
| **Probe** | `probe.rs` | Runtime `Backend::{Gpu, CpuSimd}` selection via wgpu adapter enumeration, guarded by `catch_unwind` so a GPU-less host never panics (Rule 5). |
| **Errors** | `error.rs` | `thiserror` enum; library code returns `Result<_, E>`, no `unwrap`/`expect` in library paths. |
| **Benchmark** | `bin/bench.rs`, `bin/bench_simd.rs`, `bin/bench_gpu.rs` | 3-tier harness (JS / CPU-SIMD / GPU) writing the append-only ledger. |

### 1.2 Hot path (the optimization surface)

The performance-critical routes are matrix + transcendental heavy and embarrassingly parallel
per pixel:

- **rgb→xyz**: sRGB inverse-gamma (`powf(2.4)` piecewise) → 3×3 matrix.
- **xyz→lab**: normalize → LAB transfer (`cbrt` piecewise) → scale.
- **rgb→lab**: the composition of the two (the headline route).
- **rgb→hsl** and the **rgb→hsl→rgb** round-trip: min/max/delta + a branchy hue selection.

These are exactly where SIMD and GPU pay off, and where all six improvement candidates were aimed.

### 1.3 Observations feeding the candidate list

1. The SIMD hot loops applied the piecewise **transfer functions scalar lane-by-lane**
   (`to_array()` → 8 scalar `powf`/`cbrt` calls → `f32x8::new()`), de-vectorizing the most
   expensive step. → candidate #65.
2. **rgb→hsl** was the heaviest per-pixel scalar route with **no SIMD path**. → candidate #58.
3. The **rgb→hsl→rgb** round-trip had a JS baseline but no SIMD path (no vectorized `hsl→rgb`).
   → candidate #64.
4. `rgb→lab` materialized an **intermediate `xyz` buffer** between the two SIMD steps. → #61.
5. Data-layout question: would **SoA** beat the interleaved **AoS** gather for the SIMD hot
   routes? → candidate #25.
6. Open question on the GPU: is the kernel **compute-bound** (so workgroup tuning helps)? → #24.

---

## 2. Proposed improvements & outcomes

Each was driven through the full RED→GREEN→BLUE cycle in its own worktree, benchmarked at N=50M
on the reference host (NVIDIA RTX 2000 Ada laptop, NixOS), and kept **only if it beat BOTH** the
JS baseline AND the previous Rust iteration.

| # | Improvement | Target route | Before → After @50M | Δ | Decision |
|---|-------------|--------------|---------------------|---|----------|
| #58 | SIMD `rgb→hsl` (f32x8 mask-blend hue) | rgb→hsl | 37.1 → **142.1** MP/s | **3.8×** | ✅ kept |
| #61 | Fuse `rgb→xyz→lab` single pass (drop intermediate xyz buffer) | rgb→lab | 21.7 → **24.1** MP/s | **+10.9%** | ✅ kept |
| #65 | Vectorize srgb/LAB piecewise transfer across f32x8 (SIMD `powf`/`cbrt` + mask-blend) | rgb→lab | 24.4 → **31.9** MP/s | **+30.7%** | ✅ kept |
| #65 | (same change) | rgb→xyz | 38.2 → **46.3** MP/s | **+21.2%** | ✅ kept |
| #64 | SIMD `hsl→rgb` + `rgb→hsl→rgb` round-trip | rgb→hsl→rgb | 21.0 → **65.0** MP/s | **3.1× (9.2× vs JS)** | ✅ kept |
| #71 | SIMD `rgb→hsv` (f32x8 mask-blend hue) | rgb→hsv | 38.9 → **144.7** MP/s | **3.72× (11.8× vs JS)** | ✅ kept |
| #72 | SIMD `rgb→cmyk` (f32x8 black-guard mask-blend) | rgb→cmyk | 63.8 → **130.1** MP/s | **2.04×** | ✅ kept |
| #78 | SIMD `rgb→hwb` (f32x8, reuses hsl hue mask-blend) | rgb→hwb | 44.0 → **146.4** MP/s | **3.33×** | ✅ kept |
| #79 | SIMD `rgb→oklab` (f32x8 powf/cbrt + dual matrix) | rgb→oklab | 9.1 → **31.9** MP/s | **3.51×** | ✅ kept |
| #25 | SoA vs AoS memory layout | rgb→lab | 22.1 → 20.2 MP/s | −8.6% | ❌ dropped |
| #24 | GPU workgroup `BLOCK_SIZE` sweep {32,64,128,256} | rgb→lab (gpu) | 33.6 → 32.7–34.0 MP/s | ±3% noise | ❌ dropped |

Waves 1–4 total: **8 kept, 2 dropped.**

### 2.1 Why the two drops are correct (not laziness)

- **#25 SoA dropped:** the AoS→SoA transpose (de-interleave + two extra allocations) costs more
  than the contiguous-load benefit at stride-3; the x86 prefetcher already handles the AoS gather
  well. Recorded as `decision:"reverted"`.
- **#24 GPU sweep dropped:** the kernel is **transfer-bound, not compute-bound** — per-phase
  timing shows compute flat at ~0.01ms while host→device upload dominates (67–79% of wall time
  and growing linearly with N). Every `BLOCK_SIZE` landed within a ±3% noise band, so tuning the
  workgroup optimizes a path that is already ~0.01ms. `BLOCK_SIZE=64` retained. The higher-leverage
  GPU work is memory staging / async upload-compute overlap, not workgroup size.

### 2.2 Cumulative result

**rgb→lab CPU-SIMD journey:** 10.8 (f64x4, #23) → 22.1 (f32x8, #51) → 24.1 (fused, #61) →
**31.9** (vectorized transfer, #65) MP/s at N=50M — a **2.95×** cumulative gain over the f64x4
baseline, every step measured and each kept only on a proven win.

**Wave-3 new SIMD routes:** `rgb→hsv` (#71) and `rgb→cmyk` (#72) each gained their first SIMD
path via the same f32x8 mask-blend pattern proven on `rgb→hsl` (#58) — 3.72× and 2.04× over their
scalar batch baselines respectively. Every route measured by the JS baseline now has a CPU-SIMD
path that beats it by ≥9×.

---

## 3. Correctness discipline

- Every SIMD path is verified against the scalar/JS reference within the pre-existing tolerances
  (`XYZ 5e-4`, `LAB 1e-3`, `HSL 1e-3`, `RGB 1e-3`). The #65 vectorized `powf`/`cbrt` did **not**
  loosen any tolerance to hide approximation error (Rule 8).
- Round-trip correctness (#64): `rgb→hsl→rgb` returns the original pixel within rounding tolerance.
- All expectations come from JS-generated vectors and the scalar reference, never hand-fudged.

---

## 4. Residual / future opportunities (not yet actioned)

These were identified during the review but **not** implemented (out of scope for this pass, or
lower expected leverage). Listed for the next self-improvement wave:

1. **GPU memory staging** (from the #24 analysis): pinned/zero-copy upload buffers and
   double-buffered async upload/compute overlap — attacks the real (transfer) bottleneck.
2. **SIMD for the remaining scalar routes**: `hsv` (#71), `cmyk` (#72), `hwb` (#78) and
   `oklab` (#79) are now done; `lch` / `oklch` batch paths still have no SIMD.
3. **GPU-tier coverage for the new SIMD routes**: every wave-1–4 CPU-SIMD record is
   `gpu_present:false` — the new routes (hsv, cmyk, hwb, oklab, hsl round-trip) have no GPU
   kernel and no `tier:"gpu"` measurement. Only `rgb→lab` has a GPU path. A future wave could
   add GPU kernels (or at least record GPU-tier numbers via `run-bench-gpu.sh`) for parity.
4. **Fused multi-hop `convert` for SIMD batches**: the BFS `Graph` chains scalar adapters; a
   batch fast-path for common multi-hop routes could avoid per-hop materialization.
5. **`find_path` has no covering test** (flagged by codegraph); the routing graph is exercised
   only indirectly via `convert`. A direct BFS-path unit test would harden it.

---

## 5. Contract compliance

- All work landed via squash-merged PRs off `staging` (never direct commits); Conventional Commits
  with TDD phase emojis; each improvement in its own `.worktrees/` isolate (Rule 11).
- No `unwrap`/`expect` in library code; `unsafe` only in `gpu.rs` with `// SAFETY:` comments.
- The ledger is append-only; dropped improvements are recorded as negative results, not deleted.
- Every step documented on its GitHub issue (#24, #25, #58, #61, #64, #65).
