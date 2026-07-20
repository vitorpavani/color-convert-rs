//! CPU-SIMD batch conversion for rgb→hsv using mask-blend selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→hsv ([`rgb_to_hsv_batch`])
//!
//! The scalar reference is [`crate::rgb::hsv`], which normalises RGB to
//! [0,1], finds min/max/delta, computes v=max, s=delta/max (0 if diff==0),
//! hue via a 3-way branch on which channel is max, and scales to
//! h∈[0,360], s∈[0,100], v∈[0,100]. This SIMD path replaces branching
//! with mask-blend on all three candidate hue expressions. The achromatic
//! case (max==min) and max==0 case force hue and s to zero via blend.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//! The hue calculation involves division by delta (as small as 1/255≈0.004),
//! amplifying the initial f32 representation error.
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - s (0–100): absolute tolerance ≤ 1e-3
//! - v (0–100): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hsv_routes.rs`.

use wide::f32x8;

/// Public wrapper: auto-selects serial or parallel based on input size.
///
/// Delegates to [`crate::simd_parallel::auto_batch`] which chooses serial
/// SIMD for ≤ 4096 pixels and multi-core rayon for larger batches.
///
/// # Mask-blend strategy
///
/// The JS reference uses an if/else-if chain to select the hue formula
/// based on which channel is the maximum. We compute all three for all
/// 8 lanes simultaneously and use SIMD blend to select:
///
/// 1. Compute `hue_r`, `hue_g`, `hue_b` — the three candidate expressions.
/// 2. Build masks `mask_r = (max == r)`, `mask_g = (max == g)`.
/// 3. Select: start with `hue_b` (JS "else"), blend in `hue_g` where
///    `mask_g` is true, then blend in `hue_r` where `mask_r` is true.
///    This mirrors the JS precedence: r checked first, then g, else b.
///
/// Achromatic pixels (max==min) have their hue AND saturation forced to
/// zero via a final blend, matching the JS `if (diff == 0) { h = 0; s = 0 }`.
/// Black pixels (max==0) also get zero hue/saturation via mask-blend to
/// guard the `s = delta / max * 100` division.
pub fn rgb_to_hsv_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(rgb, rgb_to_hsv_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
pub(crate) fn rgb_to_hsv_batch_serial(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let zero = f32x8::splat(0.0);
    let inv255 = f32x8::splat(1.0 / 255.0);
    let three_sixty = f32x8::splat(360.0);
    let hundred = f32x8::splat(100.0);
    let one_third = f32x8::splat(1.0 / 3.0);
    let two_thirds = f32x8::splat(2.0 / 3.0);
    let six = f32x8::splat(6.0);

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

        // min, max, delta
        let min = r_norm.min(g_norm).min(b_norm);
        let max = r_norm.max(g_norm).max(b_norm);
        let delta = max - min;

        // Value: v = max * 100
        let v = max * hundred;

        // Saturation: s = delta / max * 100
        let s = delta / max * hundred;

        // ── Hue via mask-blend ────────────────────────────────────
        // Mirroring the JS diffc helper: diffc(c) = (v-c)/(6·delta) + 0.5
        //   hue_r = bdif - gdif  =  (g - b) / (6·delta)           [r-is-max]
        //   hue_g = 1/3 + rdif - bdif = 1/3 + (b - r)/(6·delta)   [g-is-max]
        //   hue_b = 2/3 + gdif - rdif = 2/3 + (r - g)/(6·delta)   [b-is-max]
        let six_delta = six * delta;
        let hue_r = (g_norm - b_norm) / six_delta;
        let hue_g = one_third + (b_norm - r_norm) / six_delta;
        let hue_b = two_thirds + (r_norm - g_norm) / six_delta;

        // Build masks (simd_eq is an inherent method on f32x8 in wide ≥1.5)
        let mask_r = max.simd_eq(r_norm);
        let mask_g = max.simd_eq(g_norm);

        // Select: b→g→r (mirrors JS else-if chain: r first, then g, else b)
        // mask.blend(true_val, false_val): pick true_val where mask bit is set
        let mut hue = hue_b;
        hue = mask_g.blend(hue_g, hue); // g==max → pick hue_g
        hue = mask_r.blend(hue_r, hue); // r==max → pick hue_r

        // Scale to degrees, clamp, handle negatives
        hue *= three_sixty;
        hue = hue.min(three_sixty);
        // if h < 0: h += 360
        let mask_neg = hue.simd_lt(zero);
        hue = mask_neg.blend(hue + three_sixty, hue);

        // Force hue and saturation to zero when max==min or max==0.
        let mask_achromatic = max.simd_eq(min);
        let mask_black = max.simd_eq(zero);
        hue = mask_achromatic.blend(zero, hue);
        hue = mask_black.blend(zero, hue);
        let mut sat = s;
        sat = mask_achromatic.blend(zero, sat);
        sat = mask_black.blend(zero, sat);

        // Write results
        let h_arr = hue.to_array();
        let s_arr = sat.to_array();
        let v_arr = v.to_array();

        for j in 0..8 {
            result.push([h_arr[j], s_arr[j], v_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::hsv(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
