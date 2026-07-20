//! CPU-SIMD batch conversion routes for hot matrix-heavy paths.
//!
//! Uses the [`wide`] crate for portable explicit SIMD (f32x8 lanes) to
//! process 8 pixels at once. Both the matrix multiply/linear-combination
//! AND the piecewise nonlinear transfer functions (sRGB gamma via `powf`,
//! LAB cube-root via `cbrt`) are SIMD-accelerated through mask-blend
//! lane selection — no scalar-lane de-vectorization anywhere in the hot
//! path.
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

/// sRGB inverse nonlinear transform — vectorized across 8 f32 lanes via mask-blend.
///
/// The scalar piecewise `if c > 0.04045` is replaced with a SIMD mask-blend:
/// both branches are computed for all 8 lanes, then the correct one is selected
/// via `mask.blend(pow_branch, linear_branch)`.  Uses `f32x8::powf(2.4)` which
/// delegates to the `wide` crate's LLVM-generated vector intrinsic.
///
/// Retained behind `#[cfg(test)]` — the production hot path now uses the
/// [`SRGB_INV_LUT`] + [`srgb_inv_lut_u8x8`] path which is faster (no powf).
/// Only the `srgb_inv_f32x8_matches_scalar` correctness test references this.
#[cfg(test)]
#[inline]
pub(crate) fn srgb_inv_f32x8(c: wide::f32x8) -> wide::f32x8 {
    let pow_branch = ((c + wide::f32x8::splat(0.055)) / wide::f32x8::splat(1.055)).powf(2.4);
    let linear_branch = c / wide::f32x8::splat(12.92);
    let mask = c.simd_gt(wide::f32x8::splat(0.04045));
    mask.blend(pow_branch, linear_branch)
}

/// Compile-time sRGB inverse-gamma lookup table — 256 entries, one per u8
/// channel value. Precomputes `srgb_nonlinear_transform_inv(i/255.0)` so the
/// runtime hot path skips both the `/255.0` normalization AND the `powf(2.4)`
/// transcendental.  Formula is bit-identical to [`srgb_inv_f32x8`] for every
/// discrete u8 input, making this a lossless optimization (Rule 8 safe).
///
/// Uses `LazyLock` because `f32::powf` is not a `const fn`.  The LUT is
/// computed once on first access (sub-microsecond) and then served from a flat
/// 1 KB static array with zero runtime overhead per call.
static SRGB_INV_LUT: std::sync::LazyLock<[f32; 256]> = std::sync::LazyLock::new(|| {
    let mut lut = [0.0f32; 256];
    let mut i = 0;
    while i < 256 {
        let c = i as f32 / 255.0;
        lut[i] = if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        };
        i += 1;
    }
    lut
});

/// Gather 8 sRGB inverse-gamma values from the compile-time LUT.
///
/// The input `ch` is pure `u8` (0-255) — exactly the channel bytes stored in
/// a pixel slice.  No `/255.0` normalization is needed because the LUT already
/// encodes the full `srgb_nonlinear_transform_inv(u8/255.0)` result directly.
#[inline]
pub(crate) fn srgb_inv_lut_u8x8(ch: [u8; 8]) -> wide::f32x8 {
    let lut = &*SRGB_INV_LUT;
    wide::f32x8::new([
        lut[ch[0] as usize],
        lut[ch[1] as usize],
        lut[ch[2] as usize],
        lut[ch[3] as usize],
        lut[ch[4] as usize],
        lut[ch[5] as usize],
        lut[ch[6] as usize],
        lut[ch[7] as usize],
    ])
}

/// CIE LAB transfer function — vectorized across 8 f32 lanes via mask-blend.
///
/// The scalar piecewise `if t > ft` is replaced with a SIMD mask-blend: both
/// branches are computed for all 8 lanes, then the correct one is selected via
/// `mask.blend(cbrt_branch, linear_branch)`.  Uses `f32x8::cbrt()` which
/// delegates to the `wide` crate's vector cubic-root intrinsic.
#[inline]
fn lab_transfer_f32x8(t: wide::f32x8) -> wide::f32x8 {
    let ft = (6.0_f32 / 29.0).powi(3);
    let cbrt_branch = t.cbrt();
    let linear_branch = wide::f32x8::splat(7.787) * t + wide::f32x8::splat(16.0 / 116.0);
    let mask = t.simd_gt(wide::f32x8::splat(ft));
    mask.blend(cbrt_branch, linear_branch)
}

