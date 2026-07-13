---
name: git-worktrees
description: MANDATORY worktree isolation for color-convert-rs — every agent and sub-agent works in its own git worktree under .worktrees/ so parallel RED/GREEN/BLUE and multi-issue work never collide on a shared checkout
license: MIT
compatibility: opencode
metadata:
  audience: all-agents
  workflow: git
  priority: high
---

# git-worktrees — parallel-safe isolation

Multiple agents (parallel issues, and RED/GREEN/BLUE within an issue) share ONE repository. If they
all edit the single main checkout, they clobber each other's branches and files. **Every unit of
work MUST run in its own git worktree.** Never `git checkout -b` in the main checkout.

## Why (read once)

- Each worktree is a separate working directory bound to its own branch — parallel agents never
  fight over the index or the checked-out files.
- Worktrees tear down cleanly after merge — no leftover branches littering the main checkout.
- Build artifacts (`target/`) stay scoped per worktree, so a `cargo build` in one does not disturb
  another.
- `.worktrees/` is `.gitignore`d — it is scratch space, never committed.

## Layout convention

```
color-convert-rs/                 # main checkout (where .git lives) — stays on staging, read-mostly
└── .worktrees/
    ├── feat-rgb-hsl-route/       # one worktree per branch
    ├── perf-lab-simd-lut/
    └── test-xyz-vectors/
```

One worktree per branch. Never share a worktree across branches. Directory name mirrors the branch
with `/` → `-`.

## Create a worktree (issue-start)

```bash
# From anywhere inside the repo:
MAIN_ROOT="$(git rev-parse --show-toplevel)"
# If already inside a worktree, resolve the real main root:
COMMON_DIR="$(git rev-parse --git-common-dir)"
MAIN_ROOT="$(dirname "$COMMON_DIR")"

BRANCH="<type>/<slug>"                       # e.g. feat/rgb-hsl-route
WT_DIR="$MAIN_ROOT/.worktrees/${BRANCH//\//-}"

# Fail fast if it already exists
if [ -d "$WT_DIR" ]; then
  echo "ERROR: worktree already exists at $WT_DIR — resume there, or remove it first." >&2
  return 1 2>/dev/null || exit 1
fi

git fetch origin staging
git worktree add -b "$BRANCH" "$WT_DIR" origin/staging
cd "$WT_DIR"
git rev-parse --show-toplevel                # sanity: should print $WT_DIR, not the main checkout
```

All subsequent work for the issue — RED test writing, GREEN impl, BLUE refactor, `cargo test`,
commits — happens **inside `$WT_DIR`**.

## Sub-agent handoff (CRITICAL for RED/GREEN/BLUE)

When the orchestrator spawns `red-dev`/`green-dev`/`blue-dev`, it MUST pass the worktree path in the
prompt, and each sub-agent MUST `cd` into it before doing anything:

```
"Work inside the worktree at <ABS PATH to .worktrees/feat-rgb-hsl-route>.
 cd there first; confirm with `git rev-parse --show-toplevel`. Do NOT touch the main checkout."
```

Sub-agents in the same issue share ONE worktree (they hand the same branch RED → GREEN → BLUE).
Different issues get different worktrees and run fully in parallel.

## Verify where you are

```bash
git worktree list                 # all active worktrees + their branches
git rev-parse --show-toplevel     # confirm you are in the worktree, not the main checkout
```

## Tear down (issue-finish, after merge)

```bash
MAIN_ROOT="$(dirname "$(git rev-parse --git-common-dir)")"
cd "$MAIN_ROOT"                                   # move OUT of the worktree first
git worktree remove "$WT_DIR"                     # refuses if uncommitted changes exist
git worktree prune                                # clean stale metadata
```

If an agent crashed and abandoned a worktree with no valuable uncommitted work,
`git worktree remove --force "$WT_DIR"` is allowed only after confirming nothing matters.
(Note: `rm -rf` on a worktree is denied in this environment — always use `git worktree remove`.)

## Rules

- **Never** `git checkout -b` in the main checkout — always `git worktree add`.
- One worktree per branch; never reuse across branches.
- Sub-agents of the same issue share the issue's worktree; different issues → different worktrees.
- `cd` into the worktree and verify with `git rev-parse --show-toplevel` before editing.
- Always remove the worktree after the PR merges (`issue-workflow` finish step).
- `.worktrees/` is never committed.

## DO / DON'T

| DO | DON'T |
|----|-------|
| `git worktree add -b <branch> .worktrees/<name> origin/staging` | `git checkout -b` in the main checkout |
| Pass the worktree path to every sub-agent | Assume a sub-agent inherits your cwd |
| One worktree per branch | Share a worktree across branches |
| `git worktree remove` after merge | `rm -rf` a worktree (denied anyway) |
