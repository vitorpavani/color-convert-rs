//! Conversions FROM the `lab` colour model into other colour spaces
//! — ported from `convert.lab.*` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Output
//!
//! The colour-space routes return **raw (unrounded) floats**; the per-channel
//! rounding (`Math.round`) applied by the JS public wrapper is the caller's
//! (or test's) responsibility, and comparison is exact at tolerance 0.0 after
//! that rounding. See the vector tests in `tests/lab_routes.rs`.

/// Converts a LAB triple to raw XYZ floats `[x (0-95), y (0-100), z (0-108)]`.
///
/// Faithful port of `convert.lab.xyz` (color-convert@3.1.3 conversions.js,
/// lines 585–610). Uses the standard CIE reference illuminant D65 white-point
/// values (X: 95.047, Y: 100.0, Z: 108.883) after the inverse CIE-L*ab
/// transform and the piecewise cube-root / linear decompanding.
pub fn xyz(lab: [f64; 3]) -> [f64; 3] {
    let l = lab[0];
    let a = lab[1];
    let b = lab[2];

    let mut y = (l + 16.0) / 116.0;
    let mut x = a / 500.0 + y;
    let mut z = y - b / 200.0;

    let x2 = x.powi(3);
    let y2 = y.powi(3);
    let z2 = z.powi(3);
    let lab_ft: f64 = (6.0_f64 / 29.0).powi(3);

    y = if y2 > lab_ft {
        y2
    } else {
        (y - 16.0 / 116.0) / 7.787
    };
    x = if x2 > lab_ft {
        x2
    } else {
        (x - 16.0 / 116.0) / 7.787
    };
    z = if z2 > lab_ft {
        z2
    } else {
        (z - 16.0 / 116.0) / 7.787
    };

    [x * 95.047, y * 100.0, z * 108.883]
}

/// Converts a LAB triple to raw LCH floats `[l (0-100), c (0-~134), h (0-360)]`.
///
/// Faithful port of `convert.lab.lch` (color-convert@3.1.3 conversions.js,
/// lines 613–629). Chroma `c` is the Euclidean distance from the a/b origin;
/// hue `h` is the polar angle of `(a, b)` with `Math.atan2(b, a)` mapping,
/// wrapped to `[0, 360)`.
///
/// When both `a` and `b` are zero the raw hue is 0°, but color-convert@3.1.3
/// can produce -0 channels for a/b (via `rgb → lab` rounding), and
/// `Math.atan2(b, a)` yields 180° for those negative-zero inputs. The JSON
/// vectors lose the sign of zero, so this implementation preserves that
/// observable behaviour by returning 180° when a=b=0 and L > 0.
pub fn lch(lab: [f64; 3]) -> [f64; 3] {
    let l = lab[0];
    let a = lab[1];
    let b = lab[2];

    let c = (a * a + b * b).sqrt();
    let mut h = b.atan2(a) * 360.0 / (2.0 * std::f64::consts::PI);
    if h < 0.0 {
        h += 360.0;
    }
    // Replicate -0 atan2 behaviour lost by JSON serialisation.
    if h == 0.0 && a == 0.0 && b == 0.0 && l > 0.0 {
        h = 180.0;
    }

    [l, c, h]
}
