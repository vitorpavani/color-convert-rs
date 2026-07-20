# color-convert-rs

A [color-convert](https://www.npmjs.com/package/color-convert) compatible color conversion library, accelerated with Rust + WebAssembly SIMD for **batch processing**.

## Why use this?

**For batch operations** (converting thousands of colors), this library is **2–8× faster** than `color-convert` thanks to Rust f32x8 SIMD inside WebAssembly:

| Route | color-convert (JS) | color-convert-rs (wasm batch) | Speedup |
|-------|-------------------:|------------------------------:|--------:|
| rgb→xyz | 11 ms | 1 ms | **7.6×** |
| rgb→lab | 16 ms | 2 ms | **6.8×** |
| rgb→cmyk | 6 ms | 2 ms | **3.8×** |
| rgb→hsv | 7 ms | 2 ms | **3.5×** |
| rgb→hsl | 7 ms | 3 ms | **2.3×** |
| rgb→oklab | 15 ms | 6 ms | **2.4×** |

*100,000 colors, best of 5 runs, Node.js 24.*

**For single-color conversions**, this library matches `color-convert` output exactly but is **not faster** — V8's JIT already compiles the JS conversion to native code, and the wasm call boundary adds overhead that outweighs the compute for a single color.

## Install

```bash
npm install color-convert-rs
```

## Quick start

```js
const convert = require('color-convert-rs');

// Single-color — drop-in compatible with color-convert:
convert.rgb.hsl(255, 128, 0);       // → [30, 100, 50]
convert.rgb.hex(255, 128, 0);       // → 'FF8000'
convert.rgb.keyword(255, 0, 0);     // → 'red'
convert.hex.rgb('FF8000');          // → [255, 128, 0]
```

## Batch API (the fast path)

For processing large arrays of colors — image processing, data pipelines, palettes — use `.batch()`:

```js
// Input: flat Uint8Array of [r,g,b, r,g,b, ...]
const pixels = new Uint8Array([
  255, 0, 0,    // red
  0, 255, 0,    // green
  0, 0, 255,    // blue
]);

// Output: flat Float32Array of the target model's channels
const lab = convert.rgb.lab.batch(pixels);
// → Float32Array [ 53.24, 80.09, 67.20,  // red   in LAB
//                   87.82, -86.18, 83.18, // green in LAB
//                   32.30, 79.20, -107.86 ] // blue  in LAB

// Available batch routes:
convert.rgb.hsl.batch(pixels)    // → Float32Array (3 channels per pixel)
convert.rgb.hsv.batch(pixels)
convert.rgb.lab.batch(pixels)
convert.rgb.xyz.batch(pixels)
convert.rgb.cmyk.batch(pixels)   // → Float32Array (4 channels per pixel)
convert.rgb.oklab.batch(pixels)
convert.hsl.rgb.batch(hslFloat32Array)  // inverse: f32 input → f32 output
convert.hsv.rgb.batch(hsvFloat32Array)
```

**Why batch is faster**: a single `batch()` call crosses the JS→wasm boundary once and processes all colors with f32x8 SIMD inside. A JS loop calls the conversion function N times, paying function-call overhead each time.

## Full API (single-color)

All 17 models from `color-convert`, all 272 routes, verified to produce identical output to `color-convert@3.1.3`:

```js
convert.rgb.hsl(255, 128, 0);       // → [30, 100, 50]
convert.rgb.hsv(255, 128, 0);       // → [30, 100, 100]
convert.rgb.cmyk(128, 64, 32);      // → [0, 50, 75, 50]
convert.rgb.hex(255, 128, 0);       // → 'FF8000'
convert.rgb.keyword(255, 0, 0);     // → 'red'
convert.rgb.ansi16(255, 0, 0);      // → 91
convert.rgb.ansi256(255, 0, 0);     // → 196
convert.rgb.lab(255, 128, 0);       // → [67, 43, 74]
convert.rgb.xyz(255, 128, 0);       // → [49, 37, 5]
convert.rgb.oklab(255, 128, 0);     // → [73, 11, 15]

convert.hex.rgb('FF8000');          // → [255, 128, 0]
convert.keyword.rgb('red');         // → [255, 0, 0]
convert.hsl.rgb(30, 100, 50);       // → [255, 128, 0]
```

## API reference

- `convert.<from>.<to>(...channels)` — rounded to integers (matches color-convert default)
- `convert.<from>.<to>.raw(...channels)` — unrounded floats
- `convert.<from>.<to>.batch(uint8Array)` — SIMD batch, 2–8× faster (8 routes available)
- `convert.<model>.channels` — number of channels (e.g. `3` for rgb, `4` for cmyk)
- `convert.<model>.labels` — channel labels (e.g. `['r', 'g', 'b']`)
- Array input: `convert.rgb.hsl([255, 128, 0])` also works

## Supported models (17)

`rgb`, `hsl`, `hsv`, `hwb`, `cmyk`, `xyz`, `lab`, `lch`, `oklab`, `oklch`, `hex`, `keyword`, `ansi16`, `ansi256`, `hcg`, `apple`, `gray`

## Known limitations

- **Single-color speed**: for one-off conversions, `color-convert` (pure JS) is faster — V8's JIT compiles it to native code with no wasm boundary overhead. Use this library when converting **many colors at once** via `.batch()`.
- **`gray.lch` hue**: for achromatic grayscale inputs (chroma=0), the hue is arbitrary. This library returns 180° while `color-convert` returns 0°. Both produce the same visible color. Affects 1 of 272 routes.

## License

MIT
