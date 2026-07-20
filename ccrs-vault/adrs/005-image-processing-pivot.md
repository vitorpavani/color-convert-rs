---
adr: 005
title: Image processing pivot — from JS port to batch image library
status: accepted
date: 2026-07-20
tags: [adr, strategy, image-processing, positioning]
---

# ADR-005: Image Processing Pivot

## Context

After publishing the npm package ([ADR-004]), market research revealed:
- `color-convert` is used **95%+ for single-color** (chalk, CSS, themes)
- **Nobody batches with it** — image processing uses `sharp`/`jimp`, data viz uses `d3-color`
- Our wasm package is 7–25× slower for the dominant use case (single-color)

The npm package's value prop was weak. But the **Rust crate** has a genuine advantage that the
npm package can't express: 111M px/s SIMD, multi-core rayon, GPU kernels — **for batch image
processing**.

## Research: the Rust color ecosystem

| Crate | Color spaces | Batch/SIMD | GPU | Speed |
|-------|:-----------:|:----------:|:---:|:-----:|
| `image` | RGB/RGBA/Luma only | — | — | — |
| `palette` | 20+ spaces, type-safe | single-color | — | slow |
| `color-convert-rs` | **17 spaces** | **f32x8 + rayon** | **CubeCL** | **111M px/s** |

Nobody covers all four axes. `image` has pixels but no color conversion. `palette` has
conversions but no batch. **We fill the gap.**

## Decision

**Reposition from "drop-in JS port" to "fast Rust color conversion for batch image processing."**

Three concrete moves:

### Move 1: Stride-aware raw byte API (`src/batch.rs`)

```rust
pub fn rgb_to_lab(input: &[u8], stride: usize) -> Vec<[f32; 3]>
// stride=3 for RGB, stride=4 for RGBA (alpha skipped)
```

Zero-friction: any pixel buffer works without re-packing. Wraps existing SIMD.

### Move 2: `image` crate integration (feature-gated)

```rust
let img = image::open("photo.jpg")?;
let lab = color_convert_rs::batch::image::to_lab(&img); // one-liner
```

### Move 3: README + npm repositioning

- Rust README leads with "batch image processing" + frame-level throughput
- npm README is honest: "single-color not faster, batch 2–8× faster"

## Measured throughput (1920×1080 frame, single-core)

| Route | Time | Throughput |
|-------|-----:|-----------:|
| rgb→hsl | 9ms | 224 M px/s |
| rgb→oklab | 12ms | 180 M px/s |
| rgb→xyz | 18ms | 117 M px/s |
| rgb→lab | 29ms | 72 M px/s |

All routes process a full HD frame in <30ms — real-time video at 30+ fps.

## Consequences

**Positive:**
- Clear value prop: "full HD frame in <30ms, 17 color spaces"
- Fills a real gap in the Rust ecosystem (between `image` and `palette`)
- The GPU path becomes relevant (real-time video processing)
- The npm package finds a niche (browser image editors, Node.js batch processing)

**Negative:**
- The "drop-in color-convert replacement" claim is weakened
- Requires maintaining both the Rust-first and npm stories
- Future: may need YCbCr/Rec.709/Rec.2020 for true video support

## References

- [[ADR-004]] — the npm performance findings that triggered this pivot
- [[03-image-batch-api]] — the batch API engineering doc
- [README](../README.md) — repositioned for image processing
