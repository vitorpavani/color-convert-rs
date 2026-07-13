//! Conversions FROM the `hcg` colour model into other colour spaces
//! — ported from `convert.hcg.*` in color-convert@3.1.3 `conversions.js`.
//!
//! The HCG model (hue, chroma, gray) represents a colour as:
//!
//! - **hue** (0-360°) — the colour angle on the wheel.
//! - **chroma** (0-100) — purity/saturation relative to the maximum the
//!   lightness can support.
//! - **gray** (0-100) — the gray component that mixes with the pure hue.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/hcg_routes.rs`.

/// Converts an HCG triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.hcg.rgb` (color-convert@3.1.3 conversions.js,
/// lines 834–884). The `hi.floor() as i64` cast is safe because
/// `hi = (h%1)*6` with `h ∈ [0, 1]`, so `hi ∈ [0, 6)`.
pub fn rgb(hcg: [f64; 3]) -> [f64; 3] {
    let h = hcg[0] / 360.0;
    let c = hcg[1] / 100.0;
    let g = hcg[2] / 100.0;

    if c == 0.0 {
        return [g * 255.0, g * 255.0, g * 255.0];
    }

    let hi = (h % 1.0) * 6.0;
    let v = hi % 1.0;
    let w = 1.0 - v;

    let pure = match hi.floor() as i64 {
        0 => [1.0, v, 0.0],
        1 => [w, 1.0, 0.0],
        2 => [0.0, 1.0, v],
        3 => [0.0, w, 1.0],
        4 => [v, 0.0, 1.0],
        // JS default: case 5 → [1, 0, w]
        _ => [1.0, 0.0, w],
    };

    let mg = (1.0 - c) * g;

    [
        (c * pure[0] + mg) * 255.0,
        (c * pure[1] + mg) * 255.0,
        (c * pure[2] + mg) * 255.0,
    ]
}

/// Converts an HCG triple to raw HSV floats `[h (0-360), s (0-100), v (0-100)]`.
///
/// Faithful port of `convert.hcg.hsv` (color-convert@3.1.3 conversions.js,
/// lines 886–898).
pub fn hsv(hcg: [f64; 3]) -> [f64; 3] {
    let c = hcg[1] / 100.0;
    let g = hcg[2] / 100.0;
    let v = c + g * (1.0 - c);
    let mut f = 0.0;
    if v > 0.0 {
        f = c / v;
    }
    [hcg[0], f * 100.0, v * 100.0]
}

/// Converts an HCG triple to raw HSL floats `[h (0-360), s (0-100), l (0-100)]`.
///
/// Faithful port of `convert.hcg.hsl` (color-convert@3.1.3 conversions.js,
/// lines 900–914).
pub fn hsl(_hcg: [f64; 3]) -> [f64; 3] {
    todo!()
}

/// Converts an HCG triple to raw HWB floats `[h (0-360), w (0-100), b (0-100)]`.
///
/// Faithful port of `convert.hcg.hwb` (color-convert@3.1.3 conversions.js,
/// lines 916–921).
pub fn hwb(_hcg: [f64; 3]) -> [f64; 3] {
    todo!()
}
