//! SIMD CMYK batch conversion from RGB using `wide::f32x8`.
//!
//! ## Reference
//!
//! Mirrors the scalar [`crate::rgb::cmyk_f64`] which faithfully ports the
//! npm `color-convert` JS library's `convert.rgb.cmyk` (conversions.js,
//! lines 158–174). The JS `|| 0` fallback when `k == 1` (division by
//! `(1-k) == 0`) is implemented as a mask-blend to zero out c, m, y where
//! `denom == 0`.
//!
//! ## Tolerance
//!
//! f32 output compared to the f64 scalar reference: absolute tolerance
//! ≤ 1e-3 for all four channels (c, m, y, k ∈ [0, 100]). The division
//! by `(1-k)` is guarded at the `k == 1` extreme; the remaining
//! amplification is bounded.
//!
//! ## SIMD layout
//!
//! - `f32x8` lanes: pixels `[i+0]` through `[i+7]`
//! - Scalar remainder (< 8 pixels): falls back to `crate::rgb::cmyk_f64`
//!   and converts f64→f32.

use wide::f32x8;

/// Convert a batch of RGB `[u8;3]` pixels to CMYK `[f32;4]` (c,m,y,k ∈ [0,100]).
///
/// Processes 8 pixels at a time using `wide::f32x8` SIMD lanes. The scalar
/// remainder (< 8 pixels) is handled by `crate::rgb::cmyk_f64`.
pub fn rgb_to_cmyk_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 4]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let one = f32x8::splat(1.0);
    let zero = f32x8::splat(0.0);
    let inv255 = f32x8::splat(1.0 / 255.0);
    let hundred = f32x8::splat(100.0);

    while i + 7 < n {
        // Load 8 pixels into SoA SIMD lanes
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

        // Normalise to [0, 1]
        let r_norm = r * inv255;
        let g_norm = g * inv255;
        let b_norm = b * inv255;

        // k = 1 - max(r, g, b)
        let max = r_norm.max(g_norm).max(b_norm);
        let k = one - max;

        // denom = 1 - k
        let denom = one - k;

        // Compute c, m, y for all 8 lanes — these are safe even for
        // lanes where denom == 0 (we mask-blend to zero afterwards).
        let c = (one - r_norm - k) / denom;
        let m_val = (one - g_norm - k) / denom;
        let y = (one - b_norm - k) / denom;

        // Black guard: where denom == 0 (k == 1, pure black), force
        // c, m, y to 0 — mirroring the JS `|| 0` fallback.
        // mask.blend(true_val, false_val): pick true_val (0) where mask
        // is set (denom == 0), otherwise pick the computed value.
        let mask_black = denom.simd_eq(zero);
        let c_safe = mask_black.blend(zero, c);
        let m_safe = mask_black.blend(zero, m_val);
        let y_safe = mask_black.blend(zero, y);

        // Scale to [0, 100]
        let c_scaled = c_safe * hundred;
        let m_scaled = m_safe * hundred;
        let y_scaled = y_safe * hundred;
        let k_scaled = k * hundred;

        // Write results
        let c_arr = c_scaled.to_array();
        let m_arr = m_scaled.to_array();
        let y_arr = y_scaled.to_array();
        let k_arr = k_scaled.to_array();

        for j in 0..8 {
            result.push([c_arr[j], m_arr[j], y_arr[j], k_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::cmyk_f64([
            f64::from(rgb[i][0]),
            f64::from(rgb[i][1]),
            f64::from(rgb[i][2]),
        ]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
            f64_result[3] as f32,
        ]);
        i += 1;
    }

    result
}
