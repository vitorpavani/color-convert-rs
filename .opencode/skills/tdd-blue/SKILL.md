---
name: tdd-blue
description: BLUE phase of the color-convert-rs TDD loop — refactor GREEN's code for clarity/consistency/perf without changing behavior, review against conventions, hand correctness gaps back to GREEN
license: MIT
compatibility: opencode
metadata:
  audience: all-agents
  workflow: tdd
  phase: blue
  priority: high
---

# tdd-blue — Refactor + review

The BLUE phase improves what GREEN wrote **without changing observable behavior**. All tests stay
green throughout. Any behavior change requires a new RED test first.

## When to use

Invoked by `blue-dev` (or the `orchestrator`) after GREEN passes, before the issue is finished.

## Procedure

1. **Review the GREEN diff** against `AGENTS.md` conventions and anti-patterns.
2. **Refactor safely:**
   - remove duplication introduced during GREEN,
   - add/verify the module `//!` doc explaining the route + reference behavior + tolerance (Rule 12),
   - ensure `thiserror` + `?` error handling; no `unwrap`/`expect` in library paths,
   - isolate any `unsafe` (SIMD intrinsics) into a small module with a `// SAFETY:` justification,
   - improve naming and structure for consistency with sibling routes.
3. **Keep it green:** run `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` after
   every meaningful step.
4. **If perf-relevant:** append a 3-tier record via skill `benchmark-ledger`; compare to JS baseline
   and previous iteration. Keep the refactor only if correctness holds (speed must never break
   correctness).
5. **Commit:** `refactor(<scope>): tidy <behavior> implementation 🔵`.

## Feedback loop (BLUE → GREEN)

If you discover a **correctness gap or missing behavior** (not a refactor), do NOT quietly fix it —
that would be untested behavior change. Report to the orchestrator as `BLUE → GREEN: <finding>` so
it is logged on the issue and handled as its own RED → GREEN step.

## Behavior-preservation guard

The test suite is your safety net, but tests only cover known vectors. Before merging a refactor,
re-run the full suite. If you changed a numeric path, add a note for the orchestrator to consider an
additional vector rather than trusting the refactor blindly.

## DO / DON'T

| DO | DON'T |
|----|-------|
| Refactor with tests green | Change observable behavior without a RED test |
| Report correctness gaps to GREEN | Silently expand scope |
| Back perf claims with `results.jsonl` | Claim "faster" without measuring |
| Document non-obvious math | Leave `unsafe` unjustified |
