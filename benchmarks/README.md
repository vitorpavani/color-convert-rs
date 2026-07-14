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

<!-- Scaling sweep (issue #23).  Host: NVIDIA RTX 2000 Ada laptop (NixOS).
     CPU tier = wide::f64x4 SIMD batch (bench_simd).  GPU tier = CubeCL/wgpu (bench_gpu).
     best-of-N wall time after GPU JIT / CPU cache warmup.  Higher MP/s is better.
     JS at N=100M → OOM crash (GC wall). -->

### rgb→lab throughput (MP/s) — the hot matrix + gamma + LAB-transfer route

| N | JS baseline | CPU SIMD (f64x4) | GPU (CubeCL) | gpu vs cpu | gpu vs js |
|---|-------------|-----------------|--------------|------------|-----------|
| 100k | 7.3 | 11.5 | 13.9 | 1.2× | 1.9× |
| 1M | 7.1 | 10.8 | 26.9 | 2.5× | 3.8× |
| 10M | 7.0 | 10.5 | 32.8 | 3.1× | 4.7× |
| 50M | 6.1 | 10.8 | 34.9 | 3.2× | 5.7× |
| 100M | OOM | 10.7 | 33.5 | 3.1× | — |

### Key observations

- **CPU SIMD throughput is flat** at ~10.8 MP/s across all N — predictable, no degradation.
- **GPU throughput scales up** with N: 14→27→33→35 MP/s as batch size amortizes launch overhead,
  plateauing at ~35 MP/s around N=50M.
- **JS degrades** from 7.3→6.1 MP/s (GC pressure visible at N=50M), then **OOM crash** at N=100M.
- **GPU crossover**: GPU beats CPU-SIMD even at N=100k (the smallest tested N). The real crossover
  is at N < 100k (where GPU launch/setup overhead may make it slower — not measured here).
- **Transfer-vs-compute**: Upload (host→device) consumes ~50% of GPU wall time across all N > 100k.
  The kernel is **transfer-bound**, not compute-bound. Tuning kernel arithmetic alone will yield
  diminishing returns; reducing transfer overhead (pinned memory, async overlap, fused ops) is
  higher-leverage. See issue #23→#24 gate analysis below.
- **GPU kernel panic** at N=10M before the 2-D launch grid fix (wgpu dispatch limit 65535 per
  dimension); fixed in commit `970a7c4`.

### rgb→hsl throughput (MP/s)

| N | JS baseline | CPU (scalar) |
|---|-------------|-------------|
| 100k | 18.0 | — |
| 1M | 18.2 | — |
| 10M | 18.3 | — |
| 50M | 14.3 | — |
| 100M | OOM | — |

> No GPU or SIMD path for rgb→hsl yet. JS shows GC-driven degradation at 50M (14.3 MP/s vs 18.3
> at 10M). CPU scalar measurements are unreliable (compiler elimination suspected) — see bench
> harness note in `AGENTS.md`.

### rgb→xyz throughput (MP/s) — CPU SIMD

| N | CPU SIMD (f64x4) |
|---|-----------------|
| 100k | 24.7 |
| 1M | 24.5 |
| 10M | 19.6 |
| 50M | 20.4 |
| 100M | 20.3 |

> SIMD throughput for rgb→xyz is stable at 20–25 MP/s. No JS baseline available (JS
> `color-convert.rgb.xyz` at 100k = 11.1 MP/s, 10M = 10.7 MP/s — SIMD is ~2× faster).
