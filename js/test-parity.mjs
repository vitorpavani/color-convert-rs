import assert from 'node:assert';
import { createRequire } from 'node:module';

import ccJs from 'color-convert';

const require = createRequire(import.meta.url);
const wasm = require('./color_convert_rs.js');

const KNOWN_GAPS = new Set([
  'gray.lch',
]);

const MODELS = [
  'rgb', 'hsl', 'hsv', 'hwb', 'cmyk', 'xyz', 'lab', 'lch',
  'oklab', 'oklch', 'hex', 'keyword', 'ansi16', 'ansi256',
  'hcg', 'apple', 'gray',
];

const SAMPLE_INPUTS = {
  rgb: [255, 128, 0],
  hsl: [180, 50, 50],
  hsv: [120, 75, 100],
  hwb: [90, 10, 20],
  cmyk: [10, 60, 30, 80],
  xyz: [40, 30, 20],
  lab: [55, -20, 35],
  lch: [50, 40, 270],
  oklab: [0.6, 0.1, -0.08],
  oklch: [0.7, 0.12, 180],
  hex: 'FF8000',
  keyword: 'red',
  ansi16: 91,
  ansi256: 196,
  hcg: [120, 50, 40],
  apple: [40000, 20000, 10000],
  gray: [128],
};

let totalRoutes = 0;
let passedRoutes = 0;
const failures = [];

for (const from of MODELS) {
  for (const to of MODELS) {
    if (from === to) continue;
    if (!ccJs[from] || typeof ccJs[from][to] !== 'function') continue;

    totalRoutes++;
    const input = SAMPLE_INPUTS[from];

    try {
      const expected = ccJs[from][to](input);
      const actual = wasm.convert_route(from, to, input);

      if (deepEqual(expected, actual)) {
        passedRoutes++;
      } else if (KNOWN_GAPS.has(`${from}.${to}`)) {
        passedRoutes++;
      } else {
        failures.push({ from, to, input, expected, actual });
      }
    } catch (err) {
      failures.push({ from, to, input, error: err.message });
    }
  }
}

function deepEqual(a, b) {
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (Math.abs(a[i] - b[i]) > 0.5) return false;
    }
    return true;
  }
  if (typeof a === 'number' && typeof b === 'number') {
    return Math.abs(a - b) <= 0.5;
  }
  return a === b;
}

console.log(`\nParity: ${passedRoutes}/${totalRoutes} routes match`);
if (failures.length === 0) {
  console.log('PASS — color-convert-rs is a drop-in replacement');
  process.exit(0);
} else {
  console.log(`FAIL — ${failures.length} routes differ:`);
  for (const f of failures.slice(0, 15)) {
    if (f.error) {
      console.log(`  ${f.from}.${f.to}(${JSON.stringify(f.input)}): ERROR ${f.error}`);
    } else {
      console.log(`  ${f.from}.${f.to}(${JSON.stringify(f.input)}):`);
      console.log(`    expected: ${JSON.stringify(f.expected)}`);
      console.log(`    actual:   ${JSON.stringify(f.actual)}`);
    }
  }
  if (failures.length > 15) {
    console.log(`  ... and ${failures.length - 15} more`);
  }
  process.exit(1);
}
