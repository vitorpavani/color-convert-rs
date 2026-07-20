'use strict';

const path = require('path');
const native = require(path.join(__dirname, 'color_convert_rs.node'));
const js = require('./js-routes');

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

const STRING_MODELS = new Set(['hex', 'keyword']);
const NUMBER_MODELS = new Set(['ansi16', 'ansi256']);

const JS_ROUTES = {
  'rgb.hsl':   (i) => js.rgbHsl(i[0], i[1], i[2]),
  'rgb.hsv':   (i) => js.rgbHsv(i[0], i[1], i[2]),
  'rgb.hwb':   (i) => js.rgbHwb(i[0], i[1], i[2]),
  'rgb.cmyk':  (i) => js.rgbCmyk(i[0], i[1], i[2]),
  'rgb.xyz':   (i) => js.rgbXyz(i[0], i[1], i[2]),
  'rgb.lab':   (i) => js.rgbLab(i[0], i[1], i[2]),
  'rgb.oklab': (i) => js.rgbOklab(i[0], i[1], i[2]),
  'hsl.rgb':   (i) => js.hslRgb(i[0], i[1], i[2]),
  'hsv.rgb':   (i) => js.hsvRgb(i[0], i[1], i[2]),
};

const BATCH_FNS = {
  'rgb.hsl':   native.rgbToHslBatch,
  'rgb.hsv':   native.rgbToHsvBatch,
  'rgb.cmyk':  native.rgbToCmykBatch,
  'rgb.lab':   native.rgbToLabBatch,
  'rgb.xyz':   native.rgbToXyzBatch,
  'rgb.oklab': native.rgbToOklabBatch,
};

const INTO_FNS = {
  'rgb.hsl':   native.rgbHslInto,
  'rgb.hsv':   native.rgbHsvInto,
  'rgb.cmyk':  native.rgbCmykInto,
  'rgb.lab':   native.rgbLabInto,
  'rgb.xyz':   native.rgbXyzInto,
  'rgb.oklab': native.rgbOklabInto,
};

function normalizeArgs(args) {
  const arg0 = args[0];
  if (arg0 === undefined || arg0 === null) return arg0;
  if (Array.isArray(arg0)) return arg0;
  if (typeof arg0 === 'string' && arg0.length > 1) return arg0;
  return args;
}

function makeRouteFn(from, to) {
  const routeKey = `${from}.${to}`;
  const jsFn = JS_ROUTES[routeKey];
  const toIsString = STRING_MODELS.has(to);
  const fromIsString = STRING_MODELS.has(from);
  const toIsNumber = NUMBER_MODELS.has(to);

  let fn;
  if (jsFn) {
    fn = function (...args) {
      const a0 = args[0];
      if (a0 === undefined || a0 === null) return a0;
      if (Array.isArray(a0)) return jsFn(a0);
      return jsFn(args);
    };
  } else {
    fn = function (...args) {
      const input = normalizeArgs(args);
      if (input === undefined || input === null) return input;
      if (fromIsString) {
        if (toIsString) return native.convertFromStringToString(from, to, String(input));
        if (toIsNumber) return native.convertFromStringToNumber(from, to, String(input));
        return native.convertFromString(from, to, String(input));
      }
      if (toIsString) return native.convertToString(from, to, Array.from(input));
      if (toIsNumber) return native.convertToNumber(from, to, Array.from(input));
      return native.convertRoute(from, to, Array.from(input));
    };
  }

  fn.raw = function (...args) {
    const input = normalizeArgs(args);
    if (input === undefined || input === null) return input;
    return native.convertRouteRaw(from, to, Array.from(input));
  };

  if (BATCH_FNS[routeKey]) fn.batch = BATCH_FNS[routeKey];
  if (INTO_FNS[routeKey]) {
    const intoFn = INTO_FNS[routeKey];
    fn.into = function (output, r, g, b) {
      intoFn(r, g, b, output);
      return output;
    };
  }

  return fn;
}

function makeModel(from) {
  const model = {};
  for (const to of MODELS) {
    if (from === to) continue;
    model[to] = makeRouteFn(from, to);
  }
  Object.defineProperty(model, 'channels', { value: CHANNELS[from] });
  Object.defineProperty(model, 'labels', { value: LABELS[from] });
  return model;
}

const convert = {};
for (const from of MODELS) convert[from] = makeModel(from);

module.exports = convert;
module.exports.default = convert;
