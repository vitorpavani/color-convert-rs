//! Conversions FROM the `hsl` colour model into other colour spaces
//! — ported from `convert.hsl.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! All numeric routes return **raw (unrounded) floats**. The per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility. Tolerance is 0.0 after per-channel rounding;
//! see the vector tests in `tests/hsl_routes.rs`.

/// Computes a single RGB channel value (raw, unrounded) given the
/// hue-offset parameters, mirroring one iteration of the JS loop body
/// in `convert.hsl.rgb`.
///
/// The `offset` is `+1/3`, `0`, or `-1/3` for the R, G, and B channels
/// respectively, corresponding to `h + 1/3 * -(i - 1)` for `i ∈ {0,1,2}`.
#[inline]
fn channel(h: f64, t1: f64, t2: f64, offset: f64) -> f64 {
    let mut t3 = h + offset;

    if t3 < 0.0 {
        t3 += 1.0;
    }
    if t3 > 1.0 {
        t3 -= 1.0;
    }

    let val = if 6.0 * t3 < 1.0 {
        t1 + (t2 - t1) * 6.0 * t3
    } else if 2.0 * t3 < 1.0 {
        t2
    } else if 3.0 * t3 < 2.0 {
        t1 + (t2 - t1) * (2.0 / 3.0 - t3) * 6.0
    } else {
        t1
    };

    val * 255.0
}

/// Converts an HSL triple to raw RGB floats `[r (0-255), g (0-255), b (0-255)]`.
///
/// Faithful port of `convert.hsl.rgb` (color-convert@3.1.3 conversions.js,
/// lines 304–345). The achromatic case uses a direct `== 0.0` comparison;
/// the saturation is `⟨integer⟩ / 100.0`, so exact equality is well-defined
/// and matches the JS `s === 0` control flow.
pub fn rgb(hsl: [f64; 3]) -> [f64; 3] {
    let h = hsl[0] / 360.0;
    let s = hsl[1] / 100.0;
    let l = hsl[2] / 100.0;

    // Achromatic case — saturation is exactly zero
    if s == 0.0 {
        let val = l * 255.0;
        return [val, val, val];
    }

    let t2 = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let t1 = 2.0 * l - t2;

    // Hue offsets for R, G, B channels (JS loop i=0,1,2 → +1/3, 0, -1/3)
    let r = channel(h, t1, t2, 1.0 / 3.0);
    let g = channel(h, t1, t2, 0.0);
    let b = channel(h, t1, t2, -1.0 / 3.0);

    [r, g, b]
}
