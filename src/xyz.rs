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

/// The LAB forward transfer function `f(t)` from CIE 1976 L*a*b*:
/// `t > Δ³ ? ∛t : κ t + 16/116` where `Δ = 6/29` and `κ = 7.787`.
///
/// Mirrors the inline triplicate in `convert.xyz.lab` (conversions.js
/// lines 517–519). Kept private — only the public conversion functions
/// consume it.
pub(crate) fn lab_transfer(t: f64) -> f64 {
    let delta_cubed = (6.0_f64 / 29.0).powi(3);
    if t > delta_cubed {
        t.cbrt()
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

/// Converts an XYZ triple to raw CIE L*a*b* floats `[l (0-100), a, b]`.
///
/// Faithful port of `convert.xyz.lab` (color-convert@3.1.3 conversions.js,
/// lines 508–526). Uses D65 reference white-point constants `(95.047, 100,
/// 108.883)` and the CIE forward transfer function.
pub fn lab(xyz: [f64; 3]) -> [f64; 3] {
    let x = lab_transfer(xyz[0] / 95.047);
    let y = lab_transfer(xyz[1] / 100.0);
    let z = lab_transfer(xyz[2] / 108.883);

    let l = 116.0 * y - 16.0;
    let a = 500.0 * (x - y);
    let b = 200.0 * (y - z);

    [l, a, b]
}

/// Converts an XYZ triple to raw OkLab floats `[l (0-100), a, b]`.
///
/// Faithful port of `convert.xyz.oklab` (color-convert@3.1.3 conversions.js,
/// lines 528–542). Normalises XYZ by 100, applies the OkLab forward matrix,
/// takes the cube root of each linear response, then mixes through the
/// perceptual matrix — returning each channel scaled by 100.
pub fn oklab(xyz: [f64; 3]) -> [f64; 3] {
    let x = xyz[0] / 100.0;
    let y = xyz[1] / 100.0;
    let z = xyz[2] / 100.0;

    let lp = (0.8189330101 * x + 0.3618667424 * y - 0.1288597137 * z).cbrt();
    let mp = (0.0329845436 * x + 0.9293118715 * y + 0.0361456387 * z).cbrt();
    let sp = (0.0482003018 * x + 0.2643662691 * y + 0.633851707 * z).cbrt();

    let l = 0.2104542553 * lp + 0.793617785 * mp - 0.0040720468 * sp;
    let a = 1.9779984951 * lp - 2.428592205 * mp + 0.4505937099 * sp;
    let b = 0.0259040371 * lp + 0.7827717662 * mp - 0.808675766 * sp;

    [l * 100.0, a * 100.0, b * 100.0]
}
