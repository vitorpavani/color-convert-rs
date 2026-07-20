---
tags: [index]
aliases:
  - Home
  - color-convert-rs Docs
updated: 2026-07-20
---

# color-convert-rs

Fast Rust color-space conversion for **batch image and video processing** — f32x8 SIMD,
multi-core rayon, optional GPU (CubeCL), and a wasm/npm package. Converts full HD frames
in under 30ms. Built fully agentically via a Red/Green/Blue TDD loop.

> **Status:** v0.1.0 published. Rust crate on crates.io path ready, npm package live.
> 17 color models, stride-aware batch API (`src/batch.rs`), `image` crate integration,
> wasm batch exports (2–8× faster than JS loops), flake.nix dev shell.
> rgb→lab: **111.3 MP/s** single-core (10.3×), **164.0 MP/s** multi-core (15.2×).

## Vault Structure

```
ccrs-vault/
├── index.md                  ← You are here
├── active-work.md            — live state dashboard
├── adrs/                     — Architecture Decision Records (5 ADRs)
├── engineering/              — dev journal, benchmark methodology, findings (4 docs)
└── articles/                 — running drafts for the two article threads
```

## I want to…

### 🚀 Understand the project

| Goal | Start here |
| ---- | ---------- |
| Read the code | [README](../README.md) |
| See the development contract | [AGENTS.md](../AGENTS.md) |
| What's the current state | [active-work](active-work.md) |

### 🏗️ Understand the decisions

| Goal | Start here |
| ---- | ---------- |
| Why CubeCL + SIMD, not raw wgpu | [ADR-001](adrs/001-cubecl-plus-simd-over-raw-wgpu.md) |
| How correctness is validated | [ADR-002](adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) |
| Why feature flags (gpu/wasm/image) | [ADR-003](adrs/003-feature-gating.md) |
| npm/wasm strategy + perf findings | [ADR-004](adrs/004-npm-wasm-performance.md) |
| Why we pivoted to image processing | [ADR-005](adrs/005-image-processing-pivot.md) |

### 📊 Understand the performance story

| Goal | Start here |
| ---- | ---------- |
| How the 3 tiers are measured | [benchmarks/README](../benchmarks/README.md) |
| The 10-wave optimization journey (10.8 → 111.3 MP/s) | [[01-optimization-journey]] |
| Publish readiness + npm package | [[02-publish-readiness]] |
| Batch API + image crate integration | [[03-image-batch-api]] |

### ✍️ Follow the articles

| Goal | Start here |
| ---- | ---------- |
| The TS→Rust+GPU port thread | [article: the port](articles/thread-01-the-port.md) |
| The agentic-process thread | [article: the process](articles/thread-02-the-agentic-process.md) |

## All Documents

### adrs/

- [ADR-001](adrs/001-cubecl-plus-simd-over-raw-wgpu.md) — GPU layer + CPU fallback choice
- [ADR-002](adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) — correctness + honest measurement
- [ADR-003](adrs/003-feature-gating.md) — feature flags: gpu, wasm, image behind opt-in
- [ADR-004](adrs/004-npm-wasm-performance.md) — npm package: single-color slower, batch faster
- [ADR-005](adrs/005-image-processing-pivot.md) — strategic pivot from JS port to image processing

### engineering/

- [00-benchmark-methodology](engineering/00-benchmark-methodology.md) — how we measure
- [01-optimization-journey](engineering/01-optimization-journey.md) — 10-wave optimization drive (33 kept / 7 dropped)
- [02-publish-readiness](engineering/02-publish-readiness.md) — #131–135, wasm-pack, npm publish, CI
- [03-image-batch-api](engineering/03-image-batch-api.md) — stride API, image crate, 72–224 M px/s

### articles/

- [thread-01-the-port](articles/thread-01-the-port.md) — porting color-convert to Rust + GPU
- [thread-02-the-agentic-process](articles/thread-02-the-agentic-process.md) — building with autonomous agents

## Project References

- Repo: [github.com/vitorpavani/color-convert-rs](https://github.com/vitorpavani/color-convert-rs)
- Upstream: [Qix-/color-convert](https://github.com/Qix-/color-convert)
- npm: [color-convert-rs](https://www.npmjs.com/package/color-convert-rs)
