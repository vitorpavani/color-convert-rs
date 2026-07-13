---
title: "Article Thread 02 — Building it with autonomous agents"
date: 2026-07
tags:
  - article
  - draft
  - agentic
  - tdd
  - opencode
status: draft
prerequisites: "none"
---

# ✍️ Thread 02 — Building it with autonomous agents

**Status:** 🟡 Running draft — grows as the agentic loop runs.
**Audience:** developers interested in agentic development, TDD orchestration, and opencode.

This note collects material for the second article thread: the *process*. The port is the vehicle;
the story is how an orchestrator + Red/Green/Blue sub-agents drove the whole build from GitHub issues
with minimal human input.

---

## Hook / thesis

A human sets up the rails once, then answers as little as possible. Autonomous agents drain a GitHub
issue queue, each issue built via a gated RED → GREEN → BLUE cycle, every step logged and measured.
Does it actually work end-to-end? Where does it break?

## Outline (living)

1. The setup — AGENTS.md contract, 5 agents, 7 skills, worktree isolation, the issue backlog.
2. The loop — orchestrator picks an issue, runs R/G/B sub-agents, validates gates, logs, finishes.
3. Worktrees for parallelism — one worktree per issue so agents never collide.
4. Gate discipline — GREEN can't start until RED truly fails; no merge without build/test/clippy/fmt.
5. Measurement as a first-class gate — keep-if-better, revert-if-not.
6. The self-improvement phase — improvement-dev proposes, measures, keeps or drops.
7. Honest retro — where delegation helped, where it failed, how much human input was really needed.

## Evidence to capture as we go

- [ ] A full issue's R/G/B comment trail (screenshot of the GitHub thread)
- [ ] A case where BLUE handed a finding back to GREEN
- [ ] A `blocked` escalation (the 3-strike rule firing)
- [ ] A parallel run — two worktrees, two issues, no collision
- [ ] An improvement-dev experiment that was **reverted** (measured loss)
- [ ] A tally of human interventions vs autonomous steps

## Linked references

- [[index]] — vault home
- [[active-work]] — the live queue this loop drains
- AGENTS.md Rule 14 (RED/GREEN/BLUE), Rule 11 (worktrees), Rule 13 (escalation)

## Scratch / quotes / snippets

<!-- Drop raw material here: issue-comment trails, orchestrator transcripts, timing of the loop. -->
