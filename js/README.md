# color-convert-rs

A [color-convert](https://www.npmjs.com/package/color-convert) compatible color conversion library with **auto-tiering**: pure JS for single-color (at parity with the original), napi-rs SIMD for batch pixel processing (10-15× faster).

## Why use this?

**Single-color conversions match color-convert speed.** The 9 hottest routes run as hand-optimized pure JS — V8's JIT compiles them to the same native code as color-convert.

**Batch processing is 10-15× faster.** Pass a `Uint8Array` of pixel data and the package auto-detects it, routing to Rust f32x8 SIMD via napi-rs — no API change needed.

| Route | color-convert (JS) | color-convert-rs (auto-tier) | Speedup |
|-------|-------------------:|------------------------------:|--------:|
| rgb→lab (100k px) | 7.6M ops/s | **103.4M ops/s** | **13.5×** |
| rgb→oklab (100k px) | 10.0M | **74.0M** | **7.4×** |
| rgb→xyz (100k px) | 15.1M | **97.4M** | **6.5×** |
| rgb→lab (single) | 8.8M | **9.0M** | **1.03×** |
| hsl→rgb (single) | 32.8M | **33.1M** | **1.01×** |

## Install

```bash
npm install color-convert-rs
```

## Quick start

```js
const convert = require('color-convert-rs');

// Single color — pure JS, same API as color-convert:
convert.rgb.hsl(255, 128, 0);       // → [30, 100, 50]
convert.rgb.hex(255, 128, 0);       // → 'FF8000'
convert.rgb.keyword(255, 0, 0);     // → 'red'
convert.hex.rgb('FF8000');          // → [255, 128, 0]
convert.hsl.rgb(30, 100, 50);       // → [255, 128, 0]

// Pixel array — auto-detected, napi SIMD batch:
const pixels = new Uint8Array([255, 0, 0, 0, 255, 0, 0, 0, 255]);
const lab = convert.rgb.lab(pixels);  // → Float32Array [53.24, 80.09, 67.20, ...]
```

## Auto-tiering

The same function automatically picks the fastest path based on input type:

| Input | Routes to | Speed | Return type |
|-------|-----------|-------|-------------|
| `convert.rgb.hsl(255, 128, 0)` | Pure JS | ~30M ops/s | `number[]` |
| `convert.rgb.hsl([255, 128, 0])` | Pure JS | ~30M ops/s | `number[]` |
| `convert.rgb.lab(uint8ArrayOfPixels)` | napi SIMD | 100M+ ops/s | `Float32Array` |
| `convert.rgb.lab(largeArrayOver300)` | napi SIMD | 100M+ ops/s | `Float32Array` |

No need to call `.batch()` explicitly — the package detects pixel data and routes accordingly. The `.batch()` and `.into()` APIs remain available for explicit control.

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

- `convert.<from>.<to>(...channels)` — auto-tiered (JS or napi based on input)
- `convert.<from>.<to>.raw(...channels)` — unrounded floats
- `convert.<from>.<to>.batch(uint8Array)` — force napi SIMD batch
- `convert.<from>.<to>.into(float64Array, r, g, b)` — force zero-alloc napi (6 routes)
- `convert.<model>.channels` — number of channels
- `convert.<model>.labels` — channel labels

## Supported models (17)

`rgb`, `hsl`, `hsv`, `hwb`, `cmyk`, `xyz`, `lab`, `lch`, `oklab`, `oklch`, `hex`, `keyword`, `ansi16`, `ansi256`, `hcg`, `apple`, `gray`

## How it works

1. **Single-color calls** route to hand-ported pure JS (`js/js-routes.js`). V8's JIT compiles the arithmetic to native code at ~3ns/call — no native boundary can beat this for trivial math.

2. **Batch calls** (typed array input) route to the Rust native addon via napi-rs. The Rust core uses f32x8 SIMD + rayon multi-core to process 100M+ pixels/sec.

3. **Auto-detection** checks `instanceof Uint8Array` (one check, ~0 overhead) and routes accordingly.

## Known limitations

- **Platform-specific binary**: ships a Linux `.node` file. macOS/Windows need cross-compiled builds (planned via CI matrix).
- **`gray.lch` hue**: for achromatic grayscale inputs (chroma=0), hue is arbitrary. Returns 180° vs color-convert's 0°. Both produce the same visible color. Affects 1 of 272 routes.

## License

MIT
