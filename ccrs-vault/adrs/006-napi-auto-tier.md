---
adr: 006
title: napi-rs replacing wasm + hybrid JS fast-path + auto-tier
status: accepted
date: 2026-07-20
tags: [adr, napi, wasm, performance, architecture]
---

# ADR-006: napi-rs + JS Fast-Path + Auto-Tier

## Context

v0.1.0 shipped a wasm-based npm package. Benchmarking revealed:
- **Single-color calls were 7-25× slower** than pure JS (wasm boundary ~500ns/call)
- **Batch calls were 2-8× faster** (SIMD amortized over many pixels)
- Market research showed 95%+ of color-convert usage is single-color

The wasm boundary overhead made the npm package worse than the original for the
dominant use case. Something had to change.

## Decision

Three-layer architecture, replacing wasm entirely:

### Layer 1: Pure JS fast-path (single-color)

Ported the 9 hottest routes (rgb→hsl/hsv/cmyk/xyz/lab/oklab/hwb, hsl→rgb,
hsv→rgb) to hand-optimized JS. Runs at V8 JIT speed — **at parity with
color-convert** (0.8-1.05×).

### Layer 2: napi-rs native addon (batch SIMD + remaining routes)

Replaced wasm-pack with napi-rs (`#[napi]` exports). Advantages:
- **rayon multi-core** works natively (wasm is single-threaded)
- **GPU (CubeCL/wgpu)** works natively (no WebGPU abstraction)
- Simpler deployment (one `.node` file, no wasm-pack)
- Zero-copy `Uint8Array`/`Float32Array` for batch operations

### Layer 3: Auto-tier routing

The same `convert.rgb.hsl(input)` function detects input type and routes:

| Input | Routes to | Return type |
|-------|-----------|-------------|
| `(r, g, b)` numbers | JS single-color | `number[]` |
| `[r, g, b]` small array | JS single-color | `number[]` |
| `Uint8Array` (pixel data) | napi SIMD batch | `Float32Array` |
| Array > 300 elements | napi SIMD batch | `Float32Array` |

Users don't need to think about tiers — the package picks the fastest path.

## Research findings (why napi can't beat JS for single-color)

| Approach | Per-call | vs JS (~3ns) |
|----------|---------|------------|
| napi `Vec<f64>` return | ~450ns | 150× slower |
| napi `Float64Array` mutation (.into) | ~256ns | 85× slower |
| **Pure JS (V8 JIT)** | **~3ns** | **baseline** |
| Batch 100k amortized | ~2ns/px | **wins** |

The ~250ns napi boundary floor is irreducible — it's the cost of a C function
call through V8's runtime. No binding technology (wasm, napi, FFI) can beat
V8's inlined JIT for trivial arithmetic.

## Performance results

### Single-color (1M iterations)

| Route | color-convert | color-convert-rs | Ratio |
|-------|-------------|-----------------|-------|
| rgb→lab | 8.8M ops/s | 9.0M ops/s | 1.03× |
| rgb→oklab | 10.8M | 10.6M | 0.98× |
| hsl→rgb | 32.8M | 33.1M | 1.01× |

### Batch (100k pixels)

| Route | JS loop | auto-tier | Speedup |
|-------|---------|-----------|---------|
| rgb→lab | 7.6M | 103.4M | **13.5×** |
| rgb→oklab | 10.0M | 74.0M | **7.4×** |
| rgb→xyz | 15.1M | 97.4M | **6.5×** |

## Consequences

**Positive:**
- Single-color at parity (solves the v0.1.0 speed problem)
- Batch 10-15× faster (genuine value prop)
- Auto-tier = zero-friction UX
- napi gives rayon + GPU access that wasm couldn't
- No wasm-pack build step

**Negative:**
- Platform-specific `.node` binaries (need CI matrix for cross-platform)
- JS and Rust conversion code must be kept in sync
- More complex JS wrapper (detection + routing logic)

## References

- [[ADR-004]] — npm/wasm performance findings that triggered this
- [[ADR-005]] — image processing pivot
- [[04-napi-auto-tier]] — engineering details
