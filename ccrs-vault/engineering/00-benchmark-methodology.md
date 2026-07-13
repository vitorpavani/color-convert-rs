---
title: "Benchmark Methodology"
date: 2026-07
tags:
  - benchmark
  - methodology
  - performance
  - honesty
status: draft
prerequisites: "none"
---

# 📊 Benchmark Methodology — how a "trivial" library still shows a real gain

**Date:** July 2026
**Status:** 🟡 Draft — populated as measurements begin
**Prerequisite:** None

---

## 1. The problem

`color-convert` is trivial per call — one `rgb→lab` is ~20 float ops. On a single conversion, Rust
and JS both finish in nanoseconds; the difference is noise. So a naive "convert one color" benchmark
would show **no story** and would even make JS look fine.

The gain is a **scale phenomenon**. Real workloads convert millions of colors (a 4000×3000 image =
12M pixels, video frames, data pipelines, palette generators). We therefore benchmark
**"convert an N-pixel buffer"** and scale N — turning a trivial function into a throughput problem.

## 2. What we measure

| Tier | Implementation | Purpose |
|------|----------------|---------|
| `js` | `color-convert` on Node.js | The baseline to beat |
| `cpu` | Native Rust + explicit SIMD | Runs everywhere, incl. GPU-less servers |
| `gpu` | CubeCL kernel (wgpu) | Peak throughput when a GPU is present |

Primary metric: **throughput** (`throughput_mpx_s`, higher-is-better). Secondary: best-of-N wall
time (`ms`). All three tiers run on the **same input generator** and the **same host** per comparison.

## 3. Where each gain comes from

| Comparison | Expected | Source of the gain |
|------------|----------|--------------------|
| Rust-CPU vs JS | ~3–15× (always) | No GC pauses (JS allocs a `[l,a,b]` per pixel → GC thrashing; Rust writes a pre-allocated buffer), no boxing/dynamic dispatch, **SIMD** on the matrix math |
| Rust-GPU vs all | ~50–500× (large N) | Embarrassingly parallel: each pixel independent → thousands of GPU threads; pure `f32`, no branches |

The **CPU-vs-JS gain is solid and always present**. The **GPU gain is spectacular but conditional** —
it needs large batches to beat the PCIe transfer cost.

## 4. Honesty guardrails (why the numbers survive scrutiny)

1. **Same input / host / metric** per comparison — enforced by `benchmarks/SCHEMA.md`.
2. **Warm-up before timing** — JS gets JIT warm-up; GPU excludes kernel-compile + device-init. We
   time steady state, not startup (the most common benchmark lie).
3. **Correctness gate first** — a tier's speed only counts if its output matches the JS reference
   vectors within tolerance. "Fast but wrong" = failure.
4. **Scaling curve, not one number** — the compelling artifact is the *throughput × N* chart: JS
   flattens/degrades under GC pressure while CPU-SIMD and GPU climb.
5. **Report the GPU crossover** — at small N the GPU *loses* (transfer dominates). We record that
   crossover point honestly; it is what makes the large-N win credible.
6. **Keep-if-better / revert-if-not** — a change is kept only if it beats the JS baseline **and** the
   previous Rust iteration, correctness intact. Negatives are recorded too.

## 5. The ledger

Every run appends one JSON object per tier to `benchmarks/results.jsonl` (schema in
`benchmarks/SCHEMA.md`). It is append-only history; the rollup table in `benchmarks/README.md` is
regenerated from it. This makes "are we improving?" a diffable, reproducible question.

## 6. Success criteria

- [ ] JS baseline recorded for every route at fixed input sizes
- [ ] CPU-SIMD tier consistently beats JS on throughput
- [ ] GPU tier beats CPU-SIMD beyond the measured crossover N
- [ ] The throughput × N chart is generated from the ledger
- [ ] The GPU crossover point is documented, not hidden

## References

- [ADR-002](../adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) — the decision this note operationalizes
- [benchmarks/README](../../benchmarks/README.md), [benchmarks/SCHEMA](../../benchmarks/SCHEMA.md)
- [AGENTS.md](../../AGENTS.md) — Measurement discipline
