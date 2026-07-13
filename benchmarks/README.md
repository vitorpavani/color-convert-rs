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

## How to read the ledger

`results.jsonl` is append-only history. Never rewrite it. To see progress for a route:

```bash
# every cpu-tier record for rgb->hsl, newest last
rg '"route":"rgb->hsl"' benchmarks/results.jsonl | rg '"tier":"cpu"'
```

## Rollup table

<!-- Regenerated from results.jsonl by the summary step. Populated during the coding session. -->

| route | js (baseline) | cpu (SIMD) | gpu (CubeCL) | cpu speedup | gpu speedup | commit |
|-------|---------------|------------|--------------|-------------|-------------|--------|
| _(populated once measurements begin)_ | | | | | | |
