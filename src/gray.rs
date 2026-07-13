//! Grayscale (single-channel) source routes.
//!
//! Converts from grayscale intensity (0..=100) to other color spaces.
//!
//! Reference: `convert.gray.*` in color-convert's `conversions.js` (lines 945–975).

/// Converts grayscale to sRGB.
///
/// Mirror of `convert.gray.rgb` (lines 945–947):
/// `v = gray[0] / 100.0 * 255.0; return [v, v, v]`
pub fn rgb(gray: [f64; 1]) -> [f64; 3] {
    let v = (gray[0] / 100.0) * 255.0;
    [v, v, v]
}
