//! Conversions FROM the `oklch` colour model into other colour spaces
//! — ported from `convert.oklch.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/oklch_routes.rs`.

/// Converts an Oklch triple to raw Oklab floats `[l (0-100), a, b]`.
///
/// Faithful port of `convert.oklch.oklab` (color-convert@3.1.3 conversions.js,
/// line 581–583), which delegates to `convert.lch.lab` (lines 631–641).
pub fn oklab(oklch: [f64; 3]) -> [f64; 3] {
    let l = oklch[0];
    let c = oklch[1];
    let h = oklch[2];

    let hr = h / 360.0 * 2.0 * std::f64::consts::PI;
    let a = c * hr.cos();
    let b = c * hr.sin();

    [l, a, b]
}
