---
tags: [dashboard]
aliases:
  - Active Work
  - Open Issues
updated: 2026-07-13
---

# Active Work

Live view of the agentic issue queue. The orchestrator drains this queue one issue at a time via
RED → GREEN → BLUE (see [AGENTS.md](../AGENTS.md) Rule 14).

> Quick query: `gh issue list --state open --label phase:ready --json number,title,labels`

---

## 🚨 Blockers

| Issue | Title | Waiting on |
|-------|-------|------------|
| _(none yet)_ | | |

## 🔄 In Progress

| Issue | Title | Phase | Worktree |
|-------|-------|-------|----------|
| _(none yet — scaffolding session)_ | | | |

## 📋 Up Next (epic:mvp-port foundation)

Recommended pickup order — infra first (they unblock every route):

| # | Issue | Size |
|---|-------|------|
| 1 | infra: Cargo project scaffold | S |
| 2 | infra: JS reference-vector generator | M |
| 3 | infra: vector test harness (rstest) | M |
| 4 | infra: error type + module layout | S |
| 5 | route: hsl → {rgb, hsv, hcg} | S |
| … | (see `gh issue list`) | |

## Epics

| Epic | Meaning | Open |
|------|---------|------|
| `epic:mvp-port` | Minimal compatible port of all routes | 18 |
| `epic:gpu` | CubeCL GPU + CPU-SIMD + runtime probe | 4 |
| `epic:self-improvement` | Post-MVP measured improvements | 3 |

## How to Refresh

```bash
gh issue list --state open --limit 50 --json number,title,labels
gh issue list --state open --label 'epic:mvp-port' --json number,title
```

---

See [[index]] to navigate the full vault.