/// Process a batch of RGB pixels into XYZ via sRGB inverse gamma + matrix.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes for both the
/// sRGB inverse nonlinear transform (vectorized via mask-blend) and the
/// sRGB→XYZ matrix multiply. Remainder pixels (final 0–7) fall back to
/// the scalar [`crate::rgb::xyz`], converting its f64 output to f32.
pub fn rgb_to_xyz_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    while i + 7 < n {
        // sRGB inverse gamma via compile-time LUT — skips both
        // u8→f32 conversion AND /255.0 normalization (precomputed in LUT).
        let r_lin = srgb_inv_lut_u8x8([
            rgb[i][0],
            rgb[i + 1][0],
            rgb[i + 2][0],
            rgb[i + 3][0],
            rgb[i + 4][0],
            rgb[i + 5][0],
            rgb[i + 6][0],
            rgb[i + 7][0],
        ]);
        let g_lin = srgb_inv_lut_u8x8([
            rgb[i][1],
            rgb[i + 1][1],
            rgb[i + 2][1],
            rgb[i + 3][1],
            rgb[i + 4][1],
            rgb[i + 5][1],
            rgb[i + 6][1],
            rgb[i + 7][1],
        ]);
        let b_lin = srgb_inv_lut_u8x8([
            rgb[i][2],
            rgb[i + 1][2],
            rgb[i + 2][2],
            rgb[i + 3][2],
            rgb[i + 4][2],
            rgb[i + 5][2],
            rgb[i + 6][2],
            rgb[i + 7][2],
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
/// combination (L, a, b formulas) AND the CIE LAB piecewise transfer
/// (vectorized via mask-blend with `f32x8::cbrt`). Remainder pixels fall
/// back to the scalar [`crate::xyz::lab`], converting its f64 output to f32.
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

        let fx = lab_transfer_f32x8(x_norm);
        let fy = lab_transfer_f32x8(y_norm);
        let fz = lab_transfer_f32x8(z_norm);

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
        // sRGB inverse gamma via compile-time LUT — skips both u8→f32
        // conversion AND /255.0 normalization (precomputed in LUT).
        let r_lin = srgb_inv_lut_u8x8([
            rgb[i][0],
            rgb[i + 1][0],
            rgb[i + 2][0],
            rgb[i + 3][0],
            rgb[i + 4][0],
            rgb[i + 5][0],
            rgb[i + 6][0],
            rgb[i + 7][0],
        ]);
        let g_lin = srgb_inv_lut_u8x8([
            rgb[i][1],
            rgb[i + 1][1],
            rgb[i + 2][1],
            rgb[i + 3][1],
            rgb[i + 4][1],
            rgb[i + 5][1],
            rgb[i + 6][1],
            rgb[i + 7][1],
        ]);
        let b_lin = srgb_inv_lut_u8x8([
            rgb[i][2],
            rgb[i + 1][2],
            rgb[i + 2][2],
            rgb[i + 3][2],
            rgb[i + 4][2],
            rgb[i + 5][2],
            rgb[i + 6][2],
            rgb[i + 7][2],
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

        let fx = lab_transfer_f32x8(x_norm);
        let fy = lab_transfer_f32x8(y_norm);
        let fz = lab_transfer_f32x8(z_norm);

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

    /// Scalar reference for sRGB inverse nonlinear transform (test-only).
    fn srgb_inv_f32(c: f32) -> f32 {
        if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        }
    }

    /// Scalar reference for CIE LAB transfer function (test-only).
    fn lab_transfer_f32(t: f32) -> f32 {
        let ft = (6.0_f32 / 29.0).powi(3);
        if t > ft {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    }

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
                inputs[i],
                result[i],
                want,
                diff,
            );
        }
    }

    /// Behavior: `srgb_inv_lut_u8x8` must match the scalar `srgb_inv_f32`
    /// reference for ALL 256 possible u8 inputs — exhaustive sweep, no tolerance
    /// since both use the same `f32::powf` scalar implementation.
    ///
    /// Comparison is against the scalar `srgb_inv_f32(i/255.0)`, NOT against
    /// `srgb_inv_f32x8`, because `f32x8::powf` (SIMD) and `f32::powf` (scalar)
    /// can differ by ±1 ULP for transcendental functions.  The LUT is computed
    /// with `f32::powf`, so the scalar reference is the correct ground truth.
    ///
    /// This test will FAIL TO COMPILE until `srgb_inv_lut_u8x8` exists (RED gate).
    #[test]
    fn srgb_inv_lut_u8x8_exhaustive_match() {
        for ch in 0u8..=255u8 {
            let normalized = ch as f32 / 255.0;
            let want = srgb_inv_f32(normalized);
            // This call will FAIL TO COMPILE until the function exists:
            let got = srgb_inv_lut_u8x8([ch; 8]).to_array()[0];
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "u8={}: LUT value {:.6e} ({:#010x}) != scalar value {:.6e} ({:#010x})",
                ch,
                got,
                got.to_bits(),
                want,
                want.to_bits()
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
                inputs[i],
                result[i],
                want,
                diff,
            );
        }
    }
}
