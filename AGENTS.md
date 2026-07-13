# color-convert-rs — Agent Knowledge Base

**Updated:** 2026-07-13 | **Branch model:** `main` (protected) ← `staging` ← `<type>/<slug>` | **Status:** Scaffolding

---

## OVERVIEW

A behavior-faithful Rust port of the npm [`color-convert`](https://github.com/Qix-/color-convert)
library, GPU-accelerated with **CubeCL** (wgpu backend) and a native **Rust-SIMD** CPU path.
Built fully agentically via a Red/Green/Blue TDD loop driven from GitHub issues. Every color
conversion route is validated against test vectors generated from the reference JS library, and
every change is measured against both the JS baseline and the previous Rust iteration.

**Stack:** Rust (edition 2024), CubeCL for GPU compute, `std::simd`/`wide` for CPU SIMD,
`thiserror` for errors, `rstest` for parametric tests, Node.js only for generating reference
vectors and the JS benchmark baseline.

## STRUCTURE

```
color-convert-rs/
├── AGENTS.md                 ← You are here — the development contract
├── README.md                 — Project intent + agentic model
├── .opencode/
│   ├── agent/                — orchestrator, red-dev, green-dev, blue-dev, improvement-dev
│   └── skills/               — tdd-red/green/blue, issue-workflow, orchestrator-loop, benchmark-ledger, git-worktrees
├── .worktrees/               — per-branch git worktrees for parallel agents (gitignored, never committed)
├── src/                      — (born from the first failing test; empty until then)
├── tests/                    — integration tests + JS-generated vectors
├── benchmarks/               — 3-tier benchmark harness + committed results ledger
│   ├── README.md             — how tiers are measured
│   ├── results.jsonl         — append-only measurement ledger (one JSON object per run)
│   └── SCHEMA.md             — the results.jsonl record schema
└── Cargo.toml                — (born in the coding session, not before)
```

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Development rules / contract | `AGENTS.md` (this file) |
| Write a failing test | agent `red-dev` + skill `tdd-red` |
| Make a test pass | agent `green-dev` + skill `tdd-green` |
| Refactor / review | agent `blue-dev` + skill `tdd-blue` |
| Drive the issue queue | agent `orchestrator` + skill `orchestrator-loop` |
| Propose improvements (post-MVP) | agent `improvement-dev` |
| Claim / review / finish an issue | skill `issue-workflow` |
| Isolate parallel work (worktrees) | skill `git-worktrees` → `.worktrees/<branch>` |
| Record a benchmark | skill `benchmark-ledger` → `benchmarks/results.jsonl` |
| Reference conversion behavior | upstream `color-convert` (JS) |
| Deep docs (ADRs, journal, articles) | Obsidian vault, `color-convert-rs/` space |

## COMMANDS

```bash
# (available once Cargo.toml is born in the coding session)
cargo build
cargo test                       # runs unit + integration + vector tests
cargo clippy -- -D warnings      # zero-warning gate
cargo fmt --check                # formatting gate
cargo bench                      # or the benchmarks/ harness for 3-tier runs

# GitHub queue
gh issue list --state open --label 'phase:ready' --json number,title,labels

# Reference vectors (JS)
node benchmarks/js/gen-vectors.mjs   # regenerates test vectors from color-convert
```

## CONVENTIONS

### Testing — TDD Mandatory (see Rule 14)
- **RED** writes a failing test first; `cargo test` MUST exit non-zero for the new test.
- **GREEN** writes the minimum code to pass; `cargo test` MUST exit zero.
- **BLUE** refactors/reviews; all tests stay green, `clippy` clean.
- Test data comes from **factory helpers** and **JS-generated vectors**, never hand-fudged numbers.
- Conversion correctness asserts against reference vectors within a documented tolerance.

### Error handling
- Library code returns `Result<_, E>` with a `thiserror` error enum. **No `unwrap()`/`expect()`
  in library code paths.** `expect()` is allowed in tests and benches with a message.

### Safety
- **No `unsafe`** without a `// SAFETY:` comment justifying every invariant. SIMD intrinsics that
  require `unsafe` must be isolated in a small, well-documented module and covered by tests.

### Numeric fidelity
- Match `color-convert` behavior within tolerance; document the tolerance per route in the test.
- Rounding, clamping, and integer-vs-float output shapes must mirror the JS library's observable
  behavior (this is the whole point of "behavior-faithful").

### Measurement discipline (NON-NEGOTIABLE)
- Every functional change that could affect performance is measured on all 3 tiers
  (JS baseline, Rust-CPU-SIMD, Rust-GPU) and appended to `benchmarks/results.jsonl`.
- An improvement is **kept only if it beats BOTH** the JS baseline AND the previous Rust
  iteration on the target metric. Otherwise it is reverted. No "looks faster" — measure it.

## ANTI-PATTERNS

- ❌ `unwrap()` / `expect()` in library code — use `thiserror` + `?`.
- ❌ `unsafe` without a `// SAFETY:` justification.
- ❌ `panic!` as control flow.
- ❌ `println!`/`eprintln!` for diagnostics — use `tracing`/`log` if logging is needed.
- ❌ Writing implementation code before a failing test demands it.
- ❌ Adding a dependency without a dedicated issue/PR (Rule 9).
- ❌ Hand-tuned "magic" test expectations — expectations come from JS vectors.
- ❌ Committing benchmark claims without a `results.jsonl` entry backing them.
- ❌ Committing directly to `main` or `staging`.
- ❌ `as any`-style type escapes — Rust equivalent: casting away correctness with `as` to silence
  a real problem, or `#[allow(...)]` to mute clippy instead of fixing it.

## KNOWN CONSTRAINTS (environment)

- Destructive shell commands are denied (`rm`, `rmdir`, `git clean`, `git reset --hard`,
  `git push --force`). Remove tracked files with `git rm`; untracked with `find … -delete`.
- The dev/CI server may or may not have a GPU. The runtime probe MUST degrade to the CPU-SIMD
  path cleanly — never assume a GPU is present.

---

## Rule 0: Single-Task Per Session
One issue per working session. Finish (or explicitly block) it before starting another.

## Rule 1: Never Commit to `main` or `staging` Directly
Branch from `staging`: `<type>/<short-slug>`. Types: `feat`, `fix`, `chore`, `docs`, `test`,
`refactor`, `perf`. No issue number in the branch name; lowercase, hyphen-separated, < 50 chars.
Examples: `feat/rgb-hsl-route`, `perf/lab-simd-lut`, `test/xyz-vectors`.

## Rule 2: Commit Messages — Conventional Commits
```
<type>(<scope>): <short description>

[optional body: why, not what]

[optional footer: Refs #<issue>]
```
Scopes: `rgb`, `hsl`, `hsv`, `cmyk`, `xyz`, `lab`, `lch`, `named`, `gpu`, `cpu`, `probe`,
`bench`, `vectors`, `agents`, `docs`.
TDD phase commits:
- RED: `test(<scope>): add failing test for <behavior> 🔴`
- GREEN: `feat(<scope>): implement <behavior> to pass tests 🟢`
- BLUE: `refactor(<scope>): tidy <behavior> implementation 🔵`

## Rule 3: No Merge Without a Passing Gate
Before a PR is marked ready: `cargo build` ✅, `cargo test` ✅, `cargo clippy -- -D warnings` ✅,
`cargo fmt --check` ✅. PRs open as **draft** and squash-merge into `staging`.

## Rule 4: Clippy & Format — Zero Tolerance
No warnings. Do not silence with `#[allow]` unless justified in a code comment referencing why.

## Rule 5: Safety — Hard Constraints
1. No `unsafe` without `// SAFETY:`.  2. No secrets in the repo.  3. No `git push --force`.
4. No unbounded allocation from untrusted sizes.  5. Runtime GPU probe must never panic on a
GPU-less host.

## Rule 6: TODO/FIXME Policy
Every TODO references an issue: `// TODO(#123): description`. No bare TODOs.

## Rule 7: Deprecated Code — Remove, Don't Accumulate
When a route is superseded (e.g. a faster SIMD impl replaces a scalar one), delete the old path
in the same PR unless it is intentionally retained as a benchmarked baseline (then document why).

## Rule 8: Reference Vectors Are the Source of Truth
Conversion expectations are generated from the JS `color-convert` library into committed vector
files. Do not edit vectors by hand to make a test pass — fix the implementation.

## Rule 9: Dependency Management
Each new crate gets its own issue + PR with a one-paragraph justification. No incidental deps.

## Rule 10: Agent Wiring
Agents live in `.opencode/agent/*.md`; their detailed procedures live in `.opencode/skills/*/SKILL.md`.
`red-dev` is **read-only on `src/`** (writes only under `tests/`). `green-dev`/`blue-dev` may edit
`src/`. The `orchestrator` spawns them via `task()` and validates the gate between phases.

## Rule 11: Worktree Isolation — Mandatory (parallel-safe)
Every issue is worked in **its own git worktree** under `.worktrees/<branch>` (see skill
`git-worktrees`). **Never `git checkout -b` in the main checkout** — that serializes work and causes
cross-agent collisions. The main checkout stays on `staging`, read-mostly.
- Create: `git worktree add -b "<type>/<slug>" ".worktrees/<type>-<slug>" origin/staging`.
- **Sub-agents of the SAME issue share that issue's worktree** (they pass the same branch through
  RED → GREEN → BLUE). **Different issues get different worktrees and run in parallel.**
