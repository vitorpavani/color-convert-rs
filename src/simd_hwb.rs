//! CPU-SIMD batch conversion for rgb→hwb using mask-blend hue selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→hwb ([`rgb_to_hwb_batch`])
//!
//! The scalar reference is [`crate::rgb::hwb`], which computes hue via
//! `hsl_f64(rgb)[0]`, whiteness as `min×100`, and blackness as
//! `(1-max)×100`. This SIMD path reuses the same 3-way mask-blend hue
//! computation as [`crate::simd_hsl::rgb_to_hsl_batch`], then adds the
//! trivial w/b vector math.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - w (0–100): absolute tolerance ≤ 1e-3
//! - b (0–100): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hwb_routes.rs`.

use wide::f32x8;

/// Process a batch of RGB pixels into HWB via mask-blend SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. The hue is
/// computed via the same 3-way mask-blend selection as
/// [`crate::simd_hsl::rgb_to_hsl_batch`], matching the scalar
/// `hwb_f64` behaviour which calls `hsl_f64(rgb)[0]`. Whiteness and
/// blackness are straight-line vector ops (`min×100`, `(1-max)×100`).
/// Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::hwb`], converting its f64 output to f32.
///
/// # Mask-blend strategy
///
/// The JS reference computes HWB hue from `hsl(rgb)[0]`, which uses
/// an if/else-if chain to select the hue formula based on which channel
/// is the maximum. We compute all three candidate expressions for all 8
/// lanes simultaneously and use SIMD blend to select:
///
/// 1. Compute `hue_r`, `hue_g`, `hue_b` — the three candidate expressions.
/// 2. Build masks `mask_r = (max == r)`, `mask_g = (max == g)`.
/// 3. Select: start with `hue_b` (JS "else"), blend in `hue_g` where
///    `mask_g` is true, then blend in `hue_r` where `mask_r` is true.
///    This mirrors the JS/HSL precedence: r checked first, then g, else b.
///
/// Achromatic pixels (max==min) have their hue forced to zero via a
/// final blend, matching the JS `if (max == min) { h = 0 }` guard.
pub fn rgb_to_hwb_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let zero = f32x8::splat(0.0);
    let one = f32x8::splat(1.0);
    let two = f32x8::splat(2.0);
    let four = f32x8::splat(4.0);
    let inv255 = f32x8::splat(1.0 / 255.0);
    let sixty = f32x8::splat(60.0);
    let three_sixty = f32x8::splat(360.0);
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

        // min, max, delta
        let min = r_norm.min(g_norm).min(b_norm);
        let max = r_norm.max(g_norm).max(b_norm);
        let delta = max - min;

        // Whiteness = min × 100, blackness = (1 - max) × 100
        let w = min * hundred;
        let blk = (one - max) * hundred;

        // ── Hue via mask-blend (same as simd_hsl::rgb_to_hsl_batch) ──
        // Compute all three hue candidate expressions for all 8 lanes.
        let hue_r = (g_norm - b_norm) / delta; // r-is-max
        let hue_g = two + (b_norm - r_norm) / delta; // g-is-max
        let hue_b = four + (r_norm - g_norm) / delta; // b-is-max

        // Build masks (simd_eq is an inherent method on f32x8 in wide ≥1.5)
        let mask_r = max.simd_eq(r_norm);
        let mask_g = max.simd_eq(g_norm);

        // Select: b→g→r (mirrors JS else-if chain: r first, then g, else b)
        // mask.blend(true_val, false_val): pick true_val where mask bit is set
        let mut hue = hue_b;
        hue = mask_g.blend(hue_g, hue); // g==max → pick hue_g
        hue = mask_r.blend(hue_r, hue); // r==max → pick hue_r

        // Scale by 60, clamp to 360, handle negatives
        hue *= sixty;
        hue = hue.min(three_sixty);
        // if h < 0: h += 360
        let mask_neg = hue.simd_lt(zero);
        hue = mask_neg.blend(hue + three_sixty, hue);

        // Achromatic guard: when max==min, force hue to zero.
        let mask_achromatic = max.simd_eq(min);
        hue = mask_achromatic.blend(zero, hue);

        // Write results
        let h_arr = hue.to_array();
        let w_arr = w.to_array();
        let b_arr = blk.to_array();

        for j in 0..8 {
            result.push([h_arr[j], w_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::hwb(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
