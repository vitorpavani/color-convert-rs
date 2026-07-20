# color-convert-rs

Fast Rust color-space conversion for **batch image and video processing** — f32x8 SIMD,
multi-core rayon, and optional GPU (CubeCL). Converts full HD frames in under 30ms.

```rust
use color_convert_rs::batch;

// Any raw pixel buffer — RGB or RGBA, no re-packing:
let lab: Vec<[f32; 3]> = batch::rgb_to_lab(&pixel_bytes, 3);  // stride=3 for RGB
let lab: Vec<[f32; 3]> = batch::rgb_to_lab(&rgba_bytes, 4);   // stride=4 for RGBA (alpha skipped)
```

## Performance

Full 1920×1080 frame (2M pixels), single-core, release build:

| Route | Time | Throughput |
|-------|-----:|-----------:|
| rgb→hsl | 9ms | **224 M px/s** |
| rgb→oklab | 12ms | **180 M px/s** |
| rgb→hsv | 12ms | **177 M px/s** |
| rgb→xyz | 18ms | **117 M px/s** |
| rgb→cmyk | 19ms | **107 M px/s** |
| rgb→lab | 29ms | **72 M px/s** |

Multi-core (rayon, 28 cores): rgb→lab hits **164 M px/s**. All routes process a full HD frame in under 30ms — real-time video at 30+ fps.

## Quick start

### Raw pixel buffers (no dependencies)

```rust
use color_convert_rs::batch;

// Flat &[u8] — works with any pixel layout. stride = bytes per pixel.
let pixels: &[u8] = &[255, 0, 0, 0, 255, 0, 0, 0, 255]; // 3 pixels: red, green, blue
let lab = batch::rgb_to_lab(pixels, 3);
// → [[53.24, 80.09, 67.20], [87.73, -86.18, 83.18], [32.30, 79.19, -107.86]]

let xyz = batch::rgb_to_xyz(pixels, 3);
let hsl = batch::rgb_to_hsl(pixels, 3);
let oklab = batch::rgb_to_oklab(pixels, 3);
let cmyk = batch::rgb_to_cmyk(pixels, 3); // → Vec<[f32; 4]>
```

### `image` crate integration (feature-gated)

```toml
[dependencies]
color-convert-rs = { version = "0.1", features = ["image"] }
image = "0.25"
```

```rust
use color_convert_rs::batch::image;

let img = image::open("photo.jpg")?;
let lab = image::to_lab(&img);      // one-liner: DynamicImage → Vec<[f32; 3]>
let xyz = image::to_xyz(&img);
let oklab = image::to_oklab(&img);
```

### Single-color conversion (any-to-any)

All 17 models × 272 routes, behavior-verified against the JS `color-convert` reference:

```rust
use color_convert_rs::{convert, convert_rounded, Color, Model};

let orange = Color::Rgb([255.0, 128.0, 0.0]);
let lab = convert_rounded(Model::Rgb, Model::Lab, orange)?; // rounded to integers
let lab_raw = convert(Model::Rgb, Model::Lab, orange)?;     // unrounded floats
```

## Feature flags

| Feature | What it enables | Dependencies added |
|---------|----------------|-------------------|
| *(default)* | CPU SIMD batch + single-color conversion | `wide`, `rayon`, `thiserror` |
| `image` | `batch::image::to_lab()` etc. for `image::DynamicImage` | `image` |
| `gpu` | GPU kernels via CubeCL/wgpu + runtime probe | `cubecl`, `wgpu`, `pollster`, `bytemuck` |
| `wasm` | wasm-bindgen exports for npm package | `wasm-bindgen`, `js-sys` |

```toml
# Minimal — CPU SIMD only (no GPU deps):
color-convert-rs = "0.1"

# With image crate:
color-convert-rs = { version = "0.1", features = ["image"] }

# Everything:
color-convert-rs = { version = "0.1", features = ["image", "gpu"] }
```

## Supported color spaces (17)

`rgb`, `hsl`, `hsv`, `hwb`, `cmyk`, `xyz`, `lab`, `lch`, `oklab`, `oklch`, `hcg`, `apple`,
`gray`, `hex`, `keyword`, `ansi16`, `ansi256`

All 272 routes (17×17 minus self-conversions) are validated against reference vectors generated
from the [`color-convert`](https://github.com/Qix-/color-convert) JS library.

## Architecture

```
src/
├── batch.rs          ── Stride-aware raw byte API + image crate integration
├── convert.rs        ── BFS route graph + convert() + convert_batch() (fused multi-hop)
├── simd.rs           ── f32x8 SIMD: rgb↔xyz↔lab + sRGB LUT + fast cbrt
├── simd_hsl.rs       ── SIMD rgb↔hsl
├── simd_hsv.rs       ── SIMD rgb→hsv
├── simd_hsv_rgb.rs   ── SIMD hsv→rgb
├── simd_cmyk.rs      ── SIMD rgb→cmyk
├── simd_oklab.rs     ── SIMD rgb→oklab
├── simd_parallel.rs  ── Generic rayon par_batch (multi-core dispatch)
├── gpu.rs            ── CubeCL kernels (feature-gated)
├── probe.rs          ── Runtime GPU capability probe
└── {rgb,hsl,hsv,...}.rs ── Scalar [f64] routes (one module per model)
```

## Build

```bash
# Standard:
cargo build --release

# With image crate:
cargo build --release --features image

# Run examples:
cargo run --release --example image_to_lab --features image -- your_photo.jpg
cargo run --release --example batch_simd        # synthetic 100k pixel benchmark
cargo run --release --example basic_convert     # single-color demo
```

**NixOS**: `nix develop` provides a shell with gcc, wasm-pack, and nodejs pre-configured
(see `flake.nix`). The Rust linker (`ld-wrapper.sh`) issue is handled automatically.

## Optimization stack

The `rgb→lab` single-core journey: **10.8 → 111.3 MP/s** (10.3× cumulative over 10 waves).

| Optimization | Effect |
|-------------|--------|
| f32x8 SIMD batch (wide crate) | 2× over scalar |
| Fused rgb→xyz→lab single pass | +10.9% (drops intermediate buffer) |
| sRGB inverse-gamma LUT (exact 256-entry) | 3.28× on xyz, 2.12× on lab |
| Fast cbrt (bit-hack + Newton-Raphson) | +63.7% on lab fused |
| Multi-core rayon par_batch | 1.42× on lab (memory-bandwidth-bound) |

Every numeric RGB-source route — both forward (rgb→X) and inverse (X→rgb) — has a vectorized
f32x8 SIMD path. The CPU optimization surface is exhausted; the remaining ceiling is memory
bandwidth, not CPU compute.

See [`benchmarks/README.md`](./benchmarks/README.md) for the per-route benchmark rollup and
[`CHANGELOG.md`](./CHANGELOG.md) for version history.

## npm package

A WebAssembly build is available as [`color-convert-rs`](https://www.npmjs.com/package/color-convert-rs)
on npm — a drop-in replacement for `color-convert` with batch SIMD for JS/Node.js image processing.

```bash
npm install color-convert-rs
```

```js
const convert = require('color-convert-rs');
convert.rgb.lab.batch(new Uint8Array([255, 0, 0, 0, 255, 0])); // Float32Array
```

## License

MIT — see [`LICENSE`](./LICENSE).
