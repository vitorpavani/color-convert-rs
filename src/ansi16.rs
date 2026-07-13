//! ANSI‑16 colour-code decoder — ported from `convert.ansi16.rgb` in
//! color-convert@3.1.3 `conversions.js` (lines 701–725).
//!
//! ## Input
//!
//! An ANSI‑16 colour code in `[30..37]`, `[40..47]`, `[90..97]`, or
//! `[100..107]`. Codes outside these ranges are handled by the same
//! maths — the behaviour is deterministic but not meaningful.
//!
//! ## Output
//!
//! Returns **raw (unrounded) floats** `[r (0-255), g (0-255), b (0-255)]`.
//! Per-channel rounding is the caller's responsibility.
//!
//! ## Tolerance
//!
//! Outputs are integer-valued after rounding — comparison is exact at
//! tolerance 0.0.

/// Converts an ANSI‑16 colour code to raw RGB floats.
///
/// Behaviour mirrors `convert.ansi16.rgb` (color-convert@3.1.3):
/// - Greyscale codes (color 0 or 7) produce a single luminance repeated
///   across all three channels, brightened (+3.5) when `code > 50`.
/// - Chromatic codes use the low 3 bits of the colour index as RGB flags,
///   scaled by a multiplier (0.5 for regular, 1.0 for bright).
pub fn rgb(code: u16) -> [f64; 3] {
    let args = f64::from(code);
    let color = args % 10.0;

    // handle greyscale
    if color == 0.0 || color == 7.0 {
        let mut c = color;
        if args > 50.0 {
            c += 3.5; // bright
        }
        c = c / 10.5 * 255.0;
        return [c, c, c];
    }

    // chromatic: low 3 bits of `color` are RGB flags
    let mult = (if args > 50.0 { 1.0 } else { 0.0 } + 1.0) * 0.5;
    let ci = color as i64; // JS bitwise: `args % 10` coerces to int
    let r = ((ci & 1) as f64 * mult) * 255.0;
    let g = (((ci >> 1) & 1) as f64 * mult) * 255.0;
    let b = (((ci >> 2) & 1) as f64 * mult) * 255.0;
    [r, g, b]
}
