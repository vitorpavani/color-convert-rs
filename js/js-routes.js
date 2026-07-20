'use strict';

const LAB_FT = (6 / 29) ** 3;

function srgbInv(c) {
  return c > 0.04045 ? ((c + 0.055) / 1.055) ** 2.4 : c / 12.92;
}

function _rgbHslRaw(r, g, b) {
  r /= 255; g /= 255; b /= 255;
  const min = Math.min(r, g, b);
  const max = Math.max(r, g, b);
  const delta = max - min;
  let h;
  if (max === min) h = 0;
  else if (max === r) h = (g - b) / delta;
  else if (max === g) h = 2 + (b - r) / delta;
  else h = 4 + (r - g) / delta;
  h = Math.min(h * 60, 360);
  if (h < 0) h += 360;
  const l = (min + max) / 2;
  const s = max === min ? 0 : l <= 0.5 ? delta / (max + min) : delta / (2 - max - min);
  return [h, s * 100, l * 100];
}

function rgbHsl(r, g, b) {
  const [h, s, l] = _rgbHslRaw(r, g, b);
  return [Math.round(h), Math.round(s), Math.round(l)];
}

function rgbHsv(r, g, b) {
  r /= 255; g /= 255; b /= 255;
  const v = Math.max(r, g, b);
  const diff = v - Math.min(r, g, b);
  let h, s;
  if (diff === 0) { h = 0; s = 0; }
  else {
    s = diff / v;
    const rdif = (v - r) / 6 / diff + 0.5;
    const gdif = (v - g) / 6 / diff + 0.5;
    const bdif = (v - b) / 6 / diff + 0.5;
    if (v === r) h = bdif - gdif;
    else if (v === g) h = 1 / 3 + rdif - bdif;
    else h = 2 / 3 + gdif - rdif;
    if (h < 0) h += 1;
    else if (h > 1) h -= 1;
  }
  return [Math.round(h * 360), Math.round(s * 100), Math.round(v * 100)];
}

function rgbHwb(r, g, b) {
  const hsl = _rgbHslRaw(r, g, b);
  const w = 1 / 255 * Math.min(r, Math.min(g, b));
  const bk = 1 - 1 / 255 * Math.max(r, Math.max(g, b));
  return [Math.round(hsl[0]), Math.round(w * 100), Math.round(bk * 100)];
}

function rgbCmyk(r, g, b) {
  r /= 255; g /= 255; b /= 255;
  const k = Math.min(1 - r, 1 - g, 1 - b);
  const c = (1 - r - k) / (1 - k) || 0;
  const m = (1 - g - k) / (1 - k) || 0;
  const y = (1 - b - k) / (1 - k) || 0;
  return [Math.round(c * 100), Math.round(m * 100), Math.round(y * 100), Math.round(k * 100)];
}

function _rgbXyzRaw(r, g, b) {
  r = srgbInv(r / 255); g = srgbInv(g / 255); b = srgbInv(b / 255);
  return [
    (r * 0.4124564 + g * 0.3575761 + b * 0.1804375) * 100,
    (r * 0.2126729 + g * 0.7151522 + b * 0.072175) * 100,
    (r * 0.0193339 + g * 0.119192 + b * 0.9503041) * 100,
  ];
}

function rgbXyz(r, g, b) {
  const [x, y, z] = _rgbXyzRaw(r, g, b);
  return [Math.round(x), Math.round(y), Math.round(z)];
}

function rgbLab(r, g, b) {
  const xyz = _rgbXyzRaw(r, g, b);
  let x = xyz[0] / 95.047;
  let y = xyz[1] / 100;
  let z = xyz[2] / 108.883;
  x = x > LAB_FT ? x ** (1 / 3) : 7.787 * x + 16 / 116;
  y = y > LAB_FT ? y ** (1 / 3) : 7.787 * y + 16 / 116;
  z = z > LAB_FT ? z ** (1 / 3) : 7.787 * z + 16 / 116;
  const l = 116 * y - 16;
  const a = 500 * (x - y);
  const bb = 200 * (y - z);
  return [Math.round(l), Math.round(a), Math.round(bb)];
}

function rgbOklab(r, g, b) {
  r = srgbInv(r / 255); g = srgbInv(g / 255); b = srgbInv(b / 255);
  const lp = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const mp = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const sp = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const l = 0.2104542553 * lp + 0.793617785 * mp - 0.0040720468 * sp;
  const a = 1.9779984951 * lp - 2.428592205 * mp + 0.4505937099 * sp;
  const bb = 0.0259040371 * lp + 0.7827717662 * mp - 0.808675766 * sp;
  return [Math.round(l * 100), Math.round(a * 100), Math.round(bb * 100)];
}

function hslRgb(h, s, l) {
  h /= 360; s /= 100; l /= 100;
  if (s === 0) { const v = Math.round(l * 255); return [v, v, v]; }
  const t2 = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const t1 = 2 * l - t2;
  const rgb = [0, 0, 0];
  for (let i = 0; i < 3; i++) {
    let t3 = h + 1 / 3 * -(i - 1);
    if (t3 < 0) t3++;
    if (t3 > 1) t3--;
    let v;
    if (6 * t3 < 1) v = t1 + (t2 - t1) * 6 * t3;
    else if (2 * t3 < 1) v = t2;
    else if (3 * t3 < 2) v = t1 + (t2 - t1) * (2 / 3 - t3) * 6;
    else v = t1;
    rgb[i] = Math.round(v * 255);
  }
  return rgb;
}

function hsvRgb(h, s, v) {
  h /= 60; s /= 100; v /= 100;
  const hi = Math.floor(h) % 6;
  const f = h - Math.floor(h);
  const p = Math.round(255 * v * (1 - s));
  const q = Math.round(255 * v * (1 - s * f));
  const t = Math.round(255 * v * (1 - s * (1 - f)));
  const vv = Math.round(v * 255);
  switch (hi) {
    case 0: return [vv, t, p];
    case 1: return [q, vv, p];
    case 2: return [p, vv, t];
    case 3: return [p, q, vv];
    case 4: return [t, p, vv];
    case 5: return [vv, p, q];
  }
}

module.exports = {
  rgbHsl, rgbHsv, rgbHwb, rgbCmyk, rgbXyz, rgbLab, rgbOklab,
  hslRgb, hsvRgb,
};
