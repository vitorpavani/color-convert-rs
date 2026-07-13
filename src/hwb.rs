//! Conversions FROM the `hwb` colour model into other colour spaces
//! — ported from `convert.hwb.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/hwb_routes.rs`.

/// Converts an HWB triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.hwb.rgb` (color-convert@3.1.3 conversions.js,
/// lines 421–473). The `_ => (v, n, wh)` catch-all mirrors the JS
/// `default: case 6: case 0:` block — all three arms produce the same
/// `[v, n, wh]` assignment. The `i as i64` cast is safe because
/// `i = floor(6h)` with `h ∈ [0, 1]`, so `i ∈ {0, 1, 2, 3, 4, 5, 6}`.
pub fn rgb(hwb: [f64; 3]) -> [f64; 3] {
    let h = hwb[0] / 360.0;
    let mut wh = hwb[1] / 100.0;
    let mut bl = hwb[2] / 100.0;
    let ratio = wh + bl;

    // wh + bl cant be > 1
    if ratio > 1.0 {
        wh /= ratio;
        bl /= ratio;
    }

    let i = (6.0 * h).floor();
    let v = 1.0 - bl;
    let mut f = 6.0 * h - i;
    let i_int = i as i64;

    // Parity check: (i & 0x01) !== 0 in JS
    if (i_int & 0x01) != 0 {
        f = 1.0 - f;
    }

    let n = wh + f * (v - wh); // linear interpolation

    let (r, g, b) = match i_int {
        1 => (n, v, wh),
        2 => (wh, v, n),
        3 => (wh, n, v),
        4 => (n, wh, v),
        5 => (v, wh, n),
        // JS default: case 6: case 0: → [v, n, wh]
        _ => (v, n, wh),
    };

    [r * 255.0, g * 255.0, b * 255.0]
}
