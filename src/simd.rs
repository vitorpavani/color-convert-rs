//! CPU-SIMD batch conversion routes for hot matrix-heavy paths.
//!
//! Uses the [`wide`] crate for portable explicit SIMD (f64x4 lanes) to
//! process 4 pixels at once. The matrix multiply and linear combination
//! parts are SIMD-accelerated; piecewise nonlinear transforms (sRGB gamma,
//! LAB cube-root transfer) extract individual lanes, call the scalar
//! reference functions, and re-pack — matching the scalar output exactly
//! because every `f64` lane is an independent IEEE 754 computation.
//!
//! ## Routes covered
//!
//! * `rgb→xyz` — sRGB inverse gamma + sRGB→XYZ (D65) matrix
//! * `xyz→lab` — D65 white-point normalization + CIE L*a*b* transfer + linear mix
//!
//! ## Tolerance
//!
//! Each SIMD lane performs the same sequence of `f64` operations as the
//! scalar route on the same pixel, so outputs must be **bit-identical** to
//! calling the scalar function (tolerance 0.0). Documented here for
//! clarity: if a test ever observes a nonzero diff, that is a bug.
//!
//! ## Batch API
//!
//! Batch functions accept slices of pixel triples and return `Vec<[f64;3]>`,
//! processing 4 pixels at a time via `wide::f64x4` with scalar remainder
//! fallback for the final 0–3 pixels.

/// Process a batch of RGB pixels into XYZ via sRGB inverse gamma + matrix.
///
/// Processes 4 pixels at a time using `f64x4` SIMD lanes for the matrix
/// multiply; extracts lanes for the scalar piecewise gamma function and
/// re-packs. Remainder pixels (final 0–3) fall back to the scalar
/// [`crate::rgb::xyz`].
///
/// # Panics
///
/// Does not panic — every `[u8;3]` is a valid RGB triple.
pub fn rgb_to_xyz_batch(_rgb: &[[u8; 3]]) -> Vec<[f64; 3]> {
    unimplemented!("SIMD rgb→xyz batch — GREEN phase")
}
