---
tags: [index]
aliases:
  - Home
  - color-convert-rs Docs
updated: 2026-07-13
---

# color-convert-rs

A behavior-faithful Rust port of the npm [`color-convert`](https://github.com/Qix-/color-convert)
library, GPU-accelerated with CubeCL and a native Rust-SIMD CPU path — built **fully agentically**
via a Red/Green/Blue TDD loop. This vault holds the deep documentation: decisions, engineering
journal, and the two article threads.

## Vault Structure

```
ccrs-vault/
├── index.md                  ← You are here
├── active-work.md            — live issue/queue dashboard
├── adrs/                     — Architecture Decision Records
├── engineering/              — dev journal, benchmark methodology, findings
└── articles/                 — running drafts for the two article threads
```

## I want to…

### 🚀 Understand the project

| Goal | Start here |
| ---- | ---------- |
| See the development contract | [AGENTS.md](../AGENTS.md) |
| Understand the agentic model | [README](../README.md) |
| See what's being worked on | [active-work](active-work.md) |

### 🏗️ Understand the decisions

| Goal | Start here |
| ---- | ---------- |
| Why CubeCL + SIMD, not raw wgpu | [ADR-001](adrs/001-cubecl-plus-simd-over-raw-wgpu.md) |
| How correctness is validated | [ADR-002](adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) |

### 📊 Understand the performance story

| Goal | Start here |
| ---- | ---------- |
| How the 3 tiers are measured | [benchmarks/README](../benchmarks/README.md) |
| The results-ledger schema | [benchmarks/SCHEMA](../benchmarks/SCHEMA.md) |
| Why a "trivial" library still shows a big win | [ADR-002](adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) |

### ✍️ Follow the articles

| Goal | Start here |
| ---- | ---------- |
| The TS→Rust+GPU port thread | [article: the port](articles/thread-01-the-port.md) |
| The agentic-process thread | [article: the process](articles/thread-02-the-agentic-process.md) |

## All Documents

### adrs/

- [ADR-001 cubecl-plus-simd-over-raw-wgpu](adrs/001-cubecl-plus-simd-over-raw-wgpu.md) — GPU layer + CPU fallback choice
- [ADR-002 behavior-faithful-validation-and-benchmark-honesty](adrs/002-behavior-faithful-validation-and-benchmark-honesty.md) — correctness + honest measurement

### engineering/

- [00-benchmark-methodology](engineering/00-benchmark-methodology.md) — how we measure, and why a simple library still demonstrates real gains

### articles/

- [thread-01-the-port](articles/thread-01-the-port.md) — draft: porting color-convert to Rust + GPU
- [thread-02-the-agentic-process](articles/thread-02-the-agentic-process.md) — draft: building it with autonomous agents

## Project References

- Repo: [github.com/vitorpavani/color-convert-rs](https://github.com/vitorpavani/color-convert-rs)
- Upstream: [Qix-/color-convert](https://github.com/Qix-/color-convert)
