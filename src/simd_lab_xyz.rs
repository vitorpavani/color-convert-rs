//! CPU-SIMD batch conversion for the `lab→xyz` (inverse) route.
//!
//! Processes 8 pixels at a time via `wide::f32x8` lanes for the CIE
//! L*a*b* inverse transfer (piecewise `t³ > ε ? t³ : (t − 16/116) / 7.787`,
//! vectorized via mask-blend) and the final D65 white-point scale.
//! Remainder pixels fall back to the scalar [`crate::lab::xyz`],
//! converting its f64 output to f32.
//!
//! ## Reference
//!
//! Faithful to `convert.lab.xyz` in color-convert@3.1.3 `conversions.js`
//! lines 585–610 via the scalar `lab::xyz` port.  Tolerance vs scalar
//! f64: 1e-3 absolute per channel (XYZ output range [0, ~100]),
//! capturing the f32/f64 gap through three piecewise t³/linear branch
//! decisions and three white-point multiplies.

/// Inverse CIE L*a*b* transfer — vectorized across 8 f32 lanes via mask-blend.
///
/// The scalar piecewise `if t³ > ε` is replaced with a SIMD mask-blend:
/// both branches are computed for all 8 lanes, then the correct one is
/// selected via `mask.blend(true_val, false_val)`.  This is the inverse
/// of [`crate::simd::lab_transfer_f32x8`].
#[inline]
fn inv_lab_transfer_f32x8(t: wide::f32x8) -> wide::f32x8 {
    let eps = wide::f32x8::splat((6.0_f32 / 29.0).powi(3));
    let t3 = t * t * t;
    let linear_branch = (t - wide::f32x8::splat(16.0 / 116.0)) / wide::f32x8::splat(7.787);
    let mask = t3.simd_gt(eps);
    mask.blend(t3, linear_branch)
}

/// Process a batch of LAB pixels into CIE XYZ D65 via inverse lab transfer
/// and white-point scaling.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes for the linear
/// combination (fx, fy, fz formulas), the inverse CIE L*a*b* piecewise
/// transfer (vectorized via mask-blend), and the D65 white-point scale.
/// Remainder pixels fall back to the scalar [`crate::lab::xyz`],
/// converting its f64 output to f32.
///
/// ## Output
///
/// Returns raw `[f32; 3]` floats on approximately [0, 100] — the same
/// shape as the scalar `lab::xyz` which also returns unrounded f64 floats.
pub fn lab_to_xyz_batch(lab: &[[f32; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = lab.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    while i + 7 < n {
        let l = f32x8::new([
            lab[i][0],
            lab[i + 1][0],
            lab[i + 2][0],
            lab[i + 3][0],
            lab[i + 4][0],
            lab[i + 5][0],
            lab[i + 6][0],
            lab[i + 7][0],
        ]);
        let a = f32x8::new([
            lab[i][1],
            lab[i + 1][1],
            lab[i + 2][1],
            lab[i + 3][1],
            lab[i + 4][1],
            lab[i + 5][1],
            lab[i + 6][1],
            lab[i + 7][1],
        ]);
        let b = f32x8::new([
            lab[i][2],
            lab[i + 1][2],
            lab[i + 2][2],
            lab[i + 3][2],
            lab[i + 4][2],
            lab[i + 5][2],
            lab[i + 6][2],
            lab[i + 7][2],
        ]);

        let fy = (l + f32x8::splat(16.0)) / f32x8::splat(116.0);
        let fx = a / f32x8::splat(500.0) + fy;
        let fz = fy - b / f32x8::splat(200.0);

        let x = inv_lab_transfer_f32x8(fx) * f32x8::splat(95.047);
        let y = inv_lab_transfer_f32x8(fy) * f32x8::splat(100.0);
        let z = inv_lab_transfer_f32x8(fz) * f32x8::splat(108.883);

        let x_arr = x.to_array();
        let y_arr = y.to_array();
        let z_arr = z.to_array();

        for j in 0..8 {
            result.push([x_arr[j], y_arr[j], z_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_input = [lab[i][0] as f64, lab[i][1] as f64, lab[i][2] as f64];
        let f64_result = crate::lab::xyz(f64_input);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use wide::f32x8;

    /// Scalar reference for inverse LAB transfer function (test-only).
    fn inv_lab_transfer_f32(t: f32) -> f32 {
        let eps = (6.0_f32 / 29.0).powi(3);
        let t3 = t * t * t;
        if t3 > eps {
            t3
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    }

    /// Behavior: `inv_lab_transfer_f32x8` must match the scalar
    /// `inv_lab_transfer_f32` for representative values across all 8
    /// SIMD lanes.
    ///
    /// Inputs span the CIE inverse LAB piecewise threshold `ε = (6/29)³ ≈ 0.008856`
    /// (i.e., t ≈ cbrt(ε) ≈ 0.2069), plus dark and bright values.
    /// Tolerance: f32::EPSILON * 1000.
    #[test]
    fn inv_lab_transfer_f32x8_matches_scalar() {
        const TOL: f32 = f32::EPSILON * 1000.0;
        let eps = (6.0_f32 / 29.0).powi(3); // ≈ 0.008856
        let cbrt_eps = eps.cbrt(); // ≈ 0.2069 — threshold for t
        let inputs = [
            0.0_f32,
            0.05,
            cbrt_eps * 0.5,
            cbrt_eps,
            cbrt_eps * 1.01,
            0.5,
            0.8,
            1.0,
        ];
        let v = f32x8::new(inputs);
        let result = inv_lab_transfer_f32x8(v).to_array();

        for i in 0..8 {
            let want = inv_lab_transfer_f32(inputs[i]);
            let diff = (result[i] - want).abs();
            assert!(
                diff <= TOL,
                "lane {i}: inv_lab_transfer_f32x8({})={}, scalar={}, diff={:.2e} > tol",
                inputs[i],
                result[i],
                want,
                diff,
            );
        }
    }
}
