// JS baseline benchmark runner for color-convert-rs.
//
// Times color-convert@3.1.3 (the JS baseline we are porting) over a deterministic
// pixel buffer and appends one `tier: "js"` record per route to the append-only
// benchmarks/results.jsonl ledger.  Re-running appends MORE lines — it never
// rewrites history.
//
// Usage:
//   node bench.mjs [N] [warmup] [timed-iters]
//   BENCH_INPUT_SIZE=500000 BENCH_WARMUP=5 BENCH_ITERS=30 node bench.mjs
//
// Defaults: N=100000, warmup=3, timed=20.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import convert from 'color-convert';

// ── Resolve paths relative to THIS script ──────────────────────────────
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..', '..');
const RESULTS_PATH = path.resolve(SCRIPT_DIR, '..', 'results.jsonl');

// ── Configurable knobs (env vars override positional args) ─────────────
const N = parseInt(
  process.env.BENCH_INPUT_SIZE || process.argv[2] || '100000',
  10,
);
const WARMUP = parseInt(
  process.env.BENCH_WARMUP || process.argv[3] || '3',
  10,
);
const TIMED_ITERS = parseInt(
  process.env.BENCH_ITERS || process.argv[4] || '20',
  10,
);

if (N <= 0 || WARMUP <= 0 || TIMED_ITERS <= 0) {
  process.stderr.write('ERROR: N, warmup, and timed-iters must be > 0\n');
  process.exit(1);
}

