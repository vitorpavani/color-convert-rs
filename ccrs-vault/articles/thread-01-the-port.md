---
title: "Article Thread 01 — Porting color-convert to Rust + GPU"
date: 2026-07
tags:
  - article
  - draft
  - rust
  - gpu
  - port
status: draft
prerequisites: "none"
---

# ✍️ Thread 01 — Porting color-convert to Rust + GPU

**Status:** 🟡 Running draft — grows as the port progresses.
**Audience:** developers interested in TS→Rust ports, SIMD, and GPU compute.

This note collects material for the first article thread: the technical port. Capture decisions,
dead-ends, numbers, and code snippets *as they happen* — reconstruction later loses the texture.

---

## Hook / thesis

Take a "trivial" npm library (`color-convert`) and show what Rust's GC-free memory model, SIMD, and
GPU parallelism buy you — measured honestly, including where the GPU is *not* worth it.

## Outline (living)

1. Why color-convert — the deceptively simple candidate; why per-call is trivial and scale is the story.
2. Behavior-faithful porting — vectors as the source of truth; tolerance per route.
3. The CPU-SIMD tier — GC-free hot loop, matrix math, lane width; the always-present win.
4. The GPU tier — CubeCL kernel, one thread per pixel; the conditional 50–500×.
5. The runtime probe — one binary, GPU-or-CPU, never panics on a GPU-less host.
6. The honest benchmark — warm-up, same-input, the throughput × N curve, the GPU crossover.
7. Results — the numbers, the chart, what surprised us.

## Evidence to capture as we go

- [ ] The first RED test and the vector that drove it
- [ ] The scalar → SIMD diff and its measured delta
- [ ] The CubeCL kernel and its correctness check against CPU
- [ ] The throughput × N chart (from `benchmarks/results.jsonl`)
- [ ] The GPU crossover point (small-N GPU loss)
- [ ] Any dead-end (e.g. a SIMD layout that lost) — negatives are gold

## Linked decisions

- [[001-cubecl-plus-simd-over-raw-wgpu]] — GPU layer + CPU fallback
- [[002-behavior-faithful-validation-and-benchmark-honesty]] — correctness + measurement
- [[00-benchmark-methodology]] — how we measure

## Scratch / quotes / snippets

<!-- Drop raw material here as the work happens: commit SHAs, numbers, code blocks, screenshots. -->
