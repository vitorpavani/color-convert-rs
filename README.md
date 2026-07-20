# color-convert-rs

A **behavior-faithful Rust port** of the npm [`color-convert`](https://github.com/Qix-/color-convert)
library — GPU-accelerated with [CubeCL](https://github.com/tracel-ai/cubecl) and a native
Rust-SIMD CPU path — built **fully agentically** through a Red/Green/Blue TDD loop.

> **Status:** Production-ready. All 17 color models ported, 16 SIMD batch routes (f32x8),
> multi-core parallelism (rayon), GPU kernels (CubeCL), and a 10× measured speedup on the
> headline `rgb→lab` route. Every conversion is validated against JS-generated reference vectors.

## Why this exists

Two goals delivered:

1. **The port.** Reimplements `color-convert`'s conversion routes (RGB, HSL, HSV, CMYK, XYZ,
   LAB, LCH, OkLab, OkLCh, HWB, HCG, Apple, Gray, Hex, Keyword, Ansi16, Ansi256) in Rust.
   Correctness is validated against test vectors generated from the reference JS library, so
   outputs match within rounding tolerance.
2. **The process.** The whole build was driven by autonomous agents (an orchestrator that calls
   `red-dev` → `green-dev` → `blue-dev` per GitHub issue), measuring every step against both the
   JS baseline and the previous Rust iteration. The workflow itself is a first-class subject.

## Performance

Color-space conversion is embarrassingly parallel numeric work (matrix multiplies, `pow`/`cbrt`
over independent pixels). Three tiers are benchmarked head-to-head:

| Tier | Implementation |
|------|----------------|
| Baseline | `color-convert` on Node.js |
| CPU | Native Rust with `wide::f32x8` SIMD + `rayon` multi-core |
| GPU | CubeCL compute kernel (wgpu backend) |

A **runtime capability probe** selects GPU when a physical device is present, else the CPU-SIMD
path — one binary that runs on any server and never crashes for lack of a GPU. Results are
appended to a committed, diffable ledger (`benchmarks/results.jsonl`) so improvement is proven
over time.

### Headline numbers (N=50M pixels, 28-core host)

| Route | JS baseline | Rust single-core SIMD | Rust multi-core (rayon) | Cumulative speedup |
|-------|-------------|-----------------------|--------------------------|--------------------|
| rgb→lab | ~6 MP/s | **111.3 MP/s** | **164.0 MP/s** | **27× vs JS** |
| rgb→xyz | ~11 MP/s | **152.2 MP/s** | **176.3 MP/s** | **14× vs JS** |
| rgb→hsl | ~14 MP/s | **142.1 MP/s** | — | **10× vs JS** |
| rgb→oklab | ~8 MP/s | **65.5 MP/s** | — | **8× vs JS** |

### Optimization stack (10 waves, 33 kept / 7 dropped)

The `rgb→lab` single-core journey: **10.8 → 22.1 → 24.1 → 31.9 → 111.3 MP/s** (10.3× cumulative).

| Optimization | Effect |
|-------------|--------|
| f32x8 SIMD batch (wide crate) | 2× over f64x4 scalar |
| Fused rgb→xyz→lab single pass | +10.9% (drops intermediate buffer) |
| Vectorized srgb/LAB transfer (SIMD powf/cbrt) | +30.7% (lab), +21.2% (xyz) |
| sRGB inverse-gamma LUT (exact 256-entry) | 3.28× on xyz, 2.12× on lab |
| Fast cbrt (bit-hack + Newton-Raphson) | +63.7% on lab fused |
| Multi-core rayon par_batch | 1.42× on lab (memory-bandwidth-bound) |

Every numeric RGB-source route — both forward (rgb→X) and inverse (X→rgb) — has a vectorized
f32x8 SIMD path. The CPU optimization surface is genuinely exhausted; the remaining ceiling is
memory bandwidth, not CPU compute.

## Agentic development model

```
GitHub issue queue  ──▶  orchestrator
                            │
                            ├─▶ red-dev    write a failing test   🔴
                            ├─▶ green-dev  minimal code to pass   🟢
                            └─▶ blue-dev   refactor / review      🔵
                            │
                            ├─▶ measure (3-tier benchmark) → ledger
                            └─▶ log every step to the issue, then next issue
```

An `improvement-dev` agent proposes architectural / algorithmic changes, runs them through the
same R/G/B cycle, re-measures, and **keeps the change only if it beats both the JS baseline and
the previous Rust solution** — otherwise it is dropped and the negative result is recorded.
The loop ran for 10 waves across forward SIMD, inverse SIMD, multi-core parallelism, algorithmic
optimizations (LUT, fast cbrt), fused multi-hop convert, and GPU-tier parity.

See [`AGENTS.md`](./AGENTS.md) for the full development contract,
[`benchmarks/README.md`](./benchmarks/README.md) for the per-route benchmark rollup, and
[`docs/ARCHITECTURE_REVIEW.md`](./docs/ARCHITECTURE_REVIEW.md) for the architecture review.

## Architecture

```
src/
├── rgb.rs, hsl.rs, hsv.rs, hwb.rs, cmyk.rs, xyz.rs, lab.rs, lch.rs,
│   oklab.rs, oklch.rs, hcg.rs, apple.rs, gray.rs, hex.rs,
│   keyword.rs, color_name.rs, ansi16.rs, ansi256.rs
│       ── Scalar [f64;3] routes (faithful JS ports, one module per model)
├── convert.rs      ── BFS route graph + convert_batch (fused SIMD hop chaining)
├── simd.rs         ── f32x8 SIMD: rgb↔xyz↔lab + sRGB LUT + fast cbrt
├── simd_hsl.rs     ── SIMD rgb↔hsl (mask-blend hue)
├── simd_hsv.rs     ── SIMD rgb→hsv
├── simd_hsv_rgb.rs ── SIMD hsv→rgb
├── simd_cmyk.rs    ── SIMD rgb→cmyk
├── simd_hwb.rs     ── SIMD rgb→hwb
├── simd_hcg.rs     ── SIMD rgb↔hcg
├── simd_oklab.rs   ── SIMD rgb→oklab
├── simd_oklab_rgb.rs ── SIMD oklab→rgb
├── simd_apple.rs   ── SIMD rgb→apple
├── simd_xyz.rs     ── SIMD xyz→rgb (inverse)
├── simd_lab_xyz.rs ── SIMD lab→xyz (inverse)
├── simd_parallel.rs ── Generic rayon par_batch (multi-core dispatch)
├── gpu.rs          ── CubeCL kernels (rgb→lab + rgb→hsl/hsv/cmyk)
├── probe.rs        ── Runtime GPU capability probe
└── error.rs        ── thiserror error types
```

## Quick start

```bash
# Build (requires cc/ld; on NixOS: nix shell nixpkgs#gcc -c env ...)
cargo build --release

# Run the full test suite (107+ tests, all route vectors)
cargo test

# Benchmark (3 tiers)
BENCH_INPUT_SIZE=50000000 cargo run --release --bin bench_simd          # CPU SIMD
BENCH_INPUT_SIZE=50000000 cargo run --release --bin bench_simd_parallel # CPU multi-core
./run-bench-gpu.sh                                                       # GPU (needs Vulkan)

# Convert colors in code
use color_convert_rs::{convert, Model};
let lab = convert(Model::Rgb, Model::Lab, &[255, 128, 0]).unwrap();
```

## Reference

- Upstream library: [Qix-/color-convert](https://github.com/Qix-/color-convert) (MIT)
- GPU convention baseline: the author's `gpu-matmul-bench` (CubeCL)
- SIMD crate: [wide](https://github.com/Lokathor/wide) (`f32x8`)
- Multi-core: [rayon](https://github.com/rayon-rs/rayon)

## License

MIT — see [`LICENSE`](./LICENSE).
