# results.jsonl — record schema

`benchmarks/results.jsonl` is an **append-only** ledger: one JSON object per line, one line per
measured run (a single tier, route, and input size). Never rewrite existing lines. See skill
`benchmark-ledger` for the measurement procedure.

## Fields

| Field | Type | Required | Description |
|-------|------|:--------:|-------------|
| `ts` | string (ISO-8601 UTC) | ✅ | When the run was recorded, e.g. `2026-07-13T20:30:00Z` |
| `commit` | string | ✅ | Git short SHA the measurement was taken at |
| `issue` | integer | ✅ | GitHub issue number driving this change |
| `route` | string | ✅ | Conversion route, e.g. `rgb->hsl`, `lab->rgb` |
| `tier` | string enum | ✅ | One of `js` \| `cpu` \| `gpu` |
| `input_size` | integer | ✅ | Number of elements/pixels processed in the timed run |
| `metric` | string enum | ✅ | Metric name, e.g. `throughput_mpx_s` \| `ms_per_run` \| `ns_per_op` |
| `value` | number | ✅ | The metric value (higher-is-better for throughput; lower-is-better for latency) |
| `ms` | number | ✅ | Best-of-N wall time in milliseconds for the timed run |
| `iters` | integer | ✅ | N (timed iterations after warm-up) |
| `warmup` | integer | ✅ | Warm-up iterations before timing (GPU JIT / cache) |
| `host` | string | ✅ | Hostname / machine identifier (comparisons are host-scoped) |
| `gpu_present` | boolean | ✅ | Whether a physical GPU was detected by the runtime probe |
| `gpu_name` | string | ⬜ | GPU/adapter name when `gpu_present` is true |
| `baseline_ref` | string | ⬜ | For a `cpu`/`gpu` record: the commit of the previous Rust iteration it is compared against |
| `decision` | string enum | ⬜ | `kept` \| `reverted` \| `baseline` — the keep-or-revert outcome for this change |
| `notes` | string | ⬜ | Free text: what was tried, why kept/reverted, anomalies |

## Conventions

- **Higher-is-better** metrics (`throughput_mpx_s`) and **lower-is-better** metrics (`ms_per_run`,
  `ns_per_op`) must never be mixed within a single comparison — pick one target metric per decision.
- A comparison is only valid across records with the **same `route`, `input_size`, `metric`, and
  `host`**. Different hosts or sizes → different comparisons.
- If `gpu_present` is `false`, no `gpu`-tier record is written for that run (only `js` and `cpu`).
- `decision: "baseline"` marks the pre-change measurement; `kept`/`reverted` mark the post-change
  outcome. This lets the rollup reconstruct the full experiment trail.

## Example lines

```json
{"ts":"2026-07-13T20:30:00Z","commit":"a1b2c3d","issue":12,"route":"rgb->hsl","tier":"js","input_size":1048576,"metric":"throughput_mpx_s","value":94.2,"ms":11.13,"iters":20,"warmup":3,"host":"dev-box","gpu_present":false,"decision":"baseline","notes":"JS color-convert baseline"}
{"ts":"2026-07-13T20:31:00Z","commit":"a1b2c3d","issue":12,"route":"rgb->hsl","tier":"cpu","input_size":1048576,"metric":"throughput_mpx_s","value":812.4,"ms":1.29,"iters":20,"warmup":3,"host":"dev-box","gpu_present":false,"baseline_ref":"—","decision":"kept","notes":"scalar->SIMD, beats JS 8.6x"}
```
