//! Apple RGB source routes.
//!
//! Converts from Apple's calibrated RGB space (16-bit channels, 0..=65535)
//! to other color spaces.
//!
//! Reference: `convert.apple.*` in color-convert's `conversions.js` (lines 937–939).

/// Converts Apple RGB (0..=65535 per channel) to sRGB (0..=255 per channel as `f64`).
///
/// Mirror of `convert.apple.rgb`:
/// `(channel / 65535.0) * 255.0` per channel.
///
/// Tolerance: 0.0 after rounding (exact float arithmetic with no clamping needed).
pub fn rgb(apple: [f64; 3]) -> [f64; 3] {
    [
        (apple[0] / 65535.0) * 255.0,
        (apple[1] / 65535.0) * 255.0,
        (apple[2] / 65535.0) * 255.0,
    ]
}
