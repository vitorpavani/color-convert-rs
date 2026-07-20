'use strict';

const convert = require('./index.js');

console.log('color-convert-rs — drop-in replacement demo\n');

console.log('rgb.hsl(255, 128, 0):', convert.rgb.hsl(255, 128, 0));
console.log('rgb.hsv(255, 128, 0):', convert.rgb.hsv(255, 128, 0));
console.log('rgb.cmyk(128, 64, 32):', convert.rgb.cmyk(128, 64, 32));
console.log('rgb.hex(255, 128, 0):', convert.rgb.hex(255, 128, 0));
console.log('rgb.keyword(255, 0, 0):', convert.rgb.keyword(255, 0, 0));
console.log('rgb.ansi16(255, 0, 0):', convert.rgb.ansi16(255, 0, 0));
console.log('rgb.ansi256(255, 0, 0):', convert.rgb.ansi256(255, 0, 0));
console.log('rgb.gray(128, 64, 32):', convert.rgb.gray(128, 64, 32));
console.log('rgb.lab(255, 128, 0):', convert.rgb.lab(255, 128, 0));
console.log('rgb.xyz(255, 128, 0):', convert.rgb.xyz(255, 128, 0));
console.log('rgb.oklab(255, 128, 0):', convert.rgb.oklab(255, 128, 0));

console.log('\n— Reverse routes —');
console.log('hex.rgb("FF8000"):', convert.hex.rgb('FF8000'));
console.log('keyword.rgb("red"):', convert.keyword.rgb('red'));
console.log('hsl.rgb(30, 100, 50):', convert.hsl.rgb(30, 100, 50));

console.log('\n— Array input (color-convert compatible) —');
console.log('rgb.hsl([255, 128, 0]):', convert.rgb.hsl([255, 128, 0]));

console.log('\n— .raw variant (unrounded) —');
console.log('rgb.hsl.raw(255, 128, 0):', convert.rgb.hsl.raw(255, 128, 0));

console.log('\n— .channels and .labels —');
console.log('rgb.channels:', convert.rgb.channels);
console.log('rgb.labels:', convert.rgb.labels);
console.log('cmyk.channels:', convert.cmyk.channels);
console.log('cmyk.labels:', convert.cmyk.labels);
