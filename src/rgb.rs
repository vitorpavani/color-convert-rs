//! Conversions FROM the `rgb` color model.
//!
//! Mirrors `convert.rgb.*` in color-convert@3.1.3 `conversions.js`. Each
//! function returns RAW (unrounded) floats — the observable per-channel
//! rounding applied by the JS public wrapper (`Math.round`) is the caller's
//! (or test's) responsibility. Tolerance is documented per route in the
//! vector tests (currently 0.0 after rounding for rgb→hsl).

/// Normalize an RGB `[u8; 3]` input to per-channel `f64` fractions in `[0.0, 1.0]`,
/// returning the three channel values along with their min, max, and delta (max-min).
#[inline]
fn normalize_rgb(rgb: [u8; 3]) -> (f64, f64, f64, f64, f64, f64) {
    let r = f64::from(rgb[0]) / 255.0;
    let g = f64::from(rgb[1]) / 255.0;
    let b = f64::from(rgb[2]) / 255.0;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;
    (r, g, b, min, max, delta)
}

/// Converts an RGB triple to raw HSL floats `[h (0-360), s (0-100), l (0-100)]`.
///
/// Faithful port of `convert.rgb.hsl` (color-convert@3.1.3 conversions.js).
/// Channel comparisons use direct `==` exactly as the JS does; the compared
/// values are exact `/255.0` divisions of the same inputs, so equality is
/// well-defined and matches the JS control flow bit-for-bit.
pub fn hsl(rgb: [u8; 3]) -> [f64; 3] {
    let (r, g, b, min, max, delta) = normalize_rgb(rgb);

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

/// Converts an RGB triple to raw HSV floats `[h (0-360), s (0-100), v (0-100)]`.
///
/// Faithful port of `convert.rgb.hsv` (color-convert@3.1.3 conversions.js,
/// lines 128-186). The JS `switch (v)` matches channels in r, g, b order via
/// strict equality; the compared values are exact `/255.0` divisions of the
/// same inputs, so direct `==` reproduces that control flow bit-for-bit.
pub fn hsv(rgb: [u8; 3]) -> [f64; 3] {
    let (r, g, b, _min, v, diff) = normalize_rgb(rgb);

    let (h, s) = if diff == 0.0 {
        (0.0, 0.0)
    } else {
        let diffc = |c: f64| (v - c) / 6.0 / diff + 0.5;
        let rdif = diffc(r);
        let gdif = diffc(g);
        let bdif = diffc(b);

        let mut h = if v == r {
            bdif - gdif
        } else if v == g {
            1.0 / 3.0 + rdif - bdif
        } else {
            // v == b (last case of the JS switch; no default arm exists)
            2.0 / 3.0 + gdif - rdif
        };

        if h < 0.0 {
            h += 1.0;
        } else if h > 1.0 {
            h -= 1.0;
        }

        (h, diff / v)
    };

    [h * 360.0, s * 100.0, v * 100.0]
}

/// Converts an RGB triple to raw HWB floats `[h (0-360), w (0-100), b (0-100)]`.
///
/// Faithful port of `convert.rgb.hwb` (color-convert@3.1.3 conversions.js,
/// lines 188-198). The hue is derived from `hsl(rgb)[0]`, while whiteness and
/// blackness are computed from the min and max of the normalized channel
/// fractions.
pub fn hwb(rgb: [u8; 3]) -> [f64; 3] {
    let (_r, _g, _b, min, max, _delta) = normalize_rgb(rgb);
    let h = hsl(rgb)[0];
    [h, min * 100.0, (1.0 - max) * 100.0]
}

/// Converts an RGB triple to raw CMYK floats `[c (0-100), m (0-100), y (0-100), k (0-100)]`.
///
/// Faithful port of `convert.rgb.cmyk` (color-convert@3.1.3 conversions.js,
/// lines 217-228). The divide-by-zero guard when `k == 1` (pure black) mirrors
/// the JS `|| 0` fallback: the expression `(1-r-k)/(1-k) || 0` evaluates the
/// division result, and if falsy (0 or NaN) falls through to 0.
pub fn cmyk(rgb: [u8; 3]) -> [f64; 4] {
    let r = f64::from(rgb[0]) / 255.0;
    let g = f64::from(rgb[1]) / 255.0;
    let b = f64::from(rgb[2]) / 255.0;

    let k = (1.0 - r).min(1.0 - g).min(1.0 - b);
    let denom = 1.0 - k;

    let (c, m, y) = if denom == 0.0 {
        // k == 1 (pure black) — guard division by zero, mirroring JS `|| 0`
        (0.0, 0.0, 0.0)
    } else {
        (
            (1.0 - r - k) / denom,
            (1.0 - g - k) / denom,
            (1.0 - b - k) / denom,
        )
    };

    [c * 100.0, m * 100.0, y * 100.0, k * 100.0]
}