- The orchestrator MUST pass the absolute worktree path to every sub-agent it spawns; each sub-agent
  `cd`s in and verifies with `git rev-parse --show-toplevel` before touching files.
- Tear down after merge with `git worktree remove` (never `rm -rf` — denied). `.worktrees/` is
  `.gitignore`d and never committed.

## Rule 12: Every Module Gets a Doc Comment
Each `src/` module starts with a `//!` doc explaining its conversion route, the reference behavior
it mirrors, and its tolerance. Non-obvious math cites a source.

## Rule 13: Error Escalation
After **3 failed fix attempts** on the same issue: stop, revert to the last green state, label the
issue `blocked`, comment the blocker and what was tried, and move to the next issue.

## Rule 14: Test-Driven Development — Mandatory

Every behavior lands via a phase-gated RED → GREEN → BLUE cycle. The `orchestrator` runs each phase
as a sub-agent and **validates the gate before proceeding**.

**TDD plan:** decompose the issue into behaviors ordered simplest → hardest. One behavior per cycle.

**RED (agent `red-dev`, skill `tdd-red`)**
- Writes exactly one failing test for the next behavior (under `tests/` or `#[cfg(test)]`).
- Gate: `cargo test <new test>` exits **non-zero** for the right reason (assertion/҂missing fn),
  not a compile error unrelated to the behavior.
