import { createRequire } from 'node:module';
import ccJs from 'color-convert';

const require = createRequire(import.meta.url);
const ccRs = require('./index.js');

const ITERATIONS = 1_000_000;
const WARMUP = 10_000;

function bench(name, fn) {
  for (let i = 0; i < WARMUP; i++) fn(i);
  const start = process.hrtime.bigint();
  for (let i = 0; i < ITERATIONS; i++) fn(i);
  const elapsed = Number(process.hrtime.bigint() - start) / 1e6;
  const opsPerSec = Math.round(ITERATIONS / (elapsed / 1000));
  return { name, elapsed, opsPerSec };
}

const routes = [
  ['rgb.hsl',   (i) => ccJs.rgb.hsl(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.hsl(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.hsv',   (i) => ccJs.rgb.hsv(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.hsv(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.cmyk',  (i) => ccJs.rgb.cmyk(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.cmyk(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.lab',   (i) => ccJs.rgb.lab(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.lab(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.xyz',   (i) => ccJs.rgb.xyz(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.xyz(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.oklab', (i) => ccJs.rgb.oklab(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.oklab(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['rgb.hex',   (i) => ccJs.rgb.hex(i & 255, (i >> 8) & 255, (i >> 16) & 255),
               (i) => ccRs.rgb.hex(i & 255, (i >> 8) & 255, (i >> 16) & 255)],
  ['hsl.rgb',   (i) => ccJs.hsl.rgb(i % 360, (i * 7) % 100, (i * 13) % 100),
               (i) => ccRs.hsl.rgb(i % 360, (i * 7) % 100, (i * 13) % 100)],
];

console.log(`Single-color benchmark: ${ITERATIONS.toLocaleString()} iterations per route\n`);
console.log('Route       | color-convert (JS)     | color-convert-rs (wasm) | Speedup');
console.log('------------ | ---------------------- | ----------------------- | -------');

for (const [route, jsFn, rsFn] of routes) {
  const jsResult = bench(`js.${route}`, jsFn);
  const rsResult = bench(`rs.${route}`, rsFn);
  const speedup = (jsResult.elapsed / rsResult.elapsed).toFixed(2);
  const jsStr = `${jsResult.elapsed.toFixed(0)}ms (${(jsResult.opsPerSec/1000).toFixed(0)}K ops/s)`;
  const rsStr = `${rsResult.elapsed.toFixed(0)}ms (${(rsResult.opsPerSec/1000).toFixed(0)}K ops/s)`;
  const pad = (s, n) => s.padEnd(n);
  console.log(`${pad(route, 11)} | ${pad(jsStr, 22)} | ${pad(rsStr, 23)} | ${speedup}x`);
}
