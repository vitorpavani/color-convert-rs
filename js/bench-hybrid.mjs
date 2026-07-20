import { createRequire } from 'node:module';
import ccJs from 'color-convert';

const require = createRequire(import.meta.url);
const ccRs = require('./index.js');

const N = 1_000_000;
const WARMUP = 10_000;

function bench(name, fn) {
  for (let i = 0; i < WARMUP; i++) fn(i);
  const start = process.hrtime.bigint();
  for (let i = 0; i < N; i++) fn(i);
  const ms = Number(process.hrtime.bigint() - start) / 1e6;
  return { ms, ops: Math.round(N / (ms / 1000) / 1000) };
}

const routes = [
  ['rgb.hsl',   i => ccJs.rgb.hsl(i&255,(i>>8)&255,(i>>16)&255),    i => ccRs.rgb.hsl(i&255,(i>>8)&255,(i>>16)&255)],
  ['rgb.hsv',   i => ccJs.rgb.hsv(i&255,(i>>8)&255,(i>>16)&255),    i => ccRs.rgb.hsv(i&255,(i>>8)&255,(i>>16)&255)],
  ['rgb.cmyk',  i => ccJs.rgb.cmyk(i&255,(i>>8)&255,(i>>16)&255),   i => ccRs.rgb.cmyk(i&255,(i>>8)&255,(i>>16)&255)],
  ['rgb.lab',   i => ccJs.rgb.lab(i&255,(i>>8)&255,(i>>16)&255),    i => ccRs.rgb.lab(i&255,(i>>8)&255,(i>>16)&255)],
  ['rgb.xyz',   i => ccJs.rgb.xyz(i&255,(i>>8)&255,(i>>16)&255),    i => ccRs.rgb.xyz(i&255,(i>>8)&255,(i>>16)&255)],
  ['rgb.oklab', i => ccJs.rgb.oklab(i&255,(i>>8)&255,(i>>16)&255),  i => ccRs.rgb.oklab(i&255,(i>>8)&255,(i>>16)&255)],
  ['hsl.rgb',   i => ccJs.hsl.rgb(i%360,(i*7)%100,(i*13)%100),      i => ccRs.hsl.rgb(i%360,(i*7)%100,(i*13)%100)],
  ['hsv.rgb',   i => ccJs.hsv.rgb(i%360,(i*7)%100,(i*13)%100),      i => ccRs.hsv.rgb(i%360,(i*7)%100,(i*13)%100)],
];

console.log(`Single-color benchmark: ${N.toLocaleString()} iterations\n`);
console.log('Route       | color-convert (JS)   | color-convert-rs     | Speedup');
console.log('------------ | -------------------- | -------------------- | -------');

for (const [route, jsFn, rsFn] of routes) {
  const j = bench('js', jsFn);
  const r = bench('rs', rsFn);
  const speedup = (j.ms / r.ms).toFixed(2);
  const pad = (s, n) => s.padEnd(n);
  console.log(`${pad(route, 11)} | ${pad(j.ms.toFixed(0) + 'ms (' + j.ops + 'K)', 20)} | ${pad(r.ms.toFixed(0) + 'ms (' + r.ops + 'K)', 20)} | ${speedup}x`);
}
