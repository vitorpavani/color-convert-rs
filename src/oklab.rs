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
    [oklab[0], oklab[1], oklab[2]]
}
