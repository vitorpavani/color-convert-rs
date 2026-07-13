---
name: benchmark-ledger
description: How to run the 3-tier color-convert-rs benchmark (JS baseline vs Rust-CPU-SIMD vs Rust-GPU) and append honest, reproducible records to the committed results ledger, then decide keep-or-revert
license: MIT
compatibility: opencode
metadata:
  audience: all-agents
  workflow: benchmark
  priority: high
---

# benchmark-ledger — measure, record, decide

Measurement is the project's truth source for "are we improving?". Every perf-relevant change is
measured on all three tiers and appended to `benchmarks/results.jsonl`. No claim without a record.

## The three tiers

| Tier key | Implementation |
|----------|----------------|
| `js` | `color-convert` on Node.js (the baseline we are porting) |
| `cpu` | Native Rust with explicit SIMD |
| `gpu` | CubeCL compute kernel (wgpu backend) |

All three run on the **same input generator** and the **same host** within a single comparison, so
deltas are attributable.

## Record schema (one JSON object per line)

See `benchmarks/SCHEMA.md` for the authoritative field list. Every record MUST include at least:

```json
{
  "ts": "2026-07-13T20:30:00Z",
  "commit": "<git short sha>",
  "issue": 12,
  "route": "rgb->hsl",
  "tier": "cpu",
  "input_size": 1048576,
  "metric": "throughput_mpx_s",
  "value": 812.4,
  "ms": 1.29,
  "host": "<hostname>",
  "gpu_present": false,
  "notes": "scalar baseline"
}
```

Append (never rewrite history):

```bash
echo '<json-object>' >> benchmarks/results.jsonl
```

## Procedure

1. **Baseline first.** Before changing anything perf-relevant, record the current numbers for the
   target route on all applicable tiers.
2. **Make the change** through the normal RED/GREEN/BLUE cycle (correctness first).
3. **Re-measure** on identical inputs. Record a new line per tier.
4. **Compare and decide:**
   - **Keep** only if the change beats **BOTH** the JS baseline **AND** the previous Rust iteration
     on the target metric, with all correctness tests still green.
   - **Revert** otherwise. Record the negative result anyway (negative results are article gold) with
     a `notes` explaining what was tried and why it lost.
5. **Summarize.** Regenerate the human-readable rollup (`benchmarks/README.md` table) and comment the
   delta on the issue.

## Honesty rules

- Warm up before timing (GPU JIT + device init; CPU cache). Report best-of-N wall time.
- Never compare across different hosts or input sizes within one decision.
- Never hand-edit `results.jsonl` to make a change look good. It is append-only history.
- A "faster but wrong" result is a FAILURE. Correctness gates before speed, always.

## GPU-absent hosts

If `gpu_present` is false, the `gpu` tier is skipped for that run (record only `js` and `cpu`), and
the runtime probe must have selected the CPU-SIMD path. Do not fabricate GPU numbers.

## DO / DON'T

| DO | DON'T |
|----|-------|
| Record baseline before + after | Claim a win from memory |
| Same input + host per comparison | Mix hosts/sizes in one decision |
| Append-only ledger | Rewrite past records |
| Record negative results too | Hide failed experiments |
