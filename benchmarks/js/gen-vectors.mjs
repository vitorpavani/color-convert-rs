// Reference test-vector generator for color-convert-rs.
//
// Source of truth: the PUBLIC wrapper output of color-convert@3.1.3 (pinned via
// package-lock.json, with its transitive dep color-name@2.1.0). Array outputs are
// rounded by the wrapper (Math.round per element); hex/keyword outputs are strings;
// ansi16/ansi256 outputs are numbers. That rounded public output is exactly the
// observable behavior the Rust port must match (AGENTS.md Rule 8).
//
// One JSON file per NATIVE conversions.js route (50 routes) is written to
// tests/vectors/<from>_to_<to>.json. Inputs are fully deterministic (no randomness,
// no timestamps) and sorted, so re-running produces byte-identical files.
//
// Inputs for perceptual/derived models (xyz, lab, lch, oklab, oklch, hcg) are NOT
// hand-picked: they are derived by converting the deterministic rgb input set into
// that model via the public convert, so they are realistic in-gamut values.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import convert from 'color-convert';

const SOURCE = 'color-convert@3.1.3';
const OUT_DIR = path.join(
	path.dirname(fileURLToPath(import.meta.url)),
	'..', '..', 'tests', 'vectors',
);

// The 50 native routes defined in color-convert@3.1.3 conversions.js.
const ROUTES = {
	rgb: ['hsl', 'hsv', 'hwb', 'oklab', 'cmyk', 'keyword', 'xyz', 'lab', 'ansi16', 'ansi256', 'hex', 'hcg', 'apple', 'gray'],
	hsl: ['rgb', 'hsv', 'hcg'],
	hsv: ['rgb', 'hsl', 'ansi16', 'hcg'],
	hwb: ['rgb', 'hcg'],
	cmyk: ['rgb'],
	xyz: ['rgb', 'lab', 'oklab'],
	lab: ['xyz', 'lch'],
	lch: ['lab'],
	oklab: ['oklch', 'xyz', 'rgb'],
	oklch: ['oklab'],
	hcg: ['rgb', 'hsv', 'hsl', 'hwb'],
	apple: ['rgb'],
	gray: ['rgb', 'hsl', 'hsv', 'hwb', 'cmyk', 'lab', 'hex'],
	keyword: ['rgb'],
	hex: ['rgb'],
	ansi16: ['rgb'],
	ansi256: ['rgb'],
};

function dedupe(inputs) {
	const seen = new Set();
	const out = [];
	for (const input of inputs) {
		const key = JSON.stringify(input);
		if (!seen.has(key)) {
			seen.add(key);
			out.push(input);
		}
	}
	return out;
}

function compareInputs(a, b) {
	if (typeof a === 'string') {
		return a < b ? -1 : a > b ? 1 : 0;
	}
	if (typeof a === 'number') {
		return a - b;
	}
	for (let i = 0; i < Math.min(a.length, b.length); i++) {
		if (a[i] !== b[i]) {
			return a[i] - b[i];
		}
	}
	return a.length - b.length;
}

function rgbInputs() {
	const inputs = [];
	// Named boundary/special cases: black, white, pure channels, greys, off-grid values.
	inputs.push(
		[0, 0, 0], [255, 255, 255],
		[255, 0, 0], [0, 255, 0], [0, 0, 255],
		[128, 128, 128], [64, 64, 64], [192, 192, 192],
		[140, 200, 100], [1, 2, 3], [254, 1, 127],
	);
	// Small deterministic grid: 3 steps per channel -> 27 combinations.
	const grid = [0, 128, 255];
	for (const r of grid) {
		for (const g of grid) {
			for (const b of grid) {
				inputs.push([r, g, b]);
			}
		}
	}
	return dedupe(inputs);
}

// hsl / hsv / hwb share the same deterministic sampling grid:
// hue 0..360 x {0,50,100} x {0,50,100} for the two percentage channels.
function hueModelInputs() {
	const inputs = [];
	const hues = [0, 60, 120, 180, 240, 300, 360];
	const percents = [0, 50, 100];
	for (const h of hues) {
		for (const p1 of percents) {
			for (const p2 of percents) {
				inputs.push([h, p1, p2]);
			}
		}
	}
	return inputs;
}

function cmykInputs() {
	return [
		[0, 0, 0, 0], [0, 0, 0, 100],
		[100, 0, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0],
		[30, 0, 50, 22],
		[0, 0, 0, 25], [0, 0, 0, 50], [0, 0, 0, 75],
	];
}

// Inputs for perceptual models are derived from the rgb set through the public
// convert so they stay in a sensible, in-gamut domain (see top comment).
function derivedInputs(model) {
	return dedupe(rgbInputs().map((rgb) => convert.rgb[model](...rgb)));
}

function grayInputs() {
	return [[0], [25], [50], [75], [100]];
}

function keywordInputs() {
	// Includes the gray/grey alias pair on purpose.
	return [
		'black', 'blue', 'cyan', 'gray', 'green', 'grey', 'magenta',
		'navy', 'olive', 'purple', 'rebeccapurple', 'red', 'teal',
		'white', 'yellow',
	];
}

