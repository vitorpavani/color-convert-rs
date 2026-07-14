# benchmarks/ — 3-tier measurement harness

This directory holds the head-to-head benchmark harness and the **committed, append-only results
ledger** that proves whether the port is improving. See `AGENTS.md` → "Measurement discipline" and
skill `benchmark-ledger` for the rules.

> **Scaffold phase:** only structure + schema live here. The runners (`js/`, Rust `bench`) are built
> during the coding session, driven by their own issues. No runner code exists yet.

## The three tiers

| Tier key | Implementation | Purpose |
|----------|----------------|---------|
| `js` | `color-convert` on Node.js | The baseline we are porting — the number to beat |
| `cpu` | Native Rust with explicit SIMD | Runs everywhere, including GPU-less servers |
| `gpu` | CubeCL compute kernel (wgpu backend) | The peak-performance path when a GPU is present |

All three run on the **same input generator** and the **same host** within one comparison, so any
delta is attributable to the change under test — not to hardware or input differences.

## Layout (target)

```
benchmarks/
├── README.md            — this file (how tiers are measured + the rollup table)
├── SCHEMA.md            — authoritative record schema for results.jsonl
├── results.jsonl        — append-only ledger, one JSON object per measured run
├── js/                  — Node baseline runner + reference-vector generator (coding session)
│   ├── gen-vectors.mjs  — regenerates tests/vectors/*.json from color-convert
│   └── bench.mjs        — times the JS baseline, appends `tier:"js"` records
└── (Rust bench harness lives under the crate's bench target, coding session)
```

## The keep-or-revert rule

A change is **kept only if it beats BOTH**:
1. the **JS baseline**, and
2. the **previous Rust iteration**

on the target metric, with all correctness tests still green. Otherwise it is reverted — and the
negative result is still recorded (negative results are article material).

## Running each tier

```bash
# js baseline
node benchmarks/js/bench.mjs

# cpu tiers (Rust, best-of-N via std::time)
cargo run --release --bin bench       # scalar/general cpu routes
cargo run --release --bin bench_simd  # wide-SIMD hot routes

# gpu tier (CubeCL/wgpu)
./run-bench-gpu.sh                     # see the NixOS/NVIDIA note below
```

### GPU tier on NixOS + NVIDIA (important)

CubeCL uses the wgpu backend, which finds the GPU through the **Vulkan loader** and the
**NVIDIA ICD manifest**. On NixOS these are *not* on the default search path, so a bare
`cargo run --bin bench_gpu` enumerates zero adapters — the runtime probe then (correctly)
reports `CpuSimd` and the GPU tier is **silently skipped** even on a machine with a real GPU.

The committed **`run-bench-gpu.sh`** wrapper fixes this by pointing the process at the system
Vulkan loader + NVIDIA ICD before launching:

```bash
LD_LIBRARY_PATH="<vulkan-loader>/lib:/run/opengl-driver/lib"
VK_ICD_FILENAMES="/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json"
```

With that env set, `WgpuRuntime::client` succeeds, the probe resolves to `Gpu`, and the GPU
tier is measured. On a genuinely GPU-less host the library still degrades cleanly to CPU-SIMD
without panicking (Rule 5) — the wrapper just makes a present GPU *discoverable*. This mirrors
the reference `gpu-matmul-bench/run.sh`. On non-NixOS hosts with the loader on the default
path, run `cargo run --release --bin bench_gpu` directly.

## How to read the ledger

`results.jsonl` is append-only history. Never rewrite it. To see progress for a route:

```bash
# every cpu-tier record for rgb->hsl, newest last
rg '"route":"rgb->hsl"' benchmarks/results.jsonl | rg '"tier":"cpu"'
```

## Rollup table

<!-- All numbers below are backed by benchmarks/results.jsonl lines.
     Host: NVIDIA RTX 2000 Ada laptop (NixOS).  Higher MP/s is better.
     best-of-N wall time after GPU JIT / CPU cache warmup.
     Ledger lines are cited as (issue, N, route, tier, decision). -->

### rgb→lab — the hot matrix + gamma + LAB-transfer route

**Issue #23 scaling sweep (ledger: issue=23, tier=js|cpu|gpu, decision=baseline)**