// ── Deterministic PRNG (mulberry32) ────────────────────────────────────
// Seeded PRNG so that every run produces the same pixel buffer — timing
// varies naturally but the INPUT is reproducible byte-for-byte.
function mulberry32(seed) {
  let state = seed | 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ── Generate N deterministic RGB pixels ────────────────────────────────
function generatePixels(n) {
  const rng = mulberry32(42);
  const pixels = new Array(n);
  for (let i = 0; i < n; i++) {
    pixels[i] = [(rng() * 256) | 0, (rng() * 256) | 0, (rng() * 256) | 0];
  }
  return pixels;
}

// ── Best-of-N timing harness ───────────────────────────────────────────
function benchmark(pixels, warmup, iters, fn) {
  // Warmup (cache / JIT)
  for (let i = 0; i < warmup; i++) {
    for (let j = 0; j < pixels.length; j++) {
      const [r, g, b] = pixels[j];
      fn(r, g, b);
    }
  }

  // Timed iterations — best-of-N (min wall time)
  let bestNs = Infinity;
  for (let i = 0; i < iters; i++) {
    const start = process.hrtime.bigint();
    for (let j = 0; j < pixels.length; j++) {
      const [r, g, b] = pixels[j];
      fn(r, g, b);
    }
    const elapsedNs = Number(process.hrtime.bigint() - start);
    if (elapsedNs < bestNs) bestNs = elapsedNs;
  }

  return bestNs;
}

// ── Route definitions ──────────────────────────────────────────────────
const ROUTES = {
  'rgb->hsl': (r, g, b) => convert.rgb.hsl(r, g, b),
  'rgb->lab': (r, g, b) => convert.rgb.lab(r, g, b),
  'rgb->xyz': (r, g, b) => convert.rgb.xyz(r, g, b),
  'rgb->hsv': (r, g, b) => convert.rgb.hsv(r, g, b),
  'rgb->cmyk': (r, g, b) => convert.rgb.cmyk(r, g, b),
  'rgb->hwb': (r, g, b) => convert.rgb.hwb(r, g, b),
  'rgb->hcg': (r, g, b) => convert.rgb.hcg(r, g, b),
  'rgb->oklab': (r, g, b) => convert.rgb.oklab(r, g, b),
  'rgb->apple': (r, g, b) => convert.rgb.apple(r, g, b),
  'rgb->hsl->rgb': (r, g, b) => {
    const hsl = convert.rgb.hsl(r, g, b);
    return convert.hsl.rgb(hsl[0], hsl[1], hsl[2]);
  },
};

// ── Inverse routes (non-rgb input) — pre-convert inputs outside timing ──
// These are measured separately: input generation is NOT timed, only the
// inverse conversion itself.  N=10M (JS OOMs at 50M on this host).
const INVERSE_ROUTES = {
  'oklab->rgb': {
    prepare: (pixels) => pixels.map(([r, g, b]) => convert.rgb.oklab(r, g, b)),
    run: (oklabPixels) => {
      for (let j = 0; j < oklabPixels.length; j++) {
        const [l, a, b] = oklabPixels[j];
        convert.oklab.rgb(l, a, b);
      }
    },
  },
};

// ── Build a schema-conformant JSONL record ─────────────────────────────
function makeRecord(bestNs, route) {
  const bestMs = bestNs / 1e6;
  const throughputMps = (N / 1e6) / (bestMs / 1000);

  let commit;
  try {
    commit = execSync('git rev-parse --short HEAD', {
      cwd: PROJECT_ROOT,
      encoding: 'utf-8',
    }).trim();
  } catch {
    commit = 'unknown';
  }

  return {
    // Required fields (see benchmarks/SCHEMA.md)
    ts: new Date().toISOString(),
    commit,
    issue: parseInt(process.env.BENCH_ISSUE || '18', 10),
    route,
    tier: 'js',
    input_size: N,
    metric: 'throughput_mpx_s',
    value: Math.round(throughputMps * 100) / 100, // 2 decimal places
    ms: Math.round(bestMs * 1000) / 1000,         // 3 decimal places
    iters: TIMED_ITERS,
    warmup: WARMUP,
    host: os.hostname(),
    gpu_present: false,
    // Optional fields
    decision: 'baseline',
    notes: `JS color-convert@3.1.3 baseline, N=${N.toLocaleString()}`,
  };
}

// ── Main ───────────────────────────────────────────────────────────────
const pixels = generatePixels(N);

process.stdout.write(
  `Generated ${pixels.length.toLocaleString()} deterministic pixels (seed=42)\n`,
);
process.stdout.write(`Warmup: ${WARMUP}   Timed iters: ${TIMED_ITERS}\n\n`);

const routeNames = Object.keys(ROUTES);
for (const route of routeNames) {
  const fn = ROUTES[route];
  const bestNs = benchmark(pixels, WARMUP, TIMED_ITERS, fn);
  const record = makeRecord(bestNs, route);

  // Append-only — never rewrite existing lines
  fs.appendFileSync(RESULTS_PATH, JSON.stringify(record) + '\n');

  // Human-readable summary
  const msOut = record.ms.toFixed(3);
  const mpsOut = record.value.toFixed(1);
  process.stdout.write(
    `${route.padEnd(18)}  N=${String(N).padStart(8)}  ` +
      `best=${msOut.padStart(9)} ms  ` +
      `${mpsOut.padStart(10)} MP/s\n`,
  );
}

// ── Inverse routes (pre-convert, time only the inverse) ─────────────────
const INVERSE_N = 10_000_000; // JS OOMs at 50M — use 10M
const inversePixels = generatePixels(INVERSE_N);

const inverseRouteNames = Object.keys(INVERSE_ROUTES);
for (const route of inverseRouteNames) {
  const { prepare, run } = INVERSE_ROUTES[route];

  // Pre-convert: generate input data for the inverse route (NOT timed)
  const preparedInput = prepare(inversePixels);

  // Warmup
  for (let w = 0; w < WARMUP; w++) {
    run(preparedInput);
  }

  // Timed iterations
  let bestNs = Infinity;
  for (let i = 0; i < TIMED_ITERS; i++) {
    const start = process.hrtime.bigint();
    run(preparedInput);
    const elapsedNs = Number(process.hrtime.bigint() - start);
    if (elapsedNs < bestNs) bestNs = elapsedNs;
  }

  const bestMs = bestNs / 1e6;
  const throughputMps = (INVERSE_N / 1e6) / (bestMs / 1000);

  const record = {
    ts: new Date().toISOString(),
    commit: (() => {
      try { return execSync('git rev-parse --short HEAD', { cwd: PROJECT_ROOT, encoding: 'utf-8' }).trim(); }
      catch { return 'unknown'; }
    })(),
    issue: parseInt(process.env.BENCH_ISSUE || '100', 10),
    route,
    tier: 'js',
    input_size: INVERSE_N,
    metric: 'throughput_mpx_s',
    value: Math.round(throughputMps * 100) / 100,
    ms: Math.round(bestMs * 1000) / 1000,
    iters: TIMED_ITERS,
    warmup: WARMUP,
    host: os.hostname(),
    gpu_present: false,
    decision: 'baseline',
    notes: `JS color-convert@3.1.3 baseline (inverse route), N=${INVERSE_N.toLocaleString()}`,
  };

  fs.appendFileSync(RESULTS_PATH, JSON.stringify(record) + '\n');

  const msOut = record.ms.toFixed(3);
  const mpsOut = record.value.toFixed(1);
  process.stdout.write(
    `${route.padEnd(18)}  N=${String(INVERSE_N).padStart(8)}  ` +
      `best=${msOut.padStart(9)} ms  ` +
      `${mpsOut.padStart(10)} MP/s  [JS inverse]\n`,
  );
}

process.stdout.write(
  `\nAppended ${routeNames.length + inverseRouteNames.length} records to ${RESULTS_PATH}\n`,
);
