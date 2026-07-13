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

/// Converts an HSV triple to raw HSL floats `[h (0-360), s (0-100), l (0-100)]`.
///
/// Faithful port of `convert.hsv.hsl` (color-convert@3.1.3 conversions.js,
/// lines 402–419). The JS expression `sl = sl || 0` guards NaN (and the falsy
/// zero) — mirrored here with `sl.is_finite()` to catch both NaN and ±infinity.
pub fn hsl(hsv: [f64; 3]) -> [f64; 3] {
    let h = hsv[0];
    let s = hsv[1] / 100.0;
    let v = hsv[2] / 100.0;

    let vmin = v.max(0.01);
    let l = (2.0 - s) * v;
    let lmin = (2.0 - s) * vmin;

    let mut sl = s * vmin;
    sl /= if lmin <= 1.0 { lmin } else { 2.0 - lmin };
    // Mirror JS `sl = sl || 0` — catches NaN, and also ±inf for safety.
    let sl = if sl.is_finite() { sl } else { 0.0 };

    let l = l / 2.0;

    [h, sl * 100.0, l * 100.0]
}

/// Converts an HSV triple to raw HCG floats `[h (0-360), c (0-100), g (0-100)]`.
///
/// Faithful port of `convert.hsv.hcg` (color-convert@3.1.3 conversions.js,
/// lines 820–832). The grey component `g` is derived as `(v-c)/(1-c)` when
/// chroma `c < 1`; when `c == 1` (fully saturated), `g` is clamped to 0.
pub fn hcg(hsv: [f64; 3]) -> [f64; 3] {
    let s = hsv[1] / 100.0;
    let v = hsv[2] / 100.0;
    let c = s * v;
    let f = if c < 1.0 { (v - c) / (1.0 - c) } else { 0.0 };
    [hsv[0], c * 100.0, f * 100.0]
}

/// Converts an HSV triple to an ANSI-16 terminal colour code (30–37, 40–47,
/// 90–97, 100–107).
///
/// Faithful port of `convert.hsv.ansi16` (color-convert@3.1.3 conversions.js,
/// lines 667–671). The JS chains through `convert.rgb.ansi16` with the
/// original HSV value channel — not the value recomputed from the rgb
/// round-trip — so the brightness bucket matches `round(hsvV / 50)`.
///
/// The returned `u16` is an exact integer code; no rounding tolerance applies.
pub fn ansi16(hsv: [f64; 3]) -> u16 {
    let rgb_f = rgb(hsv);
    crate::rgb::ansi16_with_value(rgb_f, hsv[2])
}
