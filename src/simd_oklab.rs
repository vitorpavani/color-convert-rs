//! CPU-SIMD batch conversion for rgb→oklab.
//!
//! Vectorizes the full oklab pipeline across 8 `f32` lanes via [`wide::f32x8`]:
//! sRGB inverse gamma (`powf(2.4)` mask-blend) → LMS matrix multiply →
//! cube root (`cbrt`) → Oklab matrix multiply → ×100.
//!
//! ## Tolerance
//!
//! f32 (~7 decimal digits) vs f64 (~15 decimal digits) through three
//! transcendental steps (powf 2.4 + cbrt³) yields a detectable gap.
//! Absolute tolerance per channel: **1e-3** (matching LAB_TOLERANCE).
//!
//! ## Reference
//!
//! Ported from `convert.rgb.oklab` in color-convert@3.1.3 `conversions.js`.
//!
//! ## Batch API
//!
//! Processes 8 pixels at a time via `f32x8` lanes with scalar remainder
//! fallback to [`crate::rgb::oklab`] for the final 0–7 pixels.

/// Public wrapper: auto-selects serial or parallel based on input size.
///
/// Delegates to [`crate::simd_parallel::auto_batch`] which chooses serial
/// SIMD for ≤ 4096 pixels and multi-core rayon for larger batches.
pub fn rgb_to_oklab_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(rgb, rgb_to_oklab_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
///
/// Processes 8 pixels at a time via f32x8 lanes: sRGB inverse gamma
/// (via compile-time LUT) → LMS matrix → cbrt → Oklab matrix → ×100.
/// Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::oklab`], converting its f64 output to f32.
pub(crate) fn rgb_to_oklab_batch_serial(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    use wide::f32x8;

    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    // LMS matrix coefficients (sRGB linear → LMS cone response), truncated to f32 precision
    const M00: f32 = 0.412_221_46;
    const M01: f32 = 0.536_332_55;
    const M02: f32 = 0.051_445_995;
    const M10: f32 = 0.211_903_5;
    const M11: f32 = 0.680_699_5;
    const M12: f32 = 0.107_396_96;
    const M20: f32 = 0.088_302_46;
    const M21: f32 = 0.281_718_83;
    const M22: f32 = 0.629_978_7;

    // Oklab matrix coefficients (LMS' → L'a'b'), truncated to f32 precision
    const O00: f32 = 0.210_454_26;
    const O01: f32 = 0.793_617_8;
    const O02: f32 = 0.004_072_047;
    const O10: f32 = 1.977_998_5;
    const O11: f32 = 2.428_592_2;
    const O12: f32 = 0.450_593_7;
    const O20: f32 = 0.025_904_038;
    const O21: f32 = 0.782_771_76;
    const O22: f32 = 0.808_675_77;

    while i + 7 < n {
        // sRGB inverse gamma via compile-time LUT — skips both u8→f32
        // conversion AND /255.0 normalization (precomputed in LUT).
        let r_lin = crate::simd::srgb_inv_lut_u8x8([
            rgb[i][0],
            rgb[i + 1][0],
            rgb[i + 2][0],
            rgb[i + 3][0],
            rgb[i + 4][0],
            rgb[i + 5][0],
            rgb[i + 6][0],
            rgb[i + 7][0],
        ]);
        let g_lin = crate::simd::srgb_inv_lut_u8x8([
            rgb[i][1],
            rgb[i + 1][1],
            rgb[i + 2][1],
            rgb[i + 3][1],
            rgb[i + 4][1],
            rgb[i + 5][1],
            rgb[i + 6][1],
            rgb[i + 7][1],
        ]);
        let b_lin = crate::simd::srgb_inv_lut_u8x8([
            rgb[i][2],
            rgb[i + 1][2],
            rgb[i + 2][2],
            rgb[i + 3][2],
            rgb[i + 4][2],
            rgb[i + 5][2],
            rgb[i + 6][2],
            rgb[i + 7][2],
        ]);

        // LMS cone response — linear sRGB → LMS, then cube root
        let lp =
            (r_lin * f32x8::splat(M00) + g_lin * f32x8::splat(M01) + b_lin * f32x8::splat(M02))
                .cbrt();
        let mp =
            (r_lin * f32x8::splat(M10) + g_lin * f32x8::splat(M11) + b_lin * f32x8::splat(M12))
                .cbrt();
        let sp =
            (r_lin * f32x8::splat(M20) + g_lin * f32x8::splat(M21) + b_lin * f32x8::splat(M22))
                .cbrt();

        // Oklab matrix — LMS' → L'a'b'
        let l = lp * f32x8::splat(O00) + mp * f32x8::splat(O01) - sp * f32x8::splat(O02);
        let aa = lp * f32x8::splat(O10) - mp * f32x8::splat(O11) + sp * f32x8::splat(O12);
        let bb = lp * f32x8::splat(O20) + mp * f32x8::splat(O21) - sp * f32x8::splat(O22);

        let l_arr = l.to_array();
        let a_arr = aa.to_array();
        let b_arr = bb.to_array();

        for j in 0..8 {
            result.push([l_arr[j] * 100.0, a_arr[j] * 100.0, b_arr[j] * 100.0]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::oklab(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
