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
pub fn rgb(code: u16) -> [f64; 3] {
    let args = i64::from(code);

    // greyscale ramp (codes 232–255)
    if args >= 232 {
        let c = ((args - 232) * 10 + 8) as f64;
        return [c, c, c];
    }

    // colour cube (codes 16–231) and system colours (0–15, producing negatives).
    // f64::floor mirrors JS Math.floor — needed for negative values
    // (Rust integer division truncates toward zero, not toward −∞).
    let v = args as f64 - 16.0;
    let r = (v / 36.0).floor() / 5.0 * 255.0;
    let rem = v % 36.0; // f64 remainder preserves dividend sign (matches JS %)
    let g = (rem / 6.0).floor() / 5.0 * 255.0;
    let b = (rem % 6.0) / 5.0 * 255.0;
    [r, g, b]
}
