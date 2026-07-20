---
tags: [engineering, image-processing, batch, simd]
updated: 2026-07-20
---

# Image Batch API — stride-aware raw bytes + image crate

## Overview

After the npm performance findings ([ADR-004]) revealed single-color wasm is 7–25× slower than
JS, we pivoted to image processing ([ADR-005]). The key deliverable: `src/batch.rs` — a
stride-aware raw byte API that makes the crate a zero-friction drop-in for any pixel buffer.

## The API

```rust
// src/batch.rs
pub fn rgb_to_lab(input: &[u8], stride: usize) -> Vec<[f32; 3]>
pub fn rgb_to_xyz(input: &[u8], stride: usize) -> Vec<[f32; 3]>
pub fn rgb_to_hsl(input: &[u8], stride: usize) -> Vec<[f32; 3]>
pub fn rgb_to_hsv(input: &[u8], stride: usize) -> Vec<[f32; 3]>
pub fn rgb_to_oklab(input: &[u8], stride: usize) -> Vec<[f32; 3]>
pub fn rgb_to_cmyk(input: &[u8], stride: usize) -> Vec<[f32; 4]>
```

`stride` = bytes per pixel. `3` for RGB interleaved, `4` for RGBA (alpha channel skipped).

### Why stride matters

Real pixel data comes in different layouts:
- `image::RgbImage` → `&[u8]` with stride 3
- `image::RgbaImage` → `&[u8]` with stride 4
- Canvas `ImageData.data` → `Uint8ClampedArray` with stride 4
- Raw camera sensors → various bayer patterns

The stride parameter lets users pass any of these without copying or re-packing. The
`extract_rgb` helper chunks the input and pulls the first 3 bytes per pixel.

## image crate integration (feature-gated)

```rust
#[cfg(feature = "image")]
pub mod image {
    pub fn to_lab(img: &image::DynamicImage) -> Vec<[f32; 3]>
    pub fn to_xyz(img: &image::DynamicImage) -> Vec<[f32; 3]>
    pub fn to_oklab(img: &image::DynamicImage) -> Vec<[f32; 3]>
    pub fn to_hsl(img: &image::DynamicImage) -> Vec<[f32; 3]>
    pub fn to_hsv(img: &image::DynamicImage) -> Vec<[f32; 3]>
    pub fn to_cmyk(img: &image::DynamicImage) -> Vec<[f32; 4]>
}
```

Usage:
```rust
let img = image::open("photo.jpg")?;
let lab = color_convert_rs::batch::image::to_lab(&img);
```

## Throughput (1920×1080 frame, single-core, release)

| Route | Time | Throughput |
|-------|-----:|-----------:|
| rgb→hsl | 9ms | 224 M px/s |
| rgb→oklab | 12ms | 180 M px/s |
| rgb→hsv | 12ms | 177 M px/s |
| rgb→xyz | 18ms | 117 M px/s |
| rgb→cmyk | 19ms | 107 M px/s |
| rgb→lab | 29ms | 72 M px/s |

All routes convert a full HD frame in under 30ms — real-time video at 30+ fps.

## Tests

`tests/batch_stride.rs` (6 tests):
- `stride_3_rgb_matches_stride_4_rgba` — parity between RGB and RGBA input
- `stride_3_xyz_matches_stride_4` — same for XYZ route
- `batch_matches_typed_simd` — flat bytes vs typed `&[[u8; 3]]` produce identical output
- `cmyk_stride_works` — 4-channel output
- `stride_too_small_panics` — stride < 3 panics
- `image_dynamic_image_to_lab` — `image::DynamicImage` integration (feature-gated)

## Ecosystem positioning

| Crate | What it has | What it lacks |
|-------|------------|---------------|
| `image` | Pixel buffers (`ImageBuffer`, `DynamicImage`) | Color space conversion beyond grayscale |
| `palette` | Type-safe 20+ color spaces | No batch, no SIMD, single-color only |
| **`color-convert-rs`** | **17 spaces, f32x8 SIMD, rayon, GPU** | **Now has `image` integration** |

The `batch` module bridges the gap: `image` crate provides the pixels, we provide the fast
color conversion.

## Future

- **RGBA output**: currently output is RGB (`[f32; 3]`). For compositing pipelines, RGBA output
  with alpha pass-through would be useful.
- **In-place conversion**: for very large images, avoid the `Vec` allocation by writing into a
  caller-provided buffer.
- **YCbCr/YUV**: for video pipelines (would require Rec.709/Rec.2020 gamma curves).
