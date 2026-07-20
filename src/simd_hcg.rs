//! CPU-SIMD batch conversion for rgb↔hcg using mask-blend selection.
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
//! ## hcg→rgb ([`hcg_to_rgb_batch`])
//!
//! The scalar reference is [`crate::hcg::rgb`], which normalises HCG to
//! [0,1], handles the achromatic case (c==0 → r=g=b = g×255), then
//! applies a 6-way piecewise function via `(h%1)×6` hue slices. This SIMD
//! path computes all six candidate triples concurrently and selects via
//! mask-blend in reverse if/else-if precedence. The achromatic case is
//! handled via a `c==0` mask-blend overriding the piecewise results.
//! Negative hue values (from the RGB→HCG conversion) are forced to the
//! else/default case, matching the scalar f64 `floor()` semantics where
//! f32 `trunc()` differs for negative inputs.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//!
//! For rgb→hcg: division by chroma (as small as 1/255≈0.004) amplifies
//! the gap to ~3e-4.
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - c (0–100): absolute tolerance ≤ 1e-3
//! - g (0–100): absolute tolerance ≤ 1e-3
//!
//! For hcg→rgb: 6-way piecewise linear interpolation in [0,1] scaled by
//! 255; the f32 vs f64 gap is ~3e-5 in [0,255].
//! - r/g/b (0–255): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hcg_routes.rs`.

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
    crate::simd_parallel::auto_batch(rgb, rgb_to_hcg_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
pub(crate) fn rgb_to_hcg_batch_serial(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
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

/// Public wrapper: auto-selects serial or parallel based on input size.
///
/// # Achromatic guard
///
/// When chroma is exactly zero (`c == 0`), all three channels equal
/// `gray × 255` (mirroring the JS `if (c == 0)` fast path). The SIMD
/// path applies this via a mask-blend on the `c == 0` condition,
/// overriding the 6-way piecewise results.
pub fn hcg_to_rgb_batch(hcg: &[[f32; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(hcg, hcg_to_rgb_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
pub(crate) fn hcg_to_rgb_batch_serial(hcg: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let n = hcg.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let inv360 = f32x8::splat(1.0 / 360.0);
    let inv100 = f32x8::splat(1.0 / 100.0);
    let one = f32x8::splat(1.0);
    let zero = f32x8::splat(0.0);
    let s255 = f32x8::splat(255.0);
    let six = f32x8::splat(6.0);

    while i + 7 < n {
        // Load 8 HCG triples into SoA SIMD lanes
        let h = f32x8::new([
            hcg[i][0],
            hcg[i + 1][0],
            hcg[i + 2][0],
            hcg[i + 3][0],
            hcg[i + 4][0],
            hcg[i + 5][0],
            hcg[i + 6][0],
            hcg[i + 7][0],
        ]);
        let c = f32x8::new([
            hcg[i][1],
            hcg[i + 1][1],
            hcg[i + 2][1],
            hcg[i + 3][1],
            hcg[i + 4][1],
            hcg[i + 5][1],
            hcg[i + 6][1],
            hcg[i + 7][1],
        ]);
        let g = f32x8::new([
            hcg[i][2],
            hcg[i + 1][2],
            hcg[i + 2][2],
            hcg[i + 3][2],
            hcg[i + 4][2],
            hcg[i + 5][2],
            hcg[i + 6][2],
            hcg[i + 7][2],
        ]);

        // Normalise h to [0,1], c and g to [0,1]
        let h_norm = h * inv360;
        let c_norm = c * inv100;
        let g_norm = g * inv100;

        // Compute hi = (h % 1.0) * 6 via fract(), v = fract(hi), w = 1-v
        // For negative h_norm, fract() preserves the sign and hi.trunc()
        // returns 0 (not floor(-1) as the scalar f64 does via Math.floor).
        // The mask_neg_h below forces the else/default case for those lanes,
        // matching the JS behaviour where hi.floor() is negative → default.
        let hi = h_norm.fract() * six;
        let v = hi.fract();
        let w = one - v;

        // All six candidate triples for the 6-way hue selection
        // triple_0: [1, v, 0]   triple_1: [w, 1, 0]   triple_2: [0, 1, v]
        // triple_3: [0, w, 1]   triple_4: [v, 0, 1]   triple_5: [1, 0, w]
        let r0 = one;
        let g0 = v;
        let b0 = zero;

        let r1 = w;
        let g1 = one;
        let b1 = zero;

        let r2 = zero;
        let g2 = one;
        let b2 = v;

        let r3 = zero;
        let g3 = w;
        let b3 = one;

        let r4 = v;
        let g4 = zero;
        let b4 = one;

        let r5 = one;
        let g5 = zero;
        let b5 = w;

        // Build masks for hi.floor() ∈ {0,1,2,3,4}
        // (hi.trunc() == floor for hi ≥ 0; negative lanes are handled by mask_neg_h)
        let hi_floor = hi.trunc();
        let mask_neg_h = h_norm.simd_lt(zero);
        let mask0 = hi_floor.simd_eq(zero);
        let mask1 = hi_floor.simd_eq(one);
        let mask2 = hi_floor.simd_eq(f32x8::splat(2.0));
        let mask3 = hi_floor.simd_eq(f32x8::splat(3.0));
        let mask4 = hi_floor.simd_eq(f32x8::splat(4.0));

        // Select in reverse precedence (else → first): 5→4→3→2→1→0
        // Negative h_norm lanes are forced to the else (triple_5) via mask_neg_h
        // mask.blend(true_val, false_val): pick true_val where mask bit is set
        let mut pure_r = r5;
        let mut pure_g = g5;
        let mut pure_b = b5;
        pure_r = mask4.blend(r4, pure_r);
        pure_g = mask4.blend(g4, pure_g);
        pure_b = mask4.blend(b4, pure_b);
        pure_r = mask3.blend(r3, pure_r);
        pure_g = mask3.blend(g3, pure_g);
        pure_b = mask3.blend(b3, pure_b);
        pure_r = mask2.blend(r2, pure_r);
        pure_g = mask2.blend(g2, pure_g);
        pure_b = mask2.blend(b2, pure_b);
        pure_r = mask1.blend(r1, pure_r);
        pure_g = mask1.blend(g1, pure_g);
        pure_b = mask1.blend(b1, pure_b);
        pure_r = mask0.blend(r0, pure_r);
        pure_g = mask0.blend(g0, pure_g);
        pure_b = mask0.blend(b0, pure_b);

        // Force else case (triple_5) for negative h_norm: mask_neg_h.blend(r5, current)
        pure_r = mask_neg_h.blend(r5, pure_r);
        pure_g = mask_neg_h.blend(g5, pure_g);
        pure_b = mask_neg_h.blend(b5, pure_b);

        // Apply chroma scale + grayscale blend: (c * pure + (1-c) * g) * 255
        let mg = (one - c_norm) * g_norm;
        let r_val = (c_norm * pure_r + mg) * s255;
        let g_val = (c_norm * pure_g + mg) * s255;
        let b_val = (c_norm * pure_b + mg) * s255;

        // Achromatic guard: when c == 0, force r=g=b = gray * 255
        let mask_achromatic = c_norm.simd_eq(zero);
        let gray_val = g_norm * s255;
        let r_final = mask_achromatic.blend(gray_val, r_val);
        let g_final = mask_achromatic.blend(gray_val, g_val);
        let b_final = mask_achromatic.blend(gray_val, b_val);

        let r_arr = r_final.to_array();
        let g_arr = g_final.to_array();
        let b_arr = b_final.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::hcg::rgb([hcg[i][0] as f64, hcg[i][1] as f64, hcg[i][2] as f64]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
