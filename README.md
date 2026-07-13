# color-convert-rs

A **behavior-faithful Rust port** of the npm [`color-convert`](https://github.com/Qix-/color-convert)
library — GPU-accelerated with [CubeCL](https://github.com/tracel-ai/cubecl) and a native
Rust-SIMD CPU path — built **fully agentically** through a Red/Green/Blue TDD loop.

> **Status:** Scaffolding. No conversion code exists yet. Every line of Rust is born from a
> failing test inside the agentic loop described below — never hand-written ahead of a test.

## Why this exists

Two goals run in parallel:

1. **The port.** Reimplement `color-convert`'s conversion routes (RGB, HSL, HSV, CMYK, XYZ,
   LAB, LCH, named colors, …) in Rust. Correctness is validated against test vectors generated
   from the reference JS library, so outputs match within rounding tolerance.
2. **The process.** Drive the whole build with autonomous agents (an orchestrator that calls
   `red-dev` → `green-dev` → `blue-dev` per GitHub issue), measuring every step against both the
   JS baseline and the previous Rust iteration. The workflow itself is a first-class subject.

## Performance thesis

Color-space conversion is embarrassingly parallel numeric work (matrix multiplies, `pow`/`cbrt`
over independent pixels). Three tiers are benchmarked head-to-head:

| Tier | Implementation |
|------|----------------|
| Baseline | `color-convert` on Node.js |
| CPU | Native Rust with explicit SIMD |
| GPU | CubeCL compute kernel (wgpu backend) |

A **runtime capability probe** selects GPU when a physical device is present, else the CPU-SIMD
path — one binary that runs on any server and never crashes for lack of a GPU. Results are
appended to a committed, diffable ledger (`benchmarks/`) so we can prove improvement over time.

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

Once a minimal compatible library exists, an `improvement-dev` agent proposes architectural /
algorithmic changes, runs them through the same R/G/B cycle, re-measures, and **keeps the change
only if it beats both the JS baseline and the previous Rust solution** — otherwise it is dropped.
The loop continues until improvement possibilities are exhausted.

See [`AGENTS.md`](./AGENTS.md) for the full development contract, and the project's Obsidian vault
for deep documentation (ADRs, dev journal, article drafts).

## Reference

- Upstream library: [Qix-/color-convert](https://github.com/Qix-/color-convert) (MIT)
- GPU convention baseline: the author's `gpu-matmul-bench` (CubeCL)

## License

MIT — see [`LICENSE`](./LICENSE).
