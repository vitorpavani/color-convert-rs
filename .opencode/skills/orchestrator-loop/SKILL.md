---
name: orchestrator-loop
description: The drain-the-queue protocol for color-convert-rs — pick the next GitHub issue by priority, run RED/GREEN/BLUE sub-agents with gate validation, measure, log, finish, and repeat until the queue is empty or blocked
license: MIT
compatibility: opencode
metadata:
  audience: orchestrator
  workflow: github
  priority: high
---

# orchestrator-loop — drain the issue queue

The autonomous loop the `orchestrator` runs to port `color-convert` issue-by-issue with minimal
human input. Read `AGENTS.md` first; this skill operationalizes Rule 14 across the whole backlog.

## Queue selection order

1. Exclude `agent:working` and `blocked`.
2. Prefer `phase:ready`.
3. Priority: `p1:high` before `p2:medium`.
4. Epic order: `epic:mvp-port` → `epic:gpu` → `epic:self-improvement`.
5. Within a tier, smallest `size` first (`size:XS` → `size:S` → `size:M`).

```bash
gh issue list --state open --label phase:ready \
  --json number,title,labels,createdAt \
  --jq 'sort_by(.number)'
```

## Per-issue cycle

For the selected issue:

1. **Claim + isolate** — skill `issue-workflow` step 1: label `agent:working` and create the
   issue's **git worktree** (skill `git-worktrees`) at `.worktrees/<type>-<slug>` off
   `origin/staging`. All work for this issue lives there; the main checkout stays on `staging`.
2. **Decompose** — list the issue's behaviors simplest → hardest. One behavior per TDD cycle.
3. **For each behavior, run the gated cycle:**

   ```
   task(subagent_type="red-dev",   prompt=<WORKTREE PATH + behavior + vector source + issue context>)
     └─ GATE: cargo test shows the new test FAILING for the right reason. Else re-task red-dev.
   task(subagent_type="green-dev", prompt=<WORKTREE PATH + failing test + minimal-impl instruction>)
     └─ GATE: cargo test PASSES; cargo clippy -- -D warnings clean. Else re-task green-dev.
   task(subagent_type="blue-dev",  prompt=<WORKTREE PATH + green diff + refactor/review instruction>)
     └─ GATE: still green; fmt clean. If "BLUE → GREEN: <finding>", spawn a new RED for it.
   ```

   Always pass full context to each sub-agent (they do not share your memory OR your cwd): the
   **absolute worktree path** (they must `cd` in and verify with `git rev-parse --show-toplevel`
   before editing), the issue, the behavior, file paths, conventions, the exact failing/passing
   state. Sub-agents of the SAME issue share that one worktree.

**Parallelism:** to work multiple issues at once, give **each issue its own worktree** and run their
cycles concurrently — worktrees guarantee they never collide on the index or checked-out files.

4. **Measure** (if perf-relevant) — skill `benchmark-ledger`: append a 3-tier record. **Keep the
   change only if it beats JS baseline AND the previous Rust iteration.** Otherwise revert.
5. **Log** — comment each phase on the issue (see `issue-workflow` step 2); mirror deep rationale to
   the Obsidian dev journal.
6. **Finish** — skill `issue-workflow` steps 3–4 (draft PR → gates → squash-merge → close → cleanup),
   including **`git worktree remove`** for the issue's worktree (never `rm -rf`).
7. **Next** — return to queue selection. Continue until empty or a `blocked` wall.

## Gate discipline (do NOT skip)

- GREEN never starts until RED genuinely fails.
- Merge never happens until build + test + clippy + fmt all pass.
- No perf claim without a `results.jsonl` entry.
- 3 failed attempts on one issue → revert to last green, `blocked`, comment, move on (Rule 13).

## Minimal-human contract

Proceed autonomously through the queue. Only surface to the human for: a genuine scope change, a
destructive/irreversible action, or a design decision the issues do not resolve. Otherwise make the
small call, note it in the issue, and keep moving.

## Self-improvement phase (after MVP)

Once all `epic:mvp-port` routes are green, switch to `improvement-dev` over `epic:self-improvement`
issues: baseline → propose one hypothesis → RED/GREEN/BLUE → re-measure → keep-if-better/drop-if-not
→ repeat until hypotheses are exhausted.

## DO / DON'T

| DO | DON'T |
|----|-------|
| One behavior per TDD cycle | Batch behaviors into one cycle |
| Validate each gate before advancing | Trust a sub-agent's "done" without checking |
| Pass full context + worktree path to every sub-agent | Assume sub-agents remember prior state or share your cwd |
| One worktree per issue for parallelism | Run parallel issues in the main checkout |
| Keep-if-better, revert-if-not | Keep an unmeasured "improvement" |
