//! Conversions FROM the `hsv` colour model into other colour spaces
//! — ported from `convert.hsv.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! All numeric routes return **raw (unrounded) floats**. The per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility. Tolerance is 0.0 after per-channel rounding;
//! see the vector tests in `tests/hsv_routes.rs`.

/// Converts an HSV triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.hsv.rgb` (color-convert@3.1.3 conversions.js,
/// lines 363–401). The hue sector `hi` is computed via `rem_euclid(6)` rather
/// than `%` so the result is always in `0..=5` even if the input hue were to
/// wrap past 360 (the JS reference floor-divides by 60 then applies `% 6`).
/// The `_` match arm is statically unreachable after `rem_euclid(6)`, but the
/// compiler requires exhaustive coverage; it maps to the `hi == 0` case.
pub fn rgb(hsv: [f64; 3]) -> [f64; 3] {
    let h = hsv[0] / 60.0;
    let s = hsv[1] / 100.0;
    let mut v = hsv[2] / 100.0;

    // hi ∈ 0..=5 after rem_euclid(6) — matches JS `Math.floor(h) % 6`
    let hi = (h.floor() as i64).rem_euclid(6);

    let f = h - h.floor();
    let p = 255.0 * v * (1.0 - s);
    let q = 255.0 * v * (1.0 - s * f);
    let t = 255.0 * v * (1.0 - s * (1.0 - f));
    v *= 255.0;

    match hi {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        // Statically unreachable (hi ∈ 0..=5); matches hi==0 as the safe fallback.
        _ => [v, t, p],
    }
}
