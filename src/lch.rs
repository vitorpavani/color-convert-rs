//! Conversions FROM the `lch` colour model into other colour spaces
//! — ported from `convert.lch.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/lch_routes.rs`.

/// Converts an LCH triple to raw LAB floats `[l (0-100), a (~-128..127), b (~-128..127)]`.
///
/// Faithful port of `convert.lch.lab` (color-convert@3.1.3 conversions.js,
/// lines 631–642). Converts polar (chroma, hue) back to Cartesian (a, b)
/// coordinates via `a = c·cos(hr)`, `b = c·sin(hr)` where `hr` is the hue
/// in radians.
pub fn lab(lch: [f64; 3]) -> [f64; 3] {
    let l = lch[0];
    let c = lch[1];
    let h = lch[2];

    let hr = h / 360.0 * 2.0 * std::f64::consts::PI;
    let a = c * hr.cos();
    let b = c * hr.sin();

    [l, a, b]
}
