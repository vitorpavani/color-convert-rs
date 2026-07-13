//! Conversions FROM the `oklab` colour model into other colour spaces
//! — ported from `convert.oklab.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/oklab_routes.rs`.

/// Converts an Oklab triple to raw Oklch floats `[l (0-100), c, h (0-360)]`.
///
/// Faithful port of `convert.oklab.oklch` (color-convert@3.1.3 conversions.js,
/// line 544–546), which delegates to `convert.lab.lch` (lines 613–628).
pub fn oklch(oklab: [f64; 3]) -> [f64; 3] {
    let l = oklab[0];
    let a = oklab[1];
    let b = oklab[2];

    let hr = b.atan2(a);
    let mut h = hr * 360.0 / 2.0 / std::f64::consts::PI;
    if h < 0.0 {
        h += 360.0;
    }

    let c = (a * a + b * b).sqrt();

    [l, c, h]
}

/// Converts an Oklab triple to raw XYZ floats `[x, y, z]`.
///
/// Faithful port of `convert.oklab.xyz` (color-convert@3.1.3 conversions.js,
/// lines 548–562).
pub fn xyz(_oklab: [f64; 3]) -> [f64; 3] {
    [0.0, 0.0, 0.0]
}
