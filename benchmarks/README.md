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

<!-- Derived from results.jsonl. N=100,000 pixels, best-of-N (20 timed, 3 warmup),
     host: NVIDIA RTX 2000 Ada laptop. Higher MP/s is better. -->

| route | js baseline | cpu | gpu (CubeCL) | cpu vs js | gpu vs js |
|-------|-------------|-----|--------------|-----------|-----------|
| rgb→lab | 7.24 MP/s | 8.33 MP/s | **14.01 MP/s** | 1.15× | **1.94×** |
| rgb→hsl | 18.77 MP/s | 19.46 MP/s | — | 1.04× | — |
| rgb→xyz | — | 24.65 MP/s (SIMD) | — | — | — |

> GPU currently measured for `rgb→lab` (the matrix + gamma + LAB-transfer hot route). The GPU
> beats both the JS baseline and the CPU-SIMD path → `decision:"kept"`. Numbers are single-host
> snapshots for keep/revert decisions, not statistically rigorous criterion runs.
