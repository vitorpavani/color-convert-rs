//! CPU-SIMD batch conversion for oklab→rgb (inverse) route.
//!
//! Vectorizes the full inverse Oklab pipeline across 8 `f32` lanes via
//! [`wide::f32x8`]: inverse Oklab matrix (L,a,b → LMS) → cube (x³) →
//! inverse LMS matrix (→ linear sRGB) → forward sRGB gamma (mask-blend
//! `powf(1.0/2.4)`) → ×255.
//!
//! ## Tolerance
//!
//! f32 (~7 decimal digits) vs f64 (~15 decimal digits) through the dual
//! matrix + cube³ + gamma `powf(1/2.4)` chain yields a detectable gap.
//! Absolute tolerance per channel: **1e-2** (output range [0, 255]).
//!
//! ## Reference
//!
//! Faithful to `convert.oklab.rgb` in color-convert@3.1.3 `conversions.js`
//! lines 564–579 via the scalar `oklab::rgb` port.
//!
//! ## Batch API
//!
//! Processes 8 pixels at a time via `f32x8` SIMD lanes with scalar
//! remainder fallback to [`crate::oklab::rgb`] for the final 0–7 pixels.

/// Forward sRGB nonlinear transform — vectorized across 8 f32 lanes via mask-blend.
///
/// The piecewise `if c > 0.0031308` is replaced with a SIMD mask-blend:
/// both branches are computed for all 8 lanes, then the correct one is
/// selected via `mask.blend(pow_branch, linear_branch)`. Replicated from
/// `simd_xyz::srgb_fwd_f32x8` (private there) to keep this module self-contained.
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
/// scalar `oklab::rgb` which also returns unrounded floats.
pub fn oklab_to_rgb_batch(oklab: &[[f32; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(oklab, oklab_to_rgb_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
///
/// Processes 8 pixels at a time using f32x8 SIMD lanes. Remainder
/// pixels (final 0–7) fall back to the scalar [`crate::oklab::rgb`],
/// converting its f64 output to f32.
pub(crate) fn oklab_to_rgb_batch_serial(oklab: &[[f32; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = oklab.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    // Inverse Oklab matrix coefficients (Oklab → LMS), truncated to f32
    // From scalar oklab::rgb: l = (ll + 0.3963377774*aa + 0.2158037573*bb)³
    const IO00: f32 = 0.396_337_78;
    const IO01: f32 = 0.215_803_76;
    const IO10: f32 = -0.105_561_345;
    const IO11: f32 = -0.063_854_17;
    const IO20: f32 = -0.089_484_18;
    const IO21: f32 = -1.291_485_5;

    // Inverse LMS matrix coefficients (LMS → linear sRGB), truncated to f32
    const IM00: f32 = 4.076_741_7;
    const IM01: f32 = -3.307_711_6;
    const IM02: f32 = 0.230_969_93;
    const IM10: f32 = -1.268_438;
    const IM11: f32 = 2.609_757_4;
    const IM12: f32 = -0.341_319_4;
    const IM20: f32 = -0.004_196_086_3;
    const IM21: f32 = -0.703_418_6;
    const IM22: f32 = 1.707_614_7;

    while i + 7 < n {
        let l_in = f32x8::new([
            oklab[i][0],
            oklab[i + 1][0],
            oklab[i + 2][0],
            oklab[i + 3][0],
            oklab[i + 4][0],
            oklab[i + 5][0],
            oklab[i + 6][0],
            oklab[i + 7][0],
        ]);
        let a_in = f32x8::new([
            oklab[i][1],
            oklab[i + 1][1],
            oklab[i + 2][1],
            oklab[i + 3][1],
            oklab[i + 4][1],
            oklab[i + 5][1],
            oklab[i + 6][1],
            oklab[i + 7][1],
        ]);
        let b_in = f32x8::new([
            oklab[i][2],
            oklab[i + 1][2],
            oklab[i + 2][2],
            oklab[i + 3][2],
            oklab[i + 4][2],
            oklab[i + 5][2],
            oklab[i + 6][2],
            oklab[i + 7][2],
        ]);

        // Normalize: oklab l/a/b are in [0,100]/[-100,100]/[-100,100] → divide by 100
        let ll = l_in / f32x8::splat(100.0);
        let aa = a_in / f32x8::splat(100.0);
        let bb = b_in / f32x8::splat(100.0);

        // Inverse Oklab matrix: (L,a,b) → LMS (pre-cubed)
        // l = ll + 0.39633778*aa + 0.21580376*bb  → cube → LMS L
        // m = ll - 0.105561345*aa - 0.063854173*bb  → cube → LMS M
        // s = ll - 0.08948418*aa - 1.2914855*bb  → cube → LMS S
        let l = ll + aa * f32x8::splat(IO00) + bb * f32x8::splat(IO01);
        let m = ll + aa * f32x8::splat(IO10) + bb * f32x8::splat(IO11);
        let s = ll + aa * f32x8::splat(IO20) + bb * f32x8::splat(IO21);

        // Cube: l³, m³, s³ — use x*x*x (3 muls) instead of powf(3.0) for speed
        let l_cube = l * l * l;
        let m_cube = m * m * m;
        let s_cube = s * s * s;

        // Inverse LMS matrix: LMS³ → linear sRGB
        let r_lin =
            l_cube * f32x8::splat(IM00) + m_cube * f32x8::splat(IM01) + s_cube * f32x8::splat(IM02);
        let g_lin =
            l_cube * f32x8::splat(IM10) + m_cube * f32x8::splat(IM11) + s_cube * f32x8::splat(IM12);
        let b_lin =
            l_cube * f32x8::splat(IM20) + m_cube * f32x8::splat(IM21) + s_cube * f32x8::splat(IM22);

        // Apply forward sRGB non-linear transform and scale to 0–255
        let r = srgb_fwd_f32x8(r_lin) * f32x8::splat(255.0);
        let g = srgb_fwd_f32x8(g_lin) * f32x8::splat(255.0);
        let b = srgb_fwd_f32x8(b_lin) * f32x8::splat(255.0);

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
        let f64_input = [oklab[i][0] as f64, oklab[i][1] as f64, oklab[i][2] as f64];
        let f64_result = crate::oklab::rgb(f64_input);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
