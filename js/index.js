'use strict';

const wasm = require('./color_convert_rs.js');

const MODELS = [
  'rgb', 'hsl', 'hsv', 'hwb', 'cmyk', 'xyz', 'lab', 'lch',
  'oklab', 'oklch', 'hex', 'keyword', 'ansi16', 'ansi256',
  'hcg', 'apple', 'gray',
];

const CHANNELS = {
  rgb: 3, hsl: 3, hsv: 3, hwb: 3, cmyk: 4, xyz: 3, lab: 3, lch: 3,
  oklab: 3, oklch: 3, hex: 1, keyword: 1, ansi16: 1, ansi256: 1,
  hcg: 3, apple: 3, gray: 1,
};

const LABELS = {
  rgb: ['r', 'g', 'b'],
  hsl: ['h', 's', 'l'],
  hsv: ['h', 's', 'v'],
  hwb: ['h', 'w', 'b'],
  cmyk: ['c', 'm', 'y', 'k'],
  xyz: ['x', 'y', 'z'],
  lab: ['l', 'a', 'b'],
  lch: ['l', 'c', 'h'],
  oklab: ['okl', 'oka', 'okb'],
  oklch: ['okl', 'okc', 'okh'],
  hex: ['hex'],
  keyword: ['keyword'],
  ansi16: ['ansi16'],
  ansi256: ['ansi256'],
  hcg: ['h', 'c', 'g'],
  apple: ['r16', 'g16', 'b16'],
  gray: ['gray'],
};

function normalizeArgs(args) {
  const arg0 = args[0];
  if (arg0 === undefined || arg0 === null) {
    return arg0;
  }
  if (arg0.length > 1) {
    return arg0;
  }
  return args;
}

function makeRouteFn(from, to) {
  const fn = function (...args) {
    const input = normalizeArgs(args);
    if (input === undefined || input === null) {
      return input;
    }
    return wasm.convert_route(from, to, input);
  };
  fn.raw = function (...args) {
    const input = normalizeArgs(args);
    if (input === undefined || input === null) {
      return input;
    }
    return wasm.convert_route_raw(from, to, input);
  };
  return fn;
}

const BATCH_FNS = {
  'rgb.hsl':   wasm.rgb_to_hsl_batch,
  'rgb.hsv':   wasm.rgb_to_hsv_batch,
  'rgb.cmyk':  wasm.rgb_to_cmyk_batch,
  'rgb.lab':   wasm.rgb_to_lab_batch,
  'rgb.xyz':   wasm.rgb_to_xyz_batch,
  'rgb.oklab': wasm.rgb_to_oklab_batch,
  'hsl.rgb':   wasm.hsl_to_rgb_batch,
  'hsv.rgb':   wasm.hsv_to_rgb_batch,
};

function makeModel(from) {
  const model = {};
  for (const to of MODELS) {
    if (from === to) {
      continue;
    }
    const route = makeRouteFn(from, to);
    const batchKey = `${from}.${to}`;
    if (BATCH_FNS[batchKey]) {
      route.batch = BATCH_FNS[batchKey];
    }
    model[to] = route;
  }
  Object.defineProperty(model, 'channels', { value: CHANNELS[from] });
  Object.defineProperty(model, 'labels', { value: LABELS[from] });
  return model;
}

const convert = {};
for (const from of MODELS) {
  convert[from] = makeModel(from);
}

module.exports = convert;
module.exports.default = convert;