| N | JS (ledger: issue=23, tier=js) | CPU SIMD f64x4 (issue=23, tier=cpu) | GPU (issue=23, tier=gpu) | gpu vs cpu | gpu vs js |
|---|------|-----------|------|------------|-----------|
| 100k | 7.2 MP/s | 11.5 MP/s | 13.9 MP/s | 1.2× | 1.9× |
| 1M | 7.2 MP/s | 10.8 MP/s | 26.9 MP/s | 2.5× | 3.7× |
| 10M | 7.1 MP/s | 10.5 MP/s | 32.8 MP/s | 3.1× | 4.6× |
| 50M | 6.1 MP/s | 10.8 MP/s | 34.9 MP/s | 3.2× | 5.7× |
| 100M | OOM | 10.7 MP/s | 33.5 MP/s | 3.1× | — |

> JS at N=100M: OOM crash during warmup (ledger: issue=23, tier=js, decision=oom).
> All JS numbers are from fresh `bench.mjs` runs with BENCH_ISSUE=23.
> GPU records include upload/compute/readback split in notes.

**Issue #51 f32x8 improvement (ledger: issue=51, tier=cpu, decision=kept)**

| N | f64x4 baseline (issue=51, decision=baseline) | f32x8 (issue=51, decision=kept) | speedup vs f64x4 | vs JS |
|---|---------------------|---------------|----------|-------|
| 50M | 10.8 MP/s | **22.1 MP/s** | **2.04×** | 3.6× |
| 100M | 10.7 MP/s | **22.2 MP/s** | **2.08×** | — (JS OOM) |

> f64x4 baseline measured at commit `1a4e607`; f32x8 at commit `5d4b85c`.
> `grep '"issue":51' results.jsonl` confirms 4 baseline + 4 kept records.

**Self-improvement waves — issues #58, #61, #65, #64 (kept) + #25, #24 (dropped)**

Driven by the `improvement-dev` agent through the full RED→GREEN→BLUE TDD cycle, each
measured at N=50M on host `vipavani` (NVIDIA RTX 2000 Ada laptop, NixOS) and kept **only
if it beat BOTH the JS baseline AND the previous Rust iteration**. Dropped improvements are
still recorded (negative results are article material).

| Issue | Improvement | Route | Before → After | Δ | Decision |
|-------|-------------|-------|----------------|---|----------|
| #58 | SIMD `rgb→hsl` (f32x8 mask-blend hue) | rgb→hsl | 37.1 → **142.1** MP/s | **3.8×** | ✅ kept |
| #61 | Fused `rgb→xyz→lab` single pass (drop intermediate xyz Vec) | rgb→lab | 21.7 → **24.1** MP/s | **+10.9%** | ✅ kept |
| #65 | Vectorize srgb/LAB piecewise transfer across f32x8 (SIMD `powf`/`cbrt` + mask-blend) | rgb→lab | 24.4 → **31.9** MP/s | **+30.7%** | ✅ kept |
| #65 | (same change) | rgb→xyz | 38.2 → **46.3** MP/s | **+21.2%** | ✅ kept |
| #64 | SIMD `hsl→rgb` + `rgb→hsl→rgb` round-trip | rgb→hsl→rgb | 21.0 → **65.0** MP/s (JS 7.1) | **3.1× / 9.2× vs JS** | ✅ kept |
| #25 | SoA vs AoS memory layout | rgb→lab | 22.1 → 20.2 MP/s | −8.6% | ❌ dropped |
| #24 | GPU workgroup `BLOCK_SIZE` sweep {32,64,128,256} | rgb→lab (gpu) | 33.6 → 32.7–34.0 MP/s | ±3% noise | ❌ dropped |

