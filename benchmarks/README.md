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

> f64x4 baseline measured at commit `0aa8737`; f32x8 at commit `5d4b85c`.
> `grep '"issue":51' results.jsonl` confirms 4 baseline + 4 kept records.

### Key observations

- **CPU SIMD throughput is flat** at ~10.8 MP/s (f64x4) across all N — predictable.
- **f32x8 CPU SIMD is 2× faster**: 22.1 MP/s at N=50M (up from 10.8 MP/s f64x4) by using all 8 f32 lanes instead of 4 f64 lanes, keeping data f32 end-to-end.
- **GPU throughput scales up** with N: 14→27→33→35 MP/s, plateauing at ~35 MP/s around N=50M.
- **JS degrades** from 7.2→6.1 MP/s (GC pressure visible at N=50M), then **OOM crash** at N=100M.
- **GPU crossover**: GPU beats CPU-SIMD even at N=100k. With f32x8, the CPU-SIMD gap narrows to 1.6× (22.1 vs 34.9 MP/s at N=50M).
- **Transfer-vs-compute**: Upload (host→device) consumes ~50% of GPU wall time across all N > 100k. The kernel is **transfer-bound** — see issue #23→#24 gate analysis below.
- **GPU kernel panic** at N=10M before the 2-D launch grid fix (wgpu dispatch limit 65535 per dimension); fixed in commit `970a7c4`.

### rgb→hsl throughput (MP/s)

**Issue #23 JS sweep (ledger: issue=23, tier=js)**

| N | JS (issue=23, tier=js) |
|---|---------------------------|
| 100k | 18.7 MP/s |
| 1M | 18.4 MP/s |
| 10M | 18.5 MP/s |
| 50M | 14.1 MP/s |
| 100M | OOM (ledger: decision=oom) |

> No GPU or SIMD path for rgb→hsl yet. JS GC-driven degradation visible at N=50M (14.1 MP/s).
> CPU scalar measurements omitted — compiler elimination suspected (see AGENTS.md).

### rgb→xyz throughput (MP/s) — CPU SIMD

**Issue #23 f64x4 sweep + Issue #51 f32x8**

| N | f64x4 (issue=23, tier=cpu) | f32x8 (issue=51, tier=cpu, decision=kept) | speedup |
|---|---------|---------|---------|
| 100k | 24.7 MP/s | — | — |
| 1M | 24.5 MP/s | — | — |
| 10M | 19.6 MP/s | — | — |
| 50M | 20.4 MP/s | **37.5 MP/s** | **1.84×** |
| 100M | 20.3 MP/s | **37.6 MP/s** | **1.85×** |

> JS `color-convert.rgb.xyz`: 11.2 MP/s at N=100k, 10.8 MP/s at N=10M (ledger: issue=23, tier=js).
> f32x8 SIMD is ~3.5× faster than JS and ~1.85× faster than f64x4 SIMD.
