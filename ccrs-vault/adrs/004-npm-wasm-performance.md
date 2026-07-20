---
adr: 004
title: npm/wasm strategy — single-color slower, batch faster
status: accepted
date: 2026-07-20
tags: [adr, npm, wasm, performance, strategy]
---

# ADR-004: npm/wasm Performance Strategy

## Context

The npm package (`color-convert-rs` on npmjs.com) wraps the Rust SIMD core in WebAssembly via
`wasm-pack`. The promise was a "drop-in replacement for color-convert that's faster."

**The reality was more nuanced.** Benchmarking 1M single-color conversions revealed:

| Route | color-convert (JS) | color-convert-rs (wasm) | Ratio |
|-------|-------------------:|------------------------:|------:|
| rgb→hsl | 33,911K ops/s | 1,844K ops/s | **0.05× (18× slower)** |
| rgb→lab | 8,594K ops/s | 1,143K ops/s | **0.13× (8× slower)** |

V8's JIT compiles `color-convert` to native code (~3ns per call). Every wasm call pays ~500ns
of boundary cost: string parsing (`"rgb"→Model::Rgb`), JsValue→Rust Color marshalling,
Rust Color→JsValue back. The actual math is negligible either way.

## Decision

**Don't compete on single-color speed. Compete on batch SIMD.**

Added 8 batch wasm exports that accept flat typed arrays (`Uint8Array`/`Float32Array`) and
return `Float32Array` — zero per-color boundary cost:

```js
convert.rgb.lab.batch(new Uint8Array([255,0,0, 0,255,0, ...])) // Float32Array
```

**Batch benchmark (100k colors):**

| Route | JS loop | wasm batch SIMD | Speedup |
|-------|--------:|----------------:|--------:|
| rgb→xyz | 11ms | 1ms | **7.6×** |
| rgb→lab | 16ms | 2ms | **6.8×** |
| rgb→cmyk | 6ms | 2ms | **3.8×** |
| rgb→hsv | 7ms | 2ms | **3.5×** |
| rgb→hsl | 7ms | 3ms | **2.3×** |

## Market research findings

Research into `color-convert`'s 333M weekly downloads and 3,790 dependents revealed:
- **95%+ of usage is single-color** (chalk, CSS, themes, color pickers)
- **Batch usage is essentially nonexistent** in the color-convert ecosystem
- Image processing uses `sharp`/`jimp`, data viz uses `d3-color` — nobody loops `color-convert`

This means the batch advantage targets a use case that doesn't exist in the npm color-convert
ecosystem. See [[ADR-005]] for the strategic pivot that followed.

## Consequences

**Positive:**
- Honest positioning: the npm README says "single-color is not faster, batch is 2–8× faster"
- The `.batch()` API is the differentiator for JS users who DO process arrays
- The wasm module is small (125KB) and parity-verified (272/272 routes)

**Negative:**
- Cannot market as "faster drop-in replacement" without qualification
- The dominant npm use case (single-color) is a loss
- The npm package's value prop is weaker than the Rust crate's

## Future: hybrid approach

The planned fix is a **hybrid npm package**: bundle the original color-convert JS for
single-color calls (no wasm boundary), and only cross to wasm for `.batch()`. This gives:
- Single-color: same speed as color-convert (zero wasm overhead)
- Batch: 2–8× faster (SIMD amortized)

Not yet implemented — tracked as a follow-up.

## References

- npm package: [color-convert-rs](https://www.npmjs.com/package/color-convert-rs)
- Issue [#132](https://github.com/vitorpavani/color-convert-rs/issues/132)
- [[02-publish-readiness]]
- [[ADR-005]] — the pivot to image processing that followed this finding