- Commit: `test(<scope>): add failing test for <behavior> 🔴`.
- Logs to the issue: what behavior, what vector, expected value.

**GREEN (agent `green-dev`, skill `tdd-green`)**
- Writes the **minimum** code to pass. No extra features, no speculative generality.
- Gate: `cargo test` exits **zero**; `cargo clippy -- -D warnings` clean.
- Commit: `feat(<scope>): implement <behavior> to pass tests 🟢`.
- Logs to the issue: what was added, why minimal.

**BLUE (agent `blue-dev`, skill `tdd-blue`)**
- Refactors for clarity/consistency/perf; reviews against conventions; may hand findings back to
  GREEN (logged on the issue as "BLUE → GREEN: <finding>").
- Gate: all tests stay green; `clippy`/`fmt` clean; if perf-relevant, a `benchmarks/results.jsonl`
  entry is appended and compared to baseline + previous.
- Commit: `refactor(<scope>): tidy <behavior> implementation 🔵`.

**Measurement gate (perf-relevant issues):** the orchestrator records a 3-tier benchmark and keeps
the change only if it beats JS baseline AND the previous Rust iteration (see Measurement discipline).

**When TDD does not apply:** pure docs, ADRs, label/issue admin, benchmark-ledger structure,
dependency-only PRs (still gated by build/clippy).

## Quick Reference

| Topic | Rule |
|-------|------|
| Branch from staging | 1 |
| Conventional commits | 2 |
| Worktree isolation (parallel) | 11 |
| Passing gate before PR | 3 |
| Zero clippy warnings | 4 |
| Safety constraints | 5 |
| TODO references issue | 6 |
| Vectors are truth | 8 |
| Deps get own PR | 9 |
| 3-strike escalation | 13 |
| RED/GREEN/BLUE mandatory | 14 |

## Agent Routing

| Agent | mode | edits `src/` | Role |
|-------|------|--------------|------|
| `orchestrator` | primary | via sub-agents | Drains the issue queue; runs R/G/B; validates gates; logs to GitHub + Obsidian; records benchmarks |
| `red-dev` | subagent | no (tests only) | Writes one failing test per behavior |
| `green-dev` | subagent | yes | Minimal code to pass |
| `blue-dev` | subagent | yes | Refactor / review / feedback to GREEN |
| `improvement-dev` | subagent | yes | Post-MVP: proposes + measures architectural/algorithmic improvements, keep-if-better |
