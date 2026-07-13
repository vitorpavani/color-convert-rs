//! Hex colour-string decoder — ported from `convert.hex.rgb` in
//! color-convert@3.1.3 `conversions.js` (lines 757–777).
//!
//! ## Input
//!
//! Accepts a hex string (e.g. `"000000"`, `"8CC864"`, `"ABC"`). A leading
//! `#` is tolerated but not required. The scanner finds the *first* run of
//! 6 (or, failing that, 3) hex digits, case-insensitive — mirroring the JS
//! regex `/^(?:[a-f\d]{6}|[a-f\d]{3})/i` ("longest-leftmost" semantics).
//!
//! ## Output
//!
//! Returns **raw (unrounded) floats** `[r (0-255), g (0-255), b (0-255)]`.
//! Per-channel rounding is the caller's responsibility. If no hex run is
//! found the function returns `[0.0, 0.0, 0.0]`, matching `color-convert`'s
//! fallback.
//!
//! ## Tolerance
//!
//! Outputs are integer-valued after rounding — comparison is exact at
//! tolerance 0.0.

/// Converts a hex colour string to raw RGB floats.
///
/// Behaviour mirrors `convert.hex.rgb` (color-convert@3.1.3):
/// 1. Scan for the first run of 6 hex digits (e.g. `"8CC864"`).
/// 2. If none found, scan for the first run of 3 hex digits (e.g. `"ABC"`),
///    doubling each character to `"AABBCC"`.
/// 3. Parse the run as base‑16 and extract the red, green, and blue bytes.
/// 4. If no hex run is found, return `[0.0, 0.0, 0.0]`.
pub fn rgb(_hex: &str) -> [f64; 3] {
    // STUB — returns a clearly wrong value so RED tests fail on assertion.
    [42.0, 42.0, 42.0]
}
