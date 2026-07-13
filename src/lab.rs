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
