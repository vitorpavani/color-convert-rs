const native = require('./color_convert_rs.node');
const ccRs = require('./index.js');

const N = 1_000_000;
const WARMUP = 10_000;

function bench(name, fn) {
  for (let i = 0; i < WARMUP; i++) fn(i);
  const start = process.hrtime.bigint();
  for (let i = 0; i < N; i++) fn(i);
  const ms = Number(process.hrtime.bigint() - start) / 1e6;
  const ops = Math.round(N / (ms / 1000) / 1000);
  return { name, ms, ops };
}

const r = 255, g = 128, b = 0;

console.log(`napi benchmark: ${N.toLocaleString()} iterations\n`);
console.log('Path                          | Time     | Throughput');
console.log('----------------------------- | -------- | ----------');

// Generic convertRoute (string parsing + Vec alloc)
const a = bench('native.convertRoute("rgb","hsl",[r,g,b])', () => native.convertRoute('rgb', 'hsl', [r, g, b]));
console.log(a.name.padEnd(29) + ' | ' + `${a.ms.toFixed(0)}ms`.padEnd(8) + ` | ${a.ops}K ops/s`);

// Typed fast path (no strings, direct f64 args)
const b2 = bench('native.rgbHsl(r,g,b)', () => native.rgbHsl(r, g, b));
console.log(b2.name.padEnd(29) + ' | ' + `${b2.ms.toFixed(0)}ms`.padEnd(8) + ` | ${b2.ops}K ops/s`);

// JS wrapper (normalizeArgs + routing + napi call)
const c = bench('ccRs.rgb.hsl(r,g,b)', () => ccRs.rgb.hsl(r, g, b));
console.log(c.name.padEnd(29) + ' | ' + `${c.ms.toFixed(0)}ms`.padEnd(8) + ` | ${c.ops}K ops/s`);

console.log('');

const routes = [
  ['rgbHsl',   native.rgbHsl],
  ['rgbHsv',   native.rgbHsv],
  ['rgbLab',   native.rgbLab],
  ['rgbXyz',   native.rgbXyz],
  ['rgbOklab', native.rgbOklab],
  ['rgbCmyk',  native.rgbCmyk],
];

console.log('Typed fast-path routes:\n');
console.log('Route       | Time     | Throughput');
console.log('------------ | -------- | ----------');
for (const [name, fn] of routes) {
  const r2 = bench(name, () => fn(255, 128, 0));
  console.log(name.padEnd(11) + ' | ' + `${r2.ms.toFixed(0)}ms`.padEnd(8) + ` | ${r2.ops}K ops/s`);
}

console.log(`\nSpeedup: typed vs generic = ${(a.ms / b2.ms).toFixed(1)}x`);
console.log(`Speedup: typed vs wrapper = ${(c.ms / b2.ms).toFixed(1)}x`);