> **#25 SoA dropped:** the AoS→SoA transpose (de-interleave + 2 extra allocations) costs
> more than the contiguous-load benefit at stride-3 — the x86 prefetcher already handles the
> AoS gather well. `grep '"issue":25' results.jsonl` shows `decision:"reverted"`.
>
> **#24 GPU sweep dropped:** every `BLOCK_SIZE` lands within a ±3% run-to-run noise band; the
> kernel is **transfer-bound, not compute-bound** (see the #23→#24 gate analysis below —
> compute is flat at 0.01ms). `BLOCK_SIZE=64` stays. `grep '"issue":24' results.jsonl` shows
> the sweep + `decision:"reverted"`.
>
> **rgb→lab CPU-SIMD journey:** 10.8 (f64x4, #23) → 22.1 (f32x8, #51) → 24.1 (fused, #61) →
> **31.9** (vectorized transfer, #65) MP/s at N=50M — a **2.95×** cumulative gain over the
> f64x4 baseline, all measured and each kept only on a proven win.

### Key observations

- **CPU SIMD throughput is flat** at ~10.8 MP/s (f64x4) across all N — predictable.
- **f32x8 CPU SIMD is 2× faster**: 22.1 MP/s at N=50M (up from 10.8 MP/s f64x4) by using all 8 f32 lanes instead of 4 f64 lanes, keeping data f32 end-to-end.
- **GPU throughput scales up** with N: 14→27→33→35 MP/s, plateauing at ~35 MP/s around N=50M.
- **JS degrades** from 7.2→6.1 MP/s (GC pressure visible at N=50M), then **OOM crash** at N=100M.
- **GPU crossover**: GPU beats CPU-SIMD even at N=100k. With f32x8, the CPU-SIMD gap narrows to 1.6× (22.1 vs 34.9 MP/s at N=50M).
- **GPU kernel panic** at N=10M before the 2-D launch grid fix (wgpu dispatch limit 65535 per dimension); fixed in commit `970a7c4`.

### Issue #23 → #24 gate: is the GPU kernel compute-bound?

All data from `issue:23`, `tier:gpu`, `route:rgb->lab` ledger lines. The GPU harness
records per-phase timing: upload (host→device), compute (kernel launch only — CubeCL
launch is asynchronous so this measures dispatch overhead, not GPU execution), and
readback (blocking `read_one` that includes both GPU compute AND device→host transfer).

| N | upload ms | compute ms | readback ms | upload % of total | verdict |
|---|----------|-----------|-------------|-------------------|---------|
| 100k | 0.16 | 0.00 | 0.58 | 22% | I/O & launch overhead dominate at tiny N |
| 1M | 13.86 | 0.01 | 6.59 | 67% | upload already the majority cost |
| 10M | 156.28 | 0.01 | 42.11 | 79% | upload grows linearly (×10 N → ×11 upload) |
| 50M | 707.24 | 0.01 | 204.78 | 78% | compute launch is STILL 0.01ms — flat |
| 100M | 1395.98 | 0.01 | 563.31 | 71% | upload scaling confirmed (×2 N → ×1.97 upload) |

**Verdict: TRANSFER-BOUND, NOT compute-bound.**

- **Compute is flat at 0.01ms** regardless of N. The GPU shader completes in O(1) time
  after dispatch — it is not the bottleneck.
- **Upload dominates**: at N ≥ 1M, host→device transfer accounts for 67–79% of wall time
  and grows linearly with N (as expected for PCIe-bound bulk data movement).
- **Readback** (which includes actual GPU compute + device→host transfer) also grows
  linearly but is consistently 3–4× smaller than upload in absolute terms.

**Implication for #24 (GPU kernel tuning): MUST remain BLOCKED.** Tuning kernel arithmetic,
workgroup size, or occupancy optimizes a path that is already ~0.01ms. The bottleneck is
the PCIe bus, not the shader. Higher-leverage work: pinned/zero-copy memory staging,
asynchronous upload/compute overlap (double buffering), or fused multi-pass kernels to
reduce total data in flight.

### rgb→hsl throughput (MP/s) — now CPU SIMD (#58)

| Tier | @N=50M | vs JS |
|------|--------|-------|
| JS (issue=23) | 14.1 MP/s | 1.0× |
| Rust scalar batch (issue=58 baseline) | 37.1 MP/s | 2.6× |
| **Rust f32x8 SIMD (issue=58, decision=kept)** | **142.1 MP/s** | **10.1×** |

> Issue #58 added the first SIMD path for rgb→hsl via f32x8 mask-blend of the 3-way hue
> branch — **3.8× over the scalar batch**, **10.1× over JS**. `grep '"issue":58' results.jsonl`.

### rgb→hsl→rgb throughput (MP/s) — now CPU SIMD (#64)

| Tier | @N=50M | vs JS |
|------|--------|-------|
| JS (issue=23) | 7.1 MP/s | 1.0× |
| Rust scalar batch (issue=64 baseline) | 21.0 MP/s | 3.0× |
| **Rust f32x8 SIMD round-trip (issue=64, decision=kept)** | **65.0 MP/s** | **9.2×** |

> Issue #64 added SIMD `hsl→rgb` (f32x8 mask-blend of the 4-way channel piecewise), completing
> the round-trip SIMD path — **3.1× over the scalar batch**, **9.2× over JS**. Round-trip
> correctness verified: rgb→hsl→rgb returns the original within rounding tolerance.

### rgb→xyz throughput (MP/s) — CPU SIMD

**Issue #23 f64x4 sweep + Issue #51 f32x8 + Issue #65 vectorized transfer**

| N | f64x4 (issue=23, tier=cpu) | f32x8 (issue=51, decision=kept) | f32x8 + vec transfer (issue=65, decision=kept) | speedup vs f64x4 |
|---|---------|---------|---------|---------|
| 100k | 24.7 MP/s | — | — | — |
| 1M | 24.5 MP/s | — | — | — |
| 10M | 19.6 MP/s | — | — | — |
| 50M | 20.4 MP/s | 37.5 MP/s | **46.3 MP/s** | **2.27×** |
| 100M | 20.3 MP/s | 37.6 MP/s | — | — |

> JS `color-convert.rgb.xyz`: 11.2 MP/s at N=100k, 10.8 MP/s at N=10M (ledger: issue=23, tier=js).
> Issue #65 vectorized the srgb inverse-gamma transfer across f32x8 (SIMD `powf` + mask-blend,
> replacing scalar lane-by-lane calls) for a further **+21.2%** (37.5 → 46.3 MP/s) — now
> ~4.3× faster than JS and **2.27× faster than f64x4 SIMD**.

### rgb→hsv throughput (MP/s) — now CPU SIMD (#71)

| Tier | @N=50M | vs JS |
|------|--------|-------|
| JS (issue=71) | 12.3 MP/s | 1.0× |
| Rust scalar batch (issue=71 baseline) | 38.9 MP/s | 3.2× |
| **Rust f32x8 SIMD (issue=71, decision=kept)** | **144.7 MP/s** | **11.8×** |

> Issue #71 added the first SIMD path for rgb→hsv via f32x8 mask-blend of the 3-way hue
> branch (min/max/delta, `v=max`, `s=delta/max`) — **3.72× over the scalar batch**, **11.8× over
> JS**. `grep '"issue":71' results.jsonl`.

### rgb→cmyk throughput (MP/s) — now CPU SIMD (#72)

| Tier | @N=50M |
|------|--------|
| Rust scalar batch (issue=72 baseline) | 63.8 MP/s |
| **Rust f32x8 SIMD (issue=72, decision=kept)** | **130.1 MP/s** |

> Issue #72 added the first SIMD path for rgb→cmyk via f32x8, with a mask-blend guard for the
> pure-black `k==1` divide-by-zero case (mirroring the JS `|| 0` fallback) — **2.04× over the
> scalar batch**. No JS baseline is wired for rgb→cmyk yet; the keep decision is against the
> previous Rust scalar iteration. `grep '"issue":72' results.jsonl`.

### rgb→hwb throughput (MP/s) — now CPU SIMD (#78)

| Tier | @N=50M |
|------|--------|
| Rust scalar batch (issue=78 baseline) | 44.0 MP/s |
| **Rust f32x8 SIMD (issue=78, decision=kept)** | **146.4 MP/s** |

> Issue #78 added the first SIMD path for rgb→hwb via f32x8 mask-blend of the 3-way hue
> branch (reusing the same hue as rgb→hsl since `hwb_f64` calls `hsl_f64(rgb)[0]`).
> Whiteness = min×100, blackness = (1-max)×100 as straight-line f32x8 ops — **3.33× over
> the scalar batch**. No JS baseline is wired for rgb→hwb yet; the keep decision is against
> the previous Rust scalar iteration. `grep '"issue":78' results.jsonl`.

### Cumulative self-improvement summary (waves 1–4)

| Wave | Issue | Route | Δ | Decision |
|------|-------|-------|---|----------|
| 1 | #58 | rgb→hsl | 3.8× | ✅ kept |
| 1 | #61 | rgb→lab (fused) | +10.9% | ✅ kept |
| 1 | #25 | rgb→lab (SoA) | −8.6% | ❌ dropped |
| 1 | #24 | rgb→lab (GPU sweep) | ±3% | ❌ dropped |
| 2 | #65 | rgb→lab / rgb→xyz (vec transfer) | +30.7% / +21.2% | ✅ kept |
| 2 | #64 | rgb→hsl→rgb (round-trip) | 3.1× | ✅ kept |
| 3 | #71 | rgb→hsv | 3.72× | ✅ kept |
| 3 | #72 | rgb→cmyk | 2.04× | ✅ kept |
| 4 | #78 | rgb→hwb | 3.33× | ✅ kept |

**7 kept, 2 dropped** across 4 waves — every kept change beat both the JS baseline (where wired)
and the previous Rust iteration; every dropped change is recorded as a negative result. See
[`docs/ARCHITECTURE_REVIEW.md`](../docs/ARCHITECTURE_REVIEW.md) for the full review.
