//! CPU-SIMD batch conversion for rgb→hsl using mask-blend hue selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once. The scalar reference
//! is [`crate::rgb::hsl`] which normalises RGB to [0,1], finds min/max/delta,
//! computes hue via a 3-way branch on which channel is max, sat via a
//! lightness-based branch, and scales to h∈[0,360], s∈[0,100], l∈[0,100].
//!
//! This SIMD path replaces per-pixel branching with vectorised mask-blend:
//! all three candidate hue expressions are computed for all 8 lanes and the
//! correct one is selected by blending with the channel-maximum masks. The
//! achromatic case (max==min, delta==0) is guarded by a second blend that
//! forces hue and saturation to zero. Saturation also uses a mask-blend on
//! the lightness threshold, matching the JS branch logic exactly.
//!
//! ## Tolerance
//!
//! Each SIMD lane performs the same sequence of operations as the scalar f64
//! route in f32 instead. f32 has ~7 decimal digits of precision vs f64's
//! ~15; the hue division by delta (which can be as small as 1/255≈0.004)
//! amplifies this gap by up to ~250×, reaching ~3e-4 in the worst case.
//!
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - s (0–100): absolute tolerance ≤ 1e-3
//! - l (0–100): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hsl_routes.rs`.

use wide::f32x8;

/// Process a batch of RGB pixels into HSL via mask-blend SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. All three
/// candidate hue expressions are computed concurrently and selected
/// with `blend` using the channel-maximum masks, avoiding per-pixel
/// branching. Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::hsl`], converting its f64 output to f32.
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
/// zero via a final blend, matching the JS `if (max == min) { h = 0 }`
/// and `if (max == min) { s = 0 }` guards.
pub fn rgb_to_hsl_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let zero = f32x8::splat(0.0);
    let two = f32x8::splat(2.0);
    let four = f32x8::splat(4.0);
    let inv255 = f32x8::splat(1.0 / 255.0);
    let sixty = f32x8::splat(60.0);
    let three_sixty = f32x8::splat(360.0);
    let half = f32x8::splat(0.5);
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
        let sum_mm = max + min;

        // Lightness: l = (max + min) / 2
        let l = sum_mm * half;

        // ── Hue via mask-blend ────────────────────────────────────
        // Compute all three hue candidate expressions for all 8 lanes.
        // These are safe to compute even for lanes where the channel
        // is NOT the maximum — we select via blend afterwards.
        let hue_r = (g_norm - b_norm) / delta; // r-is-max
        let hue_g = two + (b_norm - r_norm) / delta; // g-is-max
        let hue_b = four + (r_norm - g_norm) / delta; // b-is-max

        // Build masks (simd_eq is an inherent method on f32x8 in wide ≥1.5)
        let mask_r = max.simd_eq(r_norm);
        let mask_g = max.simd_eq(g_norm);

        // Select: b→g→r (mirrors JS else-if chain: b else, g elif, r if)
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

        // ── Saturation via mask-blend ─────────────────────────────
        // JS: if l<=0.5: delta/(max+min) else: delta/(2 - max - min)
        let s_lo = delta / sum_mm;
        let s_hi = delta / (two - sum_mm);

        let mask_lo = l.simd_le(half);
        let mut sat = s_hi;
        sat = mask_lo.blend(s_lo, sat); // l<=0.5 → pick s_lo

        // Force hue and saturation to zero when max==min.
        let mask_achromatic = max.simd_eq(min);
        sat = mask_achromatic.blend(zero, sat);
        hue = mask_achromatic.blend(zero, hue);

        // Scale sat and light by 100
        sat *= hundred;
        let l_scaled = l * hundred;

        // Write results
        let h_arr = hue.to_array();
        let s_arr = sat.to_array();
        let l_arr = l_scaled.to_array();

        for j in 0..8 {
            result.push([h_arr[j], s_arr[j], l_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::hsl(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
