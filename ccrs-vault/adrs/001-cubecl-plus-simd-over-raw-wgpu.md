---
title: "ADR-001: CubeCL + native Rust-SIMD over raw wgpu for the GPU/CPU split"
status: accepted
date: 2026-07-13
decision-makers: [Vitor Amorim Pavan]
tags: [adr, gpu, cubecl, wgpu, simd, cpu-fallback, epic-gpu]
issue: null
epic: gpu
supersedes: null
---

# ADR-001: CubeCL + native Rust-SIMD over raw wgpu for the GPU/CPU split

## Status

Accepted (2026-07-13). Establishes the GPU layer and the CPU fallback strategy for the port.

## Context

The port must run on any server — with or without a GPU — and still demonstrate a real performance
gain. Three forces shape the decision:

1. **Existing convention.** The author's prior `gpu-matmul-bench` project uses **CubeCL**
   (`cubecl = { version = "0.10.0", features = ["wgpu"] }`), with kernels written in pure Rust via
   `#[cube(launch_unchecked)]` and one-line device selection (`Device::default()`). Reusing this
   convention lowers friction.
2. **GPU-less portability.** Raw `wgpu` offers an automatic *software fallback* (llvmpipe), but it is
   slow and emulates a GPU on the CPU — it does not demonstrate real CPU performance.
3. **Benchmark honesty.** We want a CPU tier that shows the *best* the CPU can do (SIMD), not a GPU
   emulation.

## Decision

**GPU layer: CubeCL** (wgpu backend), mirroring the `gpu-matmul-bench` convention.
**CPU fallback: an explicit native Rust-SIMD path**, not wgpu's software renderer.
A **runtime capability probe** selects:

- Physical GPU present → CubeCL kernel.
- No GPU → native CPU-SIMD path.
- One binary, runs on any host, and **never panics for lack of a GPU** (AGENTS.md Rule 5).

The conversion math (e.g. RGB→XYZ→LAB) is written once in algorithmic terms and materialized in two
forms: a WGSL/CubeCL kernel and a Rust-SIMD version, sharing the same constants.

## Alternatives considered

### Alternative B — raw wgpu + WGSL with automatic software fallback (rejected)
The same shader runs on the GPU or falls back to llvmpipe on the CPU with no code change.
**Why rejected:** it diverges from the author's CubeCL convention, and the software fallback is slow
and does not represent real CPU performance — it weakens the benchmark's CPU tier.

### Alternative C — CubeCL only, CPU tier in plain scalar Rust (rejected)
Simpler to build.
**Why rejected:** the CPU tier would not show the CPU's peak (no SIMD), undervaluing one of the most
honest, always-present gains (GC-free + SIMD vs JS).

## Consequences

### Positive
- One binary runs on any server; degrades to CPU-SIMD without panicking.
- Honest 3-tier benchmark: JS baseline, Rust-CPU-SIMD, Rust-GPU.
- Reuses tooling and conventions already validated by the author.
- Swapping the backend (wgpu → CUDA) is essentially a one-line change in CubeCL, if ever needed.

### Negative / expected
- The conversion logic lives in two materializations (kernel + SIMD) — small duplication, mitigated
  by sharing constants and validating both against the same vectors.
- SIMD on stable Rust may require `wide`/`glam` (or `std::simd` on nightly) — decided in a later ADR.

### Neutral / expected
- The runtime probe introduces an adapter-detection dependency; isolated and tested.

## Review conditions

Reopen if:
1. CubeCL 0.x introduces breaking changes that make the kernel infeasible.
2. The CPU-SIMD tier fails to consistently beat the JS baseline (revisit the SIMD strategy).
3. A backend not covered by wgpu becomes necessary.

## Implementation

- Issue `feature: runtime GPU capability probe (select GPU or CPU-SIMD)`.
- Issue `feature: CubeCL GPU kernel for batch color conversion (rgb→lab→rgb)`.
- Issue `feature: CPU-SIMD path for hot routes (rgb↔xyz↔lab matrix math)`.

## References

### Internal
- [AGENTS.md](../../AGENTS.md) — Rule 5 (probe never panics), Measurement discipline.
- [ADR-002](002-behavior-faithful-validation-and-benchmark-honesty.md) — benchmark honesty.

### External
- [CubeCL](https://github.com/tracel-ai/cubecl)
- [wgpu](https://github.com/gfx-rs/wgpu)
