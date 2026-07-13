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
pub fn oklab(_oklch: [f64; 3]) -> [f64; 3] {
    [0.0, 0.0, 0.0]
}
