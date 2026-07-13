//! ANSI‑256 colour-code decoder — ported from `convert.ansi256.rgb` in
//! color-convert@3.1.3 `conversions.js` (lines 727–745).
//!
//! ## Input
//!
//! An ANSI‑256 colour code in `[0..255]`.
//!
//! ## Output
//!
//! Returns **raw (unrounded) floats** `[r, g, b]`. Values may be **negative**
//! for codes 0–15 (the "system" colours). Per-channel rounding is the
//! caller's responsibility.
//!
//! ## Tolerance
//!
//! Outputs are integer-valued after rounding — comparison is exact at
//! tolerance 0.0.

/// Converts an ANSI‑256 colour code to raw RGB floats.
///
/// Behaviour mirrors `convert.ansi256.rgb` (color-convert@3.1.3):
/// - Codes ≥ 232 produce a greyscale ramp (`c = (code-232)*10 + 8`).
/// - Codes 16–231 are mapped to a 6×6×6 colour cube with three
///   floor‑divided channel indices on the `code-16` value.
/// - Codes 0–15 produce mathematically correct but visually‑meaningless
///   **negative** RGB values (faithful to the JS reference).
pub fn rgb(_code: u16) -> [f64; 3] {
    // STUB — returns a clearly wrong value so RED tests fail on assertion.
    [42.0, 42.0, 42.0]
}
