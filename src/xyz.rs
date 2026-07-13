//! Conversions FROM the `xyz` colour model into other colour spaces
//! — ported from `convert.xyz.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/xyz_routes.rs`.
//!
//! ## SRGB non-linear transform
//!
//! The `srgb_nonlinear_transform` helper mirrors `srgbNonlinearTransform`
//! (conversions.js line 40–45): a piecewise gamma function that clamps to
//! \[0, 1\]. It is private — only the conversion functions are public.

/// Converts an XYZ triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.xyz.rgb` (color-convert@3.1.3 conversions.js,
/// lines 488–506). Applies the standard sRGB matrix followed by the
/// non-linear sRGB transfer function and a final multiplication by 255.
pub fn rgb(xyz: [f64; 3]) -> [f64; 3] {
    let x = xyz[0] / 100.0;
    let y = xyz[1] / 100.0;
    let z = xyz[2] / 100.0;

    let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
    let g = x * -0.969266 + y * 1.8760108 + z * 0.041556;
    let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

    // Apply sRGB non-linear transform and scale to 0–255.
    let r = srgb_nonlinear_transform(r) * 255.0;
    let g = srgb_nonlinear_transform(g) * 255.0;
    let b = srgb_nonlinear_transform(b) * 255.0;

    [r, g, b]
}

/// sRGB non-linear transfer function (see module doc).
fn srgb_nonlinear_transform(c: f64) -> f64 {
    let cc = if c > 0.0031308 {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    } else {
        c * 12.92
    };
    cc.clamp(0.0, 1.0)
}
