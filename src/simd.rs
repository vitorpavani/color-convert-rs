//! CPU-SIMD batch conversion routes for hot matrix-heavy paths.
//!
//! Uses the [`wide`] crate for portable explicit SIMD (f32x8 lanes) to
//! process 8 pixels at once. The matrix multiply and linear combination
//! parts are SIMD-accelerated; piecewise nonlinear transforms (sRGB gamma,
//! LAB cube-root transfer) extract individual lanes, call the scalar
//! reference functions, and re-pack.
//!
//! ## Routes covered
//!
//! * `rgb→xyz` — sRGB inverse gamma + sRGB→XYZ (D65) matrix
//! * `xyz→lab` — D65 white-point normalization + CIE L*a*b* transfer + linear mix
//! * `rgb→lab` — fused single-pass rgb→xyz→lab (no intermediate XYZ buffer)
//!
//! ## Tolerance
//!
//! Each SIMD lane performs the same sequence of `f32` operations as the
//! scalar `f64` route would on the same pixel. f32 has ~7 decimal digits
//! of precision vs f64's ~15, so outputs differ by a small epsilon:
//!
//! * `rgb→xyz`: absolute tolerance ≤ 5e-4 per channel
//! * `xyz→lab`: absolute tolerance ≤ 1e-3 per channel
//! * `rgb→lab` (fused): inherits both tolerances above; additionally, the
//!   fused pass must match the two-step chain `xyz→lab(rgb→xyz(…))` within
//!   `f32::EPSILON × 10` since both paths perform identical f32 arithmetic
//!
//! These tolerances are wide enough to accept the f32/f64 gap but narrow
//! enough to catch real bugs (wrong coefficient, wrong branch condition).
//! See `tests/simd_routes.rs`.
//!
//! ## Batch API
//!
//! Batch functions accept slices of pixel triples and return `Vec<[f32;3]>`,
//! processing 8 pixels at a time via `wide::f32x8` with scalar remainder
//! fallback for the final 0–7 pixels.

/// sRGB inverse nonlinear transform — f32 version.
#[inline]
fn srgb_inv_f32(c: f32) -> f32 {
    if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }
}

