---
tags: [dashboard]
aliases:
  - Active Work
  - Open Issues
updated: 2026-07-20
---

# Active Work

Live view of the agentic issue queue. The orchestrator drains this queue one issue at a time via
RED → GREEN → BLUE (see [AGENTS.md](../AGENTS.md) Rule 14).

> Quick query: `gh issue list --state open --label phase:ready --json number,title,labels`

---

## ✅ Project Status: Production-Ready

All 17 color models ported. 16 SIMD batch routes (f32x8). Multi-core parallelism (rayon).
GPU kernels (CubeCL). sRGB LUT + fast cbrt. 10-wave optimization drive complete.

**Headline:** rgb→lab **111.3 MP/s** single-core (10.3× over scalar), **164.0 MP/s** multi-core
(15.2×). See [[01-optimization-journey]] for the full story.

## 🚨 Blockers

| Issue | Title | Waiting on |
|-------|-------|------------|
| _(none)_ | | |

## 🔄 In Progress

| Issue | Title | Phase | Worktree |
|-------|-------|-------|----------|
| _(none — optimization drive complete)_ | | | |

## 📋 Optimization Drive Summary (10 waves, 33 kept / 7 dropped)

| Wave | Scope | Kept | Dropped |
|------|-------|------|---------|
| 1–5 | Forward SIMD (rgb→X) | 10 | 2 (SoA #25, GPU sweep #24) |
| 6–8 | Inverse SIMD (X→rgb) | 5 | 0 |
| 9 | Multi-core (rayon) | 13 | 3 (cmyk/apple/lab→xyz) |
| T1–T3 | Algorithmic (LUT, cbrt, fused convert, GPU parity) | 5 | 2 (double-buffer #114, chunk tuning #122) |

See [[01-optimization-journey]] for the full per-wave breakdown.

## Remaining Opportunities

The CPU optimization surface is genuinely exhausted. The only remaining direction:

- **GPU memory staging** — pinned/zero-copy buffers to attack the PCIe bottleneck. But #114
  confirmed the GPU is transfer-bound with nothing to overlap. Low expected value.

---

See [[index]] to navigate the full vault.
