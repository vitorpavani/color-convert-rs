//! Conversions FROM the `cmyk` colour model into other colour spaces
//! — ported from `convert.cmyk.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/cmyk_routes.rs`.

/// Converts a CMYK quadruple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.cmyk.rgb` (color-convert@3.1.3 conversions.js,
/// lines 475–486). Each channel is divided by 100, then the formula
/// `channel = 1 - min(1, cmyk * (1 - k) + k)` derives the RGB complement,
/// multiplied by 255 for the 0-255 range.
pub fn rgb(cmyk: [f64; 4]) -> [f64; 3] {
    let c = cmyk[0] / 100.0;
    let m = cmyk[1] / 100.0;
    let y = cmyk[2] / 100.0;
    let k = cmyk[3] / 100.0;
    let r = 1.0 - (c * (1.0 - k) + k).min(1.0);
    let g = 1.0 - (m * (1.0 - k) + k).min(1.0);
    let b = 1.0 - (y * (1.0 - k) + k).min(1.0);
    [r * 255.0, g * 255.0, b * 255.0]
}
