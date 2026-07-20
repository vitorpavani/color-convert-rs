//! CPU-SIMD batch conversion for the `xyz→rgb` (inverse) route.
//!
//! Processes 8 pixels at a time via `wide::f32x8` lanes for the 3×3
//! sRGB-forward matrix multiply AND the piecewise forward sRGB gamma
//! (`powf(1.0/2.4)`, vectorized via mask-blend). Remainder pixels
//! fall back to the scalar [`crate::xyz::rgb`], converting its f64
//! output to f32.
//!
//! ## Reference
//!
//! Faithful to `convert.xyz.rgb` in color-convert@3.1.3 `conversions.js`
//! lines 488–506 via the scalar `xyz::rgb` port. Tolerance vs scalar
//! f64: 0.1 absolute per channel (output range [0, 255]), capturing the
//! f32/f64 floating-point gap through the matrix + gamma + ×255 chain.

/// Forward sRGB nonlinear transform — vectorized across 8 f32 lanes via mask-blend.
///
/// The piecewise `if c > 0.0031308` is replaced with a SIMD mask-blend:
/// both branches are computed for all 8 lanes, then the correct one is
/// selected via `mask.blend(pow_branch, linear_branch)`. Uses
/// `f32x8::powf(1.0/2.4)` which delegates to the LLVM-generated vector
/// intrinsic.
#[inline]
fn srgb_fwd_f32x8(c: wide::f32x8) -> wide::f32x8 {
    let pow_branch = c.powf(1.0 / 2.4) * wide::f32x8::splat(1.055) - wide::f32x8::splat(0.055);
    let linear_branch = c * wide::f32x8::splat(12.92);
    let mask = c.simd_gt(wide::f32x8::splat(0.0031308));
    mask.blend(pow_branch, linear_branch)
}

/// Public wrapper: auto-selects serial or parallel based on input size.
///
/// Delegates to [`crate::simd_parallel::auto_batch`] which chooses serial
/// SIMD for ≤ 4096 pixels and multi-core rayon for larger batches.
///
/// ## Output
///
/// Returns raw `[f32;3]` floats on [0, 255] — the same shape as the
/// scalar `xyz::rgb` which also returns unrounded floats.
pub fn xyz_to_rgb_batch(xyz: &[[f32; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(xyz, xyz_to_rgb_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
///
/// Processes 8 pixels at a time using f32x8 SIMD lanes for the matrix
/// multiply (3×3 linear combination with 9 coefficients) AND the forward
/// sRGB gamma (piecewise `powf(1.0/2.4)` via mask-blend). Remainder
/// pixels (final 0–7) fall back to the scalar [`crate::xyz::rgb`],
/// converting its f64 output to f32.
pub(crate) fn xyz_to_rgb_batch_serial(xyz: &[[f32; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = xyz.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

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

        let x_norm = x / f32x8::splat(100.0);
        let y_norm = y / f32x8::splat(100.0);
        let z_norm = z / f32x8::splat(100.0);

        // sRGB-forward (XYZ→sRGB) matrix, D65 white point
        let r = x_norm * f32x8::splat(3.2404542)
            + y_norm * f32x8::splat(-1.5371385)
            + z_norm * f32x8::splat(-0.4985314);
        let g = x_norm * f32x8::splat(-0.969266)
            + y_norm * f32x8::splat(1.8760108)
            + z_norm * f32x8::splat(0.041556);
        let b = x_norm * f32x8::splat(0.0556434)
            + y_norm * f32x8::splat(-0.2040259)
            + z_norm * f32x8::splat(1.0572252);

        // Apply forward sRGB non-linear transform and scale to 0–255
        let r = srgb_fwd_f32x8(r) * f32x8::splat(255.0);
        let g = srgb_fwd_f32x8(g) * f32x8::splat(255.0);
        let b = srgb_fwd_f32x8(b) * f32x8::splat(255.0);

        let r_arr = r.to_array();
        let g_arr = g.to_array();
        let b_arr = b.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_input = [xyz[i][0] as f64, xyz[i][1] as f64, xyz[i][2] as f64];
        let f64_result = crate::xyz::rgb(f64_input);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