/// CIE LAB transfer function — f32 version.
#[inline]
fn lab_transfer_f32(t: f32) -> f32 {
    let ft = (6.0_f32 / 29.0).powi(3);
    if t > ft {
        t.cbrt()
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

/// Process a batch of RGB pixels into XYZ via sRGB inverse gamma + matrix.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes for the matrix
/// multiply; extracts lanes for the scalar piecewise gamma function and
/// re-packs. Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::xyz`], converting its f64 output to f32.
pub fn rgb_to_xyz_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    while i + 7 < n {
        let r = f32x8::new([
            rgb[i][0] as f32,
            rgb[i + 1][0] as f32,
            rgb[i + 2][0] as f32,
            rgb[i + 3][0] as f32,
            rgb[i + 4][0] as f32,
            rgb[i + 5][0] as f32,
            rgb[i + 6][0] as f32,
            rgb[i + 7][0] as f32,
        ]);
        let g = f32x8::new([
            rgb[i][1] as f32,
            rgb[i + 1][1] as f32,
            rgb[i + 2][1] as f32,
            rgb[i + 3][1] as f32,
            rgb[i + 4][1] as f32,
            rgb[i + 5][1] as f32,
            rgb[i + 6][1] as f32,
            rgb[i + 7][1] as f32,
        ]);
        let b = f32x8::new([
            rgb[i][2] as f32,
            rgb[i + 1][2] as f32,
            rgb[i + 2][2] as f32,
            rgb[i + 3][2] as f32,
            rgb[i + 4][2] as f32,
            rgb[i + 5][2] as f32,
            rgb[i + 6][2] as f32,
            rgb[i + 7][2] as f32,
        ]);

        let r_norm = r / f32x8::splat(255.0);
        let g_norm = g / f32x8::splat(255.0);
        let b_norm = b / f32x8::splat(255.0);

        let r_arr = r_norm.to_array();
        let g_arr = g_norm.to_array();
        let b_arr = b_norm.to_array();
        let r_lin = f32x8::new([
            srgb_inv_f32(r_arr[0]),
            srgb_inv_f32(r_arr[1]),
            srgb_inv_f32(r_arr[2]),
            srgb_inv_f32(r_arr[3]),
            srgb_inv_f32(r_arr[4]),
            srgb_inv_f32(r_arr[5]),
            srgb_inv_f32(r_arr[6]),
            srgb_inv_f32(r_arr[7]),
        ]);
        let g_lin = f32x8::new([
            srgb_inv_f32(g_arr[0]),
            srgb_inv_f32(g_arr[1]),
            srgb_inv_f32(g_arr[2]),
            srgb_inv_f32(g_arr[3]),
            srgb_inv_f32(g_arr[4]),
            srgb_inv_f32(g_arr[5]),
            srgb_inv_f32(g_arr[6]),
            srgb_inv_f32(g_arr[7]),
        ]);
        let b_lin = f32x8::new([
            srgb_inv_f32(b_arr[0]),
            srgb_inv_f32(b_arr[1]),
            srgb_inv_f32(b_arr[2]),
            srgb_inv_f32(b_arr[3]),
            srgb_inv_f32(b_arr[4]),
            srgb_inv_f32(b_arr[5]),
            srgb_inv_f32(b_arr[6]),
            srgb_inv_f32(b_arr[7]),
        ]);

        let x = r_lin * f32x8::splat(0.4124564)
            + g_lin * f32x8::splat(0.3575761)
            + b_lin * f32x8::splat(0.1804375);
        let y = r_lin * f32x8::splat(0.2126729)
            + g_lin * f32x8::splat(0.7151522)
            + b_lin * f32x8::splat(0.0721750);
        let z = r_lin * f32x8::splat(0.0193339)
            + g_lin * f32x8::splat(0.119_192)
            + b_lin * f32x8::splat(0.9503041);

        let x_arr = x.to_array();
        let y_arr = y.to_array();
        let z_arr = z.to_array();

        for j in 0..8 {
            result.push([x_arr[j] * 100.0, y_arr[j] * 100.0, z_arr[j] * 100.0]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::xyz(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}

/// Process a batch of XYZ pixels into CIE L*a*b* via D65 normalization + transfer.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes for the linear
/// combination (L, a, b formulas); extracts lanes for the scalar piecewise
/// LAB transfer function and re-packs. Remainder pixels fall back to the
/// scalar [`crate::xyz::lab`], converting its f64 output to f32.
pub fn xyz_to_lab_batch(xyz: &[[f32; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = xyz.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let xn = f32x8::splat(95.047);
    let yn = f32x8::splat(100.0);
    let zn = f32x8::splat(108.883);

    while i + 7 < n {
        let x = f32x8::new([
            xyz[i][0],
            xyz[i + 1][0],
            xyz[i + 2][0],
            xyz[i + 3][0],
            xyz[i + 4][0],
            xyz[i + 5][0],
            xyz[i + 6][0],
            xyz[i + 7][0],
        ]);
        let y = f32x8::new([
            xyz[i][1],
            xyz[i + 1][1],
            xyz[i + 2][1],
            xyz[i + 3][1],
            xyz[i + 4][1],
            xyz[i + 5][1],
            xyz[i + 6][1],
            xyz[i + 7][1],
        ]);
        let z = f32x8::new([
            xyz[i][2],
            xyz[i + 1][2],
            xyz[i + 2][2],
            xyz[i + 3][2],
            xyz[i + 4][2],
            xyz[i + 5][2],
            xyz[i + 6][2],
            xyz[i + 7][2],
        ]);

        let x_norm = x / xn;
        let y_norm = y / yn;
        let z_norm = z / zn;

        let x_arr = x_norm.to_array();
        let y_arr = y_norm.to_array();
        let z_arr = z_norm.to_array();
        let fx = f32x8::new([
            lab_transfer_f32(x_arr[0]),
            lab_transfer_f32(x_arr[1]),
            lab_transfer_f32(x_arr[2]),
            lab_transfer_f32(x_arr[3]),
            lab_transfer_f32(x_arr[4]),
            lab_transfer_f32(x_arr[5]),
            lab_transfer_f32(x_arr[6]),
            lab_transfer_f32(x_arr[7]),
        ]);
        let fy = f32x8::new([
            lab_transfer_f32(y_arr[0]),
            lab_transfer_f32(y_arr[1]),
            lab_transfer_f32(y_arr[2]),
            lab_transfer_f32(y_arr[3]),
            lab_transfer_f32(y_arr[4]),
            lab_transfer_f32(y_arr[5]),
            lab_transfer_f32(y_arr[6]),
            lab_transfer_f32(y_arr[7]),
        ]);
        let fz = f32x8::new([
            lab_transfer_f32(z_arr[0]),
            lab_transfer_f32(z_arr[1]),
            lab_transfer_f32(z_arr[2]),
            lab_transfer_f32(z_arr[3]),
            lab_transfer_f32(z_arr[4]),
            lab_transfer_f32(z_arr[5]),
            lab_transfer_f32(z_arr[6]),
            lab_transfer_f32(z_arr[7]),
        ]);

        let l = fy * f32x8::splat(116.0) - f32x8::splat(16.0);
        let a = (fx - fy) * f32x8::splat(500.0);
        let b = (fy - fz) * f32x8::splat(200.0);

        let l_arr = l.to_array();
        let a_arr = a.to_array();
        let b_arr = b.to_array();

        for j in 0..8 {
            result.push([l_arr[j], a_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_input = [xyz[i][0] as f64, xyz[i][1] as f64, xyz[i][2] as f64];
        let f64_result = crate::xyz::lab(f64_input);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}

/// Process a batch of RGB pixels into CIE L*a*b* in a single SIMD pass —
/// no intermediate XYZ Vec allocation.
///
/// Fuses the sRGB inverse gamma, RGB→XYZ matrix, D65 white-point
/// normalization, and CIE L*a*b* transfer into one pipeline per block of
/// 8 pixels via `f32x8`.  Remainder pixels fall back to the same
/// f64-scalar chain used by the two-step path for bit-identical output.
pub fn rgb_to_lab_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    // D65 white-point reference values (XYZ scale, 0–100)
    let xn = f32x8::splat(95.047);
    let yn = f32x8::splat(100.0);
    let zn = f32x8::splat(108.883);

    while i + 7 < n {
        let r = f32x8::new([
            rgb[i][0] as f32,
            rgb[i + 1][0] as f32,
            rgb[i + 2][0] as f32,
            rgb[i + 3][0] as f32,
            rgb[i + 4][0] as f32,
            rgb[i + 5][0] as f32,
            rgb[i + 6][0] as f32,
            rgb[i + 7][0] as f32,
        ]);
        let g = f32x8::new([
            rgb[i][1] as f32,
            rgb[i + 1][1] as f32,
            rgb[i + 2][1] as f32,
            rgb[i + 3][1] as f32,
            rgb[i + 4][1] as f32,
            rgb[i + 5][1] as f32,
            rgb[i + 6][1] as f32,
            rgb[i + 7][1] as f32,
        ]);
        let b = f32x8::new([
            rgb[i][2] as f32,
            rgb[i + 1][2] as f32,
            rgb[i + 2][2] as f32,
            rgb[i + 3][2] as f32,
            rgb[i + 4][2] as f32,
            rgb[i + 5][2] as f32,
            rgb[i + 6][2] as f32,
            rgb[i + 7][2] as f32,
        ]);

        let r_norm = r / f32x8::splat(255.0);
        let g_norm = g / f32x8::splat(255.0);
        let b_norm = b / f32x8::splat(255.0);

        let r_arr = r_norm.to_array();
        let g_arr = g_norm.to_array();
        let b_arr = b_norm.to_array();
        let r_lin = f32x8::new([
            srgb_inv_f32(r_arr[0]),
            srgb_inv_f32(r_arr[1]),
            srgb_inv_f32(r_arr[2]),
            srgb_inv_f32(r_arr[3]),
            srgb_inv_f32(r_arr[4]),
            srgb_inv_f32(r_arr[5]),
            srgb_inv_f32(r_arr[6]),
            srgb_inv_f32(r_arr[7]),
        ]);
        let g_lin = f32x8::new([
            srgb_inv_f32(g_arr[0]),
            srgb_inv_f32(g_arr[1]),
            srgb_inv_f32(g_arr[2]),
            srgb_inv_f32(g_arr[3]),
            srgb_inv_f32(g_arr[4]),
            srgb_inv_f32(g_arr[5]),
            srgb_inv_f32(g_arr[6]),
            srgb_inv_f32(g_arr[7]),
        ]);
        let b_lin = f32x8::new([
            srgb_inv_f32(b_arr[0]),
            srgb_inv_f32(b_arr[1]),
            srgb_inv_f32(b_arr[2]),
            srgb_inv_f32(b_arr[3]),
            srgb_inv_f32(b_arr[4]),
            srgb_inv_f32(b_arr[5]),
            srgb_inv_f32(b_arr[6]),
            srgb_inv_f32(b_arr[7]),
        ]);

        let x = r_lin * f32x8::splat(0.4124564)
            + g_lin * f32x8::splat(0.3575761)
            + b_lin * f32x8::splat(0.1804375);
        let y = r_lin * f32x8::splat(0.2126729)
            + g_lin * f32x8::splat(0.7151522)
            + b_lin * f32x8::splat(0.0721750);
        let z = r_lin * f32x8::splat(0.0193339)
            + g_lin * f32x8::splat(0.119_192)
            + b_lin * f32x8::splat(0.9503041);

        // Scale to 0–100 (matches rgb_to_xyz_batch convention)
        let x = x * f32x8::splat(100.0);
        let y = y * f32x8::splat(100.0);
        let z = z * f32x8::splat(100.0);

        let x_norm = x / xn;
        let y_norm = y / yn;
        let z_norm = z / zn;

        let x_arr = x_norm.to_array();
        let y_arr = y_norm.to_array();
        let z_arr = z_norm.to_array();
        let fx = f32x8::new([
            lab_transfer_f32(x_arr[0]),
            lab_transfer_f32(x_arr[1]),
            lab_transfer_f32(x_arr[2]),
            lab_transfer_f32(x_arr[3]),
            lab_transfer_f32(x_arr[4]),
            lab_transfer_f32(x_arr[5]),
            lab_transfer_f32(x_arr[6]),
            lab_transfer_f32(x_arr[7]),
        ]);
        let fy = f32x8::new([
            lab_transfer_f32(y_arr[0]),
            lab_transfer_f32(y_arr[1]),
            lab_transfer_f32(y_arr[2]),
            lab_transfer_f32(y_arr[3]),
            lab_transfer_f32(y_arr[4]),
            lab_transfer_f32(y_arr[5]),
            lab_transfer_f32(y_arr[6]),
            lab_transfer_f32(y_arr[7]),
        ]);
        let fz = f32x8::new([
            lab_transfer_f32(z_arr[0]),
            lab_transfer_f32(z_arr[1]),
            lab_transfer_f32(z_arr[2]),
            lab_transfer_f32(z_arr[3]),
            lab_transfer_f32(z_arr[4]),
            lab_transfer_f32(z_arr[5]),
            lab_transfer_f32(z_arr[6]),
            lab_transfer_f32(z_arr[7]),
        ]);

        let l = fy * f32x8::splat(116.0) - f32x8::splat(16.0);
        let a = (fx - fy) * f32x8::splat(500.0);
        let b = (fy - fz) * f32x8::splat(200.0);

        let l_arr = l.to_array();
        let a_arr = a.to_array();
        let b_arr = b.to_array();

        for j in 0..8 {
            result.push([l_arr[j], a_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — replicate the two-step chain's f64→f32→f64→f32
    // round-trip to guarantee bit-identical output with the two-step path.
    while i < n {
        let f64_xyz = crate::rgb::xyz(rgb[i]);
        let f32_xyz: [f32; 3] = [f64_xyz[0] as f32, f64_xyz[1] as f32, f64_xyz[2] as f32];
        let f64_input: [f64; 3] = [f32_xyz[0] as f64, f32_xyz[1] as f64, f32_xyz[2] as f64];
        let f64_lab = crate::xyz::lab(f64_input);
        result.push([f64_lab[0] as f32, f64_lab[1] as f32, f64_lab[2] as f32]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use wide::f32x8;

    /// Behavior: `srgb_inv_f32x8` must match the scalar `srgb_inv_f32` for
    /// representative values across all 8 SIMD lanes.
    ///
    /// Inputs span the piecewise threshold (0.04045), dark values, bright
    /// values, and the boundary itself.  Tolerance: f32::EPSILON * 1000
    /// (generous enough to absorb any subnormal or micro-architectural
    /// quirk while catching real algorithmic divergence).
    #[test]
    fn srgb_inv_f32x8_matches_scalar() {
        const TOL: f32 = f32::EPSILON * 1000.0;
        // Representative inputs across the piecewise boundary
        let inputs = [0.0_f32, 0.01, 0.04045, 0.04046, 0.5, 0.75, 1.0, 2.0];
        let v = f32x8::new(inputs);
        // This call will FAIL TO COMPILE until the function exists:
        let result = srgb_inv_f32x8(v).to_array();

        for i in 0..8 {
            let want = srgb_inv_f32(inputs[i]);
            let diff = (result[i] - want).abs();
            assert!(
                diff <= TOL,
                "lane {i}: srgb_inv_f32x8({})={}, scalar={}, diff={:.2e} > tol",
                inputs[i], result[i], want, diff,
            );
        }
    }

    /// Behavior: `lab_transfer_f32x8` must match the scalar `lab_transfer_f32`
    /// for representative values across all 8 SIMD lanes.
    ///
    /// Inputs span the CIE LAB piecewise threshold `ft = (6/29)³ ≈ 0.008856`,
    /// plus dark and bright values. Tolerance: f32::EPSILON * 1000.
    #[test]
    fn lab_transfer_f32x8_matches_scalar() {
        const TOL: f32 = f32::EPSILON * 1000.0;
        let ft = (6.0_f32 / 29.0).powi(3); // ≈ 0.008856
        let inputs = [0.0_f32, 0.001, ft * 0.5, ft, ft * 1.01, 0.1, 0.5, 1.0];
        let v = f32x8::new(inputs);
        // This call will FAIL TO COMPILE until the function exists:
        let result = lab_transfer_f32x8(v).to_array();

        for i in 0..8 {
            let want = lab_transfer_f32(inputs[i]);
            let diff = (result[i] - want).abs();
            assert!(
                diff <= TOL,
                "lane {i}: lab_transfer_f32x8({})={}, scalar={}, diff={:.2e} > tol",
                inputs[i], result[i], want, diff,
            );
        }
    }
}
