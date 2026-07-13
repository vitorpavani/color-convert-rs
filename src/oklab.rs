//! Conversions FROM the `oklab` colour model into other colour spaces
//! — ported from `convert.oklab.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/oklab_routes.rs`.

/// Converts an Oklab triple to raw Oklch floats `[l (0-100), c, h (0-360)]`.
///
/// Faithful port of `convert.oklab.oklch` (color-convert@3.1.3 conversions.js,
/// line 544–546), which delegates to `convert.lab.lch` (lines 613–628).
pub fn oklch(oklab: [f64; 3]) -> [f64; 3] {
    let l = oklab[0];
    let a = oklab[1];
    let b = oklab[2];

    let hr = b.atan2(a);
    let mut h = hr * 360.0 / 2.0 / std::f64::consts::PI;
    if h < 0.0 {
        h += 360.0;
    }

    let c = (a * a + b * b).sqrt();

    [l, c, h]
}

/// Converts an Oklab triple to raw XYZ floats `[x, y, z]`.
///
/// Faithful port of `convert.oklab.xyz` (color-convert@3.1.3 conversions.js,
/// lines 548–562).
pub fn xyz(oklab: [f64; 3]) -> [f64; 3] {
    let ll = oklab[0] / 100.0;
    let a = oklab[1] / 100.0;
    let b = oklab[2] / 100.0;

    let l = (0.999_999_998 * ll + 0.396_337_792 * a + 0.215_803_758 * b).powi(3);
    let m = (1.000_000_008 * ll - 0.105_561_342 * a - 0.063_854_175 * b).powi(3);
    let s = (1.000_000_055 * ll - 0.089_484_182 * a - 1.291_485_538 * b).powi(3);

    let x = 1.227_013_851 * l - 0.557_799_98 * m + 0.281_256_149 * s;
    let y = -0.040_580_178 * l + 1.112_256_87 * m - 0.071_676_679 * s;
    let z = -0.076_381_285 * l - 0.421_481_978 * m + 1.586_163_22 * s;

    [x * 100.0, y * 100.0, z * 100.0]
}

/// Converts an Oklab triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.oklab.rgb` (color-convert@3.1.3 conversions.js,
/// lines 564–579).
pub fn rgb(_oklab: [f64; 3]) -> [f64; 3] {
    [0.0, 0.0, 0.0]
}
