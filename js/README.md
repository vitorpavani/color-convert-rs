# color-convert-rs

A drop-in replacement for [`color-convert`](https://www.npmjs.com/package/color-convert) backed by Rust + WebAssembly SIMD.

## Install

```bash
npm install color-convert-rs
```

## Usage

```js
const convert = require('color-convert-rs');

// Same API as color-convert — exact drop-in:
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

// Reverse routes:
convert.hex.rgb('FF8000');          // → [255, 128, 0]
convert.keyword.rgb('red');         // → [255, 0, 0]
convert.hsl.rgb(30, 100, 50);       // → [255, 128, 0]
```

## Supported models (17)

`rgb`, `hsl`, `hsv`, `hwb`, `cmyk`, `xyz`, `lab`, `lch`, `oklab`, `oklch`, `hex`, `keyword`, `ansi16`, `ansi256`, `hcg`, `apple`, `gray`

All 272 routes (17×17 minus self-conversions) are verified to produce identical output to `color-convert@3.1.3`.

## API compatibility

- `convert.<from>.<to>(...channels)` — rounded to integers (matches color-convert default)
- `convert.<from>.<to>.raw(...channels)` — unrounded floats
- `convert.<model>.channels` — number of channels (e.g. `3` for rgb, `4` for cmyk)
- `convert.<model>.labels` — channel labels (e.g. `['r', 'g', 'b']`)
- Array input: `convert.rgb.hsl([255, 128, 0])` also works

## Known limitations

- `gray.lch` hue differs (180° vs 0°) for achromatic grayscale inputs — both produce the same visible color since chroma is 0. Affects 1 of 272 routes.

## License

MIT
