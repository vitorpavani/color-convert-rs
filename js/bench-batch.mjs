import { createRequire } from 'node:module';
import ccJs from 'color-convert';

const require = createRequire(import.meta.url);
const wasm = require('./color_convert_rs.js');

const N = 100_000;
const WARMUP_N = 10_000;

const pixels = new Uint8Array(N * 3);
for (let i = 0; i < N; i++) {
  pixels[i * 3]     = i & 255;
  pixels[i * 3 + 1] = (i >> 8) & 255;
  pixels[i * 3 + 2] = (i * 37) & 255;
}

function benchJs(route, jsFn) {
  for (let i = 0; i < WARMUP_N; i++) jsFn(pixels.subarray(i * 3, i * 3 + 3));
  const runs = 5;
  let best = Infinity;
  for (let r = 0; r < runs; r++) {
    const start = process.hrtime.bigint();
    for (let i = 0; i < N; i++) {
      jsFn(pixels.subarray(i * 3, i * 3 + 3));
    }
    const elapsed = Number(process.hrtime.bigint() - start) / 1e6;
    best = Math.min(best, elapsed);
  }
  return best;
}

function benchWasm(route, batchFn) {
  for (let i = 0; i < 1; i++) batchFn(pixels);
  const runs = 5;
  let best = Infinity;
  for (let r = 0; r < runs; r++) {
    const start = process.hrtime.bigint();
    batchFn(pixels);
    const elapsed = Number(process.hrtime.bigint() - start) / 1e6;
    best = Math.min(best, elapsed);
  }
  return best;
}

const routes = [
  ['rgb→hsl',   (px) => ccJs.rgb.hsl(px[0], px[1], px[2]),     wasm.rgb_to_hsl_batch],
  ['rgb→hsv',   (px) => ccJs.rgb.hsv(px[0], px[1], px[2]),     wasm.rgb_to_hsv_batch],
  ['rgb→cmyk',  (px) => ccJs.rgb.cmyk(px[0], px[1], px[2]),    wasm.rgb_to_cmyk_batch],
  ['rgb→lab',   (px) => ccJs.rgb.lab(px[0], px[1], px[2]),     wasm.rgb_to_lab_batch],
  ['rgb→xyz',   (px) => ccJs.rgb.xyz(px[0], px[1], px[2]),     wasm.rgb_to_xyz_batch],
  ['rgb→oklab', (px) => ccJs.rgb.oklab(px[0], px[1], px[2]),   wasm.rgb_to_oklab_batch],
];

console.log(`Batch benchmark: ${N.toLocaleString()} colors, best of 5 runs\n`);
console.log('Route       | JS loop (per-call)     | wasm batch (SIMD)     | Speedup');
console.log('------------ | ---------------------- | --------------------- | -------');

for (const [route, jsFn, batchFn] of routes) {
  const jsMs = benchJs(route, jsFn);
  const rsMs = benchWasm(route, batchFn);
  const speedup = (jsMs / rsMs).toFixed(1);
  const jsStr = `${jsMs.toFixed(0)}ms (${Math.round(N / (jsMs / 1000) / 1000)}M ops/s)`;
  const rsStr = `${rsMs.toFixed(0)}ms (${Math.round(N / (rsMs / 1000) / 1000)}M ops/s)`;
  const pad = (s, n) => s.padEnd(n);
  console.log(`${pad(route, 11)} | ${pad(jsStr, 22)} | ${pad(rsStr, 21)} | ${speedup}x`);
}
