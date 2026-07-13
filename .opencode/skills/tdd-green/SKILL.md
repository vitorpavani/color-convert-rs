---
name: tdd-green
description: GREEN phase of the color-convert-rs TDD loop — write the minimum Rust code to make the failing test pass, no speculative generality, clippy clean
license: MIT
compatibility: opencode
metadata:
  audience: all-agents
  workflow: tdd
  phase: green
  priority: high
---

# tdd-green — Make it pass, minimally

The GREEN phase writes the **smallest** implementation that turns the current RED test green.
Nothing more. Consolidation and polish belong to BLUE.

## When to use

Invoked by `green-dev` (or the `orchestrator`) immediately after a RED test is confirmed failing.

## Procedure

1. **Read the failing test.** Understand the exact behavior and tolerance it demands.
2. **Write the minimum.** Implement only what makes THIS test pass. No extra routes, no premature
   traits/generics, no "while I'm here" additions. Duplication is fine now — BLUE removes it.
3. **Match observable behavior.** Rounding, clamping, and integer-vs-float output shape must mirror
   `color-convert`. If your math disagrees with the vector, your math is wrong (Rule 8).
4. **Gate:** `cargo test` exits zero; `cargo clippy -- -D warnings` is clean; `cargo fmt`.
5. **Commit:** `feat(<scope>): implement <behavior> to pass tests 🟢`.
6. **Report:** what you added, why it is minimal, anything BLUE should consider.

## Minimalism examples

- Need `rgb->hsl` for one vector? Implement `rgb::to_hsl` for the general case the test exercises —
  not `hsl->rgb`, not `rgb->hsv`. Those are separate issues.
- Do NOT introduce a `ColorSpace` trait until a test/issue actually requires polymorphism.

## Dependency discipline

Do NOT add crates here. A new dependency is its own issue + PR with justification (Rule 9). If a
behavior seems to need one, report it to the orchestrator instead of adding it.

## DO / DON'T

| DO | DON'T |
|----|-------|
| Minimum to pass the current test | Implement untested behaviors |
| Mirror JS rounding/clamping | Bend the vector to your math |
| Keep clippy clean by fixing causes | Silence clippy with `#[allow]` |
| Report follow-ups to orchestrator | Add dependencies inline |
