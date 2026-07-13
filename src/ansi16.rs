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
pub fn rgb(_code: u16) -> [f64; 3] {
    // STUB — returns a clearly wrong value so RED tests fail on assertion.
    [42.0, 42.0, 42.0]
}
