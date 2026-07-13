//! Grayscale (single-channel) source routes.
//!
//! Converts from grayscale intensity (0..=100) to other color spaces.
//!
//! Reference: `convert.gray.*` in color-convert's `conversions.js` (lines 945–975).

/// Converts grayscale to sRGB.
///
/// Mirror of `convert.gray.rgb` (lines 945–947):
/// `v = gray[0] / 100.0 * 255.0; return [v, v, v]`
pub fn rgb(gray: [f64; 1]) -> [f64; 3] {
    let v = (gray[0] / 100.0) * 255.0;
    [v, v, v]
}

/// Converts grayscale to HSL.
///
/// Mirror of `convert.gray.hsl` (lines 949–951):
/// `return [0, 0, args[0]]`
pub fn hsl(gray: [f64; 1]) -> [f64; 3] {
    [0.0, 0.0, gray[0]]
}

/// Converts grayscale to HSV.
///
/// Mirror of `convert.gray.hsv` — aliased to `gray.hsl` in the JS reference:
/// `return [0, 0, args[0]]`
pub fn hsv(gray: [f64; 1]) -> [f64; 3] {
    [0.0, 0.0, gray[0]]
}

/// Converts grayscale to HWB.
///
/// Mirror of `convert.gray.hwb` (lines 955–957):
/// `return [0, 100, gray[0]]`
pub fn hwb(gray: [f64; 1]) -> [f64; 3] {
    [0.0, 100.0, gray[0]]
}

/// Converts grayscale to CMYK.
///
/// Mirror of `convert.gray.cmyk` (lines 959–961):
/// `return [0, 0, 0, gray[0]]`
pub fn cmyk(gray: [f64; 1]) -> [f64; 4] {
    [0.0, 0.0, 0.0, gray[0]]
}
