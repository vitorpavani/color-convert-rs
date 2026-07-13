---
name: issue-workflow
description: GitHub issue lifecycle for color-convert-rs — claim an issue, branch from staging, log TDD phases as comments, open a draft PR, squash-merge to staging, close and clean up
license: MIT
compatibility: opencode
metadata:
  audience: all-agents
  workflow: github
  priority: high
---

# issue-workflow — claim → work → review → finish

Drives a single GitHub issue through its lifecycle. Adapted for a Rust/cargo project (see
`AGENTS.md`). All work branches from `staging`; nothing lands on `main`/`staging` directly.

## Labels

| Label | Meaning |
|-------|---------|
| `agent:working` | Claimed, work in progress |
| `blocked` | Stuck after 3 attempts (Rule 13) |
| `phase:ready` | Groomed and ready to pick up |
| `phase:red` / `phase:green` / `phase:blue` | Current TDD phase (optional live status) |
| `p1:high` / `p2:medium` | Priority |
| `size:XS` / `size:S` / `size:M` | Size |
| `type:feature` / `type:bug` / `type:refactor` / `type:perf` | Kind |
| `epic:mvp-port` / `epic:gpu` / `epic:self-improvement` | Epic |

## 1. Claim (issue-start)

```bash
gh issue view <N> --json labels,assignees,title
gh label create agent:working --description "WIP by an agent" --color FBCA04 2>/dev/null || true
gh issue edit <N> --add-label agent:working
gh issue comment <N> --body "🤖 Starting work. Branch: \`<type>/<slug>\` (worktree \`.worktrees/<type>-<slug>\`)."

# Create the ISOLATED worktree (see skill `git-worktrees`) — NEVER `git checkout -b` in the main checkout
MAIN_ROOT="$(dirname "$(git rev-parse --git-common-dir)")"
BRANCH="<type>/<slug>"
WT_DIR="$MAIN_ROOT/.worktrees/${BRANCH//\//-}"
git fetch origin staging
git worktree add -b "$BRANCH" "$WT_DIR" origin/staging
cd "$WT_DIR" && git rev-parse --show-toplevel   # sanity: prints $WT_DIR
```

Branch naming (Rule 1): `<type>/<short-slug>`, no issue number, lowercase, < 50 chars.
All subsequent steps run **inside `$WT_DIR`**. See skill `git-worktrees` for the full protocol.

## 2. Work + log every TDD phase

Comment each phase transition on the issue so the whole R/G/B trail is auditable:

```bash
gh issue comment <N> --body "🔴 red-dev: failing test for <behavior> (vector <src>, expected <val>)."
gh issue comment <N> --body "🟢 green-dev: minimal impl of <behavior>; cargo test green, clippy clean."
gh issue comment <N> --body "🔵 blue-dev: refactor <behavior>; tests green. [BLUE → GREEN: <finding> if any]."
```

For perf-relevant issues, also comment the benchmark delta after recording it (see
`benchmark-ledger`).

## 3. Review (issue-review)

```bash
cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt --check
git diff origin/staging...HEAD --stat
git log origin/staging...HEAD --oneline
# anti-pattern scan (Rule: no unwrap/expect in lib paths, no unjustified unsafe)
git diff origin/staging...HEAD | rg -n 'unwrap\(\)|expect\(|unsafe' || true
gh pr create --base staging --draft --title "<type>(<scope>): <summary> (#<N>)" --body "<template>"
```

PR body template: **What / Why / Behaviors covered / Vectors used / Benchmark delta (if any) /
Checklist (build, test, clippy, fmt)**.

## 4. Finish (issue-finish) — only after gates pass

```bash
gh pr ready "<branch>"
gh pr merge "<branch>" --squash --delete-branch \
  --subject "$(gh pr view "<branch>" --json title --jq '.title')"
gh issue close <N> --comment "Completed by <PR_URL>."
gh issue edit <N> --remove-label agent:working

# Tear down the isolated worktree (see skill `git-worktrees`) — NEVER `rm -rf`
MAIN_ROOT="$(dirname "$(git rev-parse --git-common-dir)")"
WT_DIR="$MAIN_ROOT/.worktrees/${BRANCH//\//-}"
cd "$MAIN_ROOT"                       # move OUT of the worktree before removing it
git worktree remove "$WT_DIR"         # refuses if uncommitted changes remain
git worktree prune
git checkout staging && git pull origin staging
git branch -D "<branch>" 2>/dev/null || true   # --delete-branch usually already removed it
```

## Escalation (Rule 13)

After 3 failed attempts: revert to last green, then
```bash
gh issue edit <N> --add-label blocked --remove-label agent:working
gh issue comment <N> --body "🛑 Blocked after 3 attempts. Tried: <summary>. Blocker: <detail>."
```
and move to the next issue.

## DO / DON'T

| DO | DON'T |
|----|-------|
| Branch from `staging` | Commit to `main`/`staging` directly |
| Log every R/G/B phase on the issue | Merge non-draft without passing gates |
| Squash-merge + delete branch | `git push --force` |
| Remove `agent:working` on finish | Leave stale claim labels |
