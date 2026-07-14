//! CPU-SIMD batch conversion for rgb→hcg using mask-blend hue selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→hcg ([`rgb_to_hcg_batch`])
//!
//! The scalar reference is [`crate::rgb::hcg`], which normalises RGB to
//! [0,1], finds min/max/chroma, computes grayscale via the chroma<1 guard,
//! and hue via a 3-way branch on which channel is max. This SIMD path
//! replaces branching with mask-blend on all three candidate hue expressions.
//! Achromatic pixels (chroma≤0) have their hue forced to zero via blend.
//! The grayscale div-by-zero guard (chroma==1 → 0) also uses mask-blend.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//! Division by chroma (as small as 1/255≈0.004) amplifies the gap to ~3e-4.
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - c (0–100): absolute tolerance ≤ 1e-3
//! - g (0–100): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hcg_routes.rs`.

use wide::f32x8;

/// Process a batch of RGB pixels into HCG via mask-blend SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. All three
/// candidate hue expressions are computed concurrently and selected
/// with `blend` using the channel-maximum masks, avoiding per-pixel
/// branching. The hue `% 6.0` and `% 1.0` (via `fract()`) are applied
/// after selection. Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::hcg`], converting its f64 output to f32.
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
/// Achromatic pixels (max==min) have their hue forced to zero via a
/// final blend, matching the JS `if (chroma <= 0) { h = 0 }` guard.
///
/// # Grayscale chroma-guard
///
/// The JS `gray = chroma < 1.0 ? min/(1-chroma) : 0.0` is implemented
/// via a mask-blend: divide `min/(1-chroma)` for all lanes, then blend
/// zero where `chroma >= 1.0`. The division is safe because the mask
/// selects zero (not the inf/NaN result) for lanes where chroma==1.
pub fn rgb_to_hcg_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let zero = f32x8::splat(0.0);
    let one = f32x8::splat(1.0);
    let two = f32x8::splat(2.0);
    let four = f32x8::splat(4.0);
    let six = f32x8::splat(6.0);
    let inv255 = f32x8::splat(1.0 / 255.0);
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

        // min, max, chroma
        let min = r_norm.min(g_norm).min(b_norm);
        let max = r_norm.max(g_norm).max(b_norm);
        let chroma = max - min;

        // ── Grayscale via chroma-guard mask-blend ─────────────────
        // chroma < 1.0 → min/(1-chroma),  else → 0.0
        // The division may produce inf/NaN when chroma==1, but the
        // mask selects zero for those lanes, so it is safe.
        let chroma_lt_one = chroma.simd_lt(one);
        let gray_raw = min / (one - chroma);
        let grayscale = chroma_lt_one.blend(gray_raw, zero);

        // ── Hue via mask-blend ───────────────────────────────────
        // Compute all three hue candidate expressions for all 8 lanes.
        let hue_r = (g_norm - b_norm) / chroma; // r-is-max
        let hue_g = two + (b_norm - r_norm) / chroma; // g-is-max
        let hue_b = four + (r_norm - g_norm) / chroma; // b-is-max

        // Build masks
        let mask_r = max.simd_eq(r_norm);
        let mask_g = max.simd_eq(g_norm);

        // Select: b→g→r (mirrors JS else-if chain: r first, then g, else b)
        let mut hue = hue_b;
        hue = mask_g.blend(hue_g, hue);
        hue = mask_r.blend(hue_r, hue);

        // Apply % 6.0 (needed for r-is-max case where raw value can be
        // negative; harmless no-op for g-is-max [1,3] and b-is-max [3,5]).
        let hue_div6 = hue / six;
        hue -= hue_div6.trunc() * six;

        // Divide by 6, then % 1.0 via fract() (same as IEEE 754 fmod 1.0)
        hue /= six;
        hue = hue.fract();

        // Scale to [0, 360]
        hue *= three_sixty;

        // Achromatic guard: when max==min, force hue to zero.
        let mask_achromatic = max.simd_eq(min);
        hue = mask_achromatic.blend(zero, hue);

        // Scale chroma and grayscale by 100
        let chroma_scaled = chroma * hundred;
        let grayscale_scaled = grayscale * hundred;

        // Write results
        let h_arr = hue.to_array();
        let c_arr = chroma_scaled.to_array();
        let g_arr = grayscale_scaled.to_array();

        for j in 0..8 {
            result.push([h_arr[j], c_arr[j], g_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::rgb::hcg(rgb[i]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
