//! Conversions FROM the `rgb` color model.
//!
//! Mirrors `convert.rgb.*` in color-convert@3.1.3 `conversions.js`. Each
//! function returns RAW (unrounded) floats — the observable per-channel
//! rounding applied by the JS public wrapper (`Math.round`) is the caller's
//! (or test's) responsibility. Tolerance is documented per route in the
//! vector tests (currently 0.0 after rounding for rgb→hsl).

/// Converts an RGB triple to raw HSL floats `[h (0-360), s (0-100), l (0-100)]`.
///
/// Faithful port of `convert.rgb.hsl` (color-convert@3.1.3 conversions.js).
/// Channel comparisons use direct `==` exactly as the JS does; the compared
/// values are exact `/255.0` divisions of the same inputs, so equality is
/// well-defined and matches the JS control flow bit-for-bit.
pub fn hsl(rgb: [u8; 3]) -> [f64; 3] {
    let r = f64::from(rgb[0]) / 255.0;
    let g = f64::from(rgb[1]) / 255.0;
    let b = f64::from(rgb[2]) / 255.0;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;

    let mut h = if max == min {
        0.0
    } else if r == max {
        (g - b) / delta
    } else if g == max {
        2.0 + (b - r) / delta
    } else {
        // b == max (last arm of the JS if/else chain)
        4.0 + (r - g) / delta
    };

    h = (h * 60.0).min(360.0);
    if h < 0.0 {
        h += 360.0;
    }

    let l = (min + max) / 2.0;

    let s = if max == min {
        0.0
    } else if l <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    [h, s * 100.0, l * 100.0]
}