function hexInputs() {
	// 'ABC' exercises the 3-digit shorthand the library expands to 'AABBCC'.
	return ['000000', '8CC864', 'ABC', 'FFFFFF'];
}

function ansi16Inputs() {
	const inputs = [];
	for (const base of [30, 40, 90, 100]) {
		for (let offset = 0; offset < 8; offset++) {
			inputs.push(base + offset);
		}
	}
	return inputs;
}

function ansi256Inputs() {
	return [0, 8, 16, 21, 100, 196, 231, 232, 244, 255];
}

function appleInputs() {
	return [
		[0, 0, 0], [65535, 65535, 65535],
		[65535, 0, 0], [0, 65535, 0], [0, 0, 65535],
		[32768, 32768, 32768], [10000, 20000, 30000],
	];
}

const INPUT_BUILDERS = {
	rgb: rgbInputs,
	hsl: hueModelInputs,
	hsv: hueModelInputs,
	hwb: hueModelInputs,
	cmyk: cmykInputs,
	xyz: () => derivedInputs('xyz'),
	lab: () => derivedInputs('lab'),
	lch: () => derivedInputs('lch'),
	oklab: () => derivedInputs('oklab'),
	oklch: () => derivedInputs('oklch'),
	hcg: () => derivedInputs('hcg'),
	apple: appleInputs,
	gray: grayInputs,
	keyword: keywordInputs,
	hex: hexInputs,
	ansi16: ansi16Inputs,
	ansi256: ansi256Inputs,
};

function callPublicConvert(from, to, input) {
	const fn = convert[from][to];
	if (typeof fn !== 'function') {
		throw new Error(`public convert.${from}.${to} is not a function`);
	}
	return Array.isArray(input) ? fn(...input) : fn(input);
}

function emitRouteFile(from, to, inputs, sourceLabel) {
	const filename = `${from}_to_${to}`;
	const cases = inputs.map((input) => ({
		input,
		expected: callPublicConvert(from, to, input),
	}));
	if (cases.length < 3) {
		throw new Error(`route ${from}->${to} has fewer than 3 cases`);
	}
	const file = path.join(OUT_DIR, `${filename}.json`);
	const payload = { from, to, source: sourceLabel, cases };
	fs.writeFileSync(file, `${JSON.stringify(payload, null, 2)}\n`);
	return { count: 1, cases: cases.length };
}

// Non-native (multi-hop) routes exercised by the public convert API, which
// auto-derives the path via BFS in route.js.  Inputs are derived by converting
// the deterministic RGB set into the source model so every input is realistic
// and in-gamut.  The 15 routes below are the ones the `convert` API must
// exercise in its own test suite (Re: #17).
const MULTI_HOP_ROUTES = [
	['cmyk', 'hsl'],
	['hsl', 'lab'],
	['lab', 'rgb'],
	['hsv', 'xyz'],
	['hwb', 'hsl'],
	['cmyk', 'hsv'],
	['xyz', 'hsl'],
	['lab', 'hsl'],
	['hcg', 'lab'],
	['gray', 'rgb'],
	['hsl', 'hex'],
	['cmyk', 'keyword'],
	['lab', 'ansi16'],
	['oklab', 'hsl'],
	['hwb', 'rgb'],
];

// Derive deterministic input set for a given source model by converting
// the fixed RGB input set via the public convert.
function deriveSourceInputs(toModel) {
	return dedupe(rgbInputs().map((rgb) => callPublicConvert('rgb', toModel, rgb)));
}

/** Return a model label that isn't a native-route source (e.g. "cmyk", "hsl" are
 * also native sources — inputs from the native builders).  The output here is
 * always a labelled JSON array representing the source channels. */
function nativeInputs(model) {
	const inputs = INPUT_BUILDERS[model]();
	if (inputs.length >= 3) return inputs;
	// fallback: derive from rgb (e.g. oklab)
	return deriveSourceInputs(model);
}

// Source-label suffix to distinguish deterministic multi-hop from native routes.
const MULTI_HOP_SOURCE = `${SOURCE} (multi-hop)`;

function main() {
	fs.mkdirSync(OUT_DIR, { recursive: true });

	let routeCount = 0;
	let totalCases = 0;

	// 1. Native routes (existing) — one file per conversions.js edge.
	for (const [from, targets] of Object.entries(ROUTES)) {
		const inputs = dedupe(INPUT_BUILDERS[from]()).sort(compareInputs);
		for (const to of targets) {
			const res = emitRouteFile(from, to, inputs, SOURCE);
			routeCount += res.count;
			totalCases += res.cases;
		}
	}

	// 2. Multi-hop (non-native) routes — exercising the public wrapper's BFS.
	for (const [from, to] of MULTI_HOP_ROUTES) {
		const inputs = dedupe(nativeInputs(from)).sort(compareInputs);
		const res = emitRouteFile(from, to, inputs, MULTI_HOP_SOURCE);
		routeCount += res.count;
		totalCases += res.cases;
	}

	process.stdout.write(`Wrote ${routeCount} routes, ${totalCases} total cases to ${OUT_DIR}\n`);
}

main();
