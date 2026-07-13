---
title: "ADR-002: Behavior-faithful validation and benchmark honesty"
status: accepted
date: 2026-07-13
decision-makers: [Vitor Amorim Pavan]
tags: [adr, testing, benchmark, honesty, vectors, tdd, epic-mvp-port]
issue: null
epic: mvp-port
supersedes: null
---

# ADR-002: Behavior-faithful validation and benchmark honesty

## Status

Accepted (2026-07-13). Defines how we prove correctness and how we measure gains honestly — the core
of the project's value and of the articles.

## Context

`color-convert` is, per call, **trivial**: a single conversion (e.g. `rgb→lab`) is ~20 floating-point
operations. On one call, Rust and JS both finish in nanoseconds and the difference is noise. This
raises the project's central question: **how do we demonstrate a real GPU/Rust gain on something so
simple?**

Two truths resolve it:

1. **Performance only appears at scale.** Nobody converts one color — you convert millions (a
   4000×3000 image = 12M pixels, video, data pipelines, palette generators). The benchmark does not
   measure "the library"; it measures **"convert an N-pixel buffer"**, scaling N. That turns a trivial
   function into a *throughput* problem — and throughput is where the tiers truly diverge.
2. **Correctness is not optional.** A "fast but wrong" result is a failure, not a trade-off. So the
   source of truth is **vectors generated from the JS library** (AGENTS.md Rule 8), and every tier
   must match those vectors within tolerance before its speed counts.

## Decision

### Validation (behavior-faithful)
- Conversion expectations come from **vectors generated from `color-convert` JS**, committed under
  `tests/vectors/`. Never hand-edit a vector to pass a test — fix the implementation.
- Each route documents its **tolerance** and the reason (rounding, integer output, clamp).
- The idiomatic Rust API need not mirror the JS API 1:1, but the **observable output** (numbers,
  rounding, clamp, integer-vs-float shape) must.

### Benchmark honesty (guardrails, see `benchmarks/SCHEMA.md`)
1. **Same input, same host, same metric** per comparison. Never compare a large JS run to a small
   Rust run.
2. **Warm-up before timing.** JS gets JIT warm-up; GPU excludes kernel compile + device init. We time
   steady state, not startup — the most common benchmark lie, excluded here.
3. **Correctness first.** A tier's result only counts if it matches the reference vector within
   tolerance.
4. **Scaling curve, not a single number.** The convincing artifact is the *throughput × N* chart: JS
   flattens/degrades under GC pressure while CPU-SIMD and GPU climb. The *shape* tells the story.
5. **Measure the GPU↔CPU transfer cost.** At small N the GPU **loses** (PCIe upload/download
   dominates). Reporting that crossover point honestly is what gives the large-N win credibility.
6. **Keep-if-better, revert-if-not.** A change is kept only if it beats the JS baseline **and** the
   previous Rust iteration on the target metric, with correctness intact. Negative results are
   recorded (article material).

## Where each gain comes from (recorded for the articles)

- **Rust-CPU vs JS (~3–15×, always present):** no GC pauses (JS allocates a `[l,a,b]` array per pixel
  → 12M short-lived arrays → GC thrashing; Rust writes into a pre-allocated buffer, zero allocation in
  the hot loop), no boxing/dynamic dispatch, and **SIMD** (the conversions are matrix multiplies).
- **Rust-GPU vs everything (~50–500× at large N, conditional):** *embarrassingly parallel* work — each
  pixel is independent; the GPU runs thousands simultaneously. Pure `f32` kernel, no branches.

## Alternatives considered

### Alternative B — literal 1:1 API with JS (rejected)
**Why rejected:** it would freeze the design into JS idioms; we want idiomatic Rust with faithful
output, not the shape of the JS API.

### Alternative C — measure a single conversion (rejected)
**Why rejected:** at N=1 there is no story; the difference is noise and JS's JIT looks adequate. The
gain is a scale phenomenon.

## Consequences

### Positive
- Numbers that survive scrutiny (including showing where the GPU is **not** worth it).
- Correctness guaranteed against the reference library.
- The better article: honest methodology, not marketing.

### Negative / expected
- Generating and maintaining vectors adds an infra step (JS) before any RED.
- Reporting the GPU crossover (where it loses) is less "impressive" — but more honest.

### Neutral / expected
- The `results.jsonl` ledger grows append-only; the rollup is regenerated from it.

## Review conditions

Reopen if:
1. A route requires a tolerance that masks a real bug (revisit the vector strategy).
2. The target metric changes (e.g. p99 latency instead of throughput).

## Implementation

- Issue `infra: JS reference-vector generator`.
- Issue `infra: vector test harness (rstest parametric loader)`.
- Issue `infra: JS baseline benchmark runner` and `infra: Rust bench harness`.
- Skill `benchmark-ledger`, files `benchmarks/README.md` and `benchmarks/SCHEMA.md`.

## References

### Internal
- [AGENTS.md](../../AGENTS.md) — Rule 8 (vectors are truth), Measurement discipline.
- [benchmarks/README](../../benchmarks/README.md), [benchmarks/SCHEMA](../../benchmarks/SCHEMA.md).
- [00-benchmark-methodology](../engineering/00-benchmark-methodology.md).

### External
- [color-convert](https://github.com/Qix-/color-convert)
