//! CPU-SIMD batch conversion for rgb↔hsl using mask-blend selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→hsl ([`rgb_to_hsl_batch`])
//!
//! The scalar reference is [`crate::rgb::hsl`], which normalises RGB to
//! [0,1], finds min/max/delta, computes hue via a 3-way branch on which
//! channel is max, sat via a lightness-based branch, and scales to
//! h∈[0,360], s∈[0,100], l∈[0,100]. This SIMD path replaces branching
//! with mask-blend on all three candidate hue expressions. The achromatic
//! case (max==min) forces hue and saturation to zero via blend.
//!
//! ## hsl→rgb ([`hsl_to_rgb_batch`])
//!
//! The scalar reference is [`crate::hsl::rgb`], which normalises HSL to
//! [0,1], computes t1/t2 from s and l, then applies a 4-way piecewise
//! function (`channel`) per RGB channel with offsets +1/3, 0, -1/3.
//! This SIMD path computes all four piecewise candidates concurrently
//! and selects via mask-blend in reverse if/else-if precedence. The
//! achromatic case (s==0) forces r=g=b=l*255 via mask-blend.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//!
//! For rgb→hsl: hue division by delta (as small as 1/255≈0.004) amplifies
//! the gap to ~3e-4 in the worst case.
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - s (0–100): absolute tolerance ≤ 1e-3
//! - l (0–100): absolute tolerance ≤ 1e-3
//!
//! For hsl→rgb: piecewise linear interpolation in [0,1] scaled by 255;
//! the f32 vs f64 gap is ~3e-5 in [0,255].
//! - r/g/b (0–255): absolute tolerance ≤ 1e-3
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

/// Compute one RGB channel value for 8 lanes via mask-blend piecewise
/// selection, mirroring the JS `convert.hsl.rgb` loop body.
///
/// Returns raw [0,1] float — caller scales by 255.
#[inline]
fn channel_simd(h: f32x8, t1: f32x8, t2: f32x8, offset: f32) -> f32x8 {
    let zero = f32x8::splat(0.0);
    let one = f32x8::splat(1.0);
    let two = f32x8::splat(2.0);
    let two_thirds = f32x8::splat(2.0 / 3.0);
    let six = f32x8::splat(6.0);

    let mut t3 = h + f32x8::splat(offset);

    // Wrap t3 into [0,1]: if t3 < 0 → t3 += 1; if t3 > 1 → t3 -= 1
    let mask_neg = t3.simd_lt(zero);
    t3 = mask_neg.blend(t3 + one, t3);
    let mask_gt = t3.simd_gt(one);
    t3 = mask_gt.blend(t3 - one, t3);

    // All four candidate values, computed for all 8 lanes
    let six_t3 = six * t3;
    let two_t3 = two * t3;
    let three_t3 = f32x8::splat(3.0) * t3;
    let delta = t2 - t1;

    let val_a = t1 + delta * six_t3; // 6*t3 < 1
    let val_b = t2; // 2*t3 < 1
    let val_c = t1 + delta * (two_thirds - t3) * six; // 3*t3 < 2
    let val_d = t1; // else

    // Conditions
    let cond_a = six_t3.simd_lt(one); // 6*t3 < 1
    let cond_b = two_t3.simd_lt(one); // 2*t3 < 1
    let cond_c = three_t3.simd_lt(two); // 3*t3 < 2

    // Blend in reverse precedence (else first, lowest-to-highest)
    let mut result = val_d;
    result = cond_c.blend(val_c, result);
    result = cond_b.blend(val_b, result);
    result = cond_a.blend(val_a, result);

    result
}

/// Process a batch of HSL triples into raw RGB floats (0–255) via
/// mask-blend SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. All four
/// piecewise hue candidates are computed concurrently per channel and
/// selected with `blend`, avoiding per-pixel branching. Remainder
/// pixels (final 0–7) fall back to the scalar [`crate::hsl::rgb`],
/// converting its f64 output to f32.
///
/// # Achromatic guard
///
/// When saturation is exactly zero, all three channels equal `l*255`
/// (mirroring the JS `if (s === 0)` fast path). The SIMD path applies
/// this via a mask-blend on the `s == 0` condition, overriding the
/// piecewise results.
pub fn hsl_to_rgb_batch(hsl: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let n = hsl.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let inv360 = f32x8::splat(1.0 / 360.0);
    let inv100 = f32x8::splat(1.0 / 100.0);
    let half = f32x8::splat(0.5);
    let one = f32x8::splat(1.0);
    let zero = f32x8::splat(0.0);
    let s255 = f32x8::splat(255.0);

    while i + 7 < n {
        // Load 8 HSL triples into SoA SIMD lanes
        let h = f32x8::new([
            hsl[i][0],
            hsl[i + 1][0],
            hsl[i + 2][0],
            hsl[i + 3][0],
            hsl[i + 4][0],
            hsl[i + 5][0],
            hsl[i + 6][0],
            hsl[i + 7][0],
        ]);
        let s = f32x8::new([
            hsl[i][1],
            hsl[i + 1][1],
            hsl[i + 2][1],
            hsl[i + 3][1],
            hsl[i + 4][1],
            hsl[i + 5][1],
            hsl[i + 6][1],
            hsl[i + 7][1],
        ]);
        let l = f32x8::new([
            hsl[i][2],
            hsl[i + 1][2],
            hsl[i + 2][2],
            hsl[i + 3][2],
            hsl[i + 4][2],
            hsl[i + 5][2],
            hsl[i + 6][2],
            hsl[i + 7][2],
        ]);

        // Normalise h to [0,1], s and l to [0,1]
        let h_norm = h * inv360;
        let s_norm = s * inv100;
        let l_norm = l * inv100;

        // Compute t2: l < 0.5 → l*(1+s)  else → l + s - l*s
        let t2_lo = l_norm * (one + s_norm);
        let t2_hi = l_norm + s_norm - l_norm * s_norm;
        let mask_lo_l = l_norm.simd_lt(half);
        let t2 = mask_lo_l.blend(t2_lo, t2_hi);

        // t1 = 2*l - t2
        let t1 = f32x8::splat(2.0) * l_norm - t2;

        // Compute R, G, B channels in [0,1] via piecewise blend
        let r_norm = channel_simd(h_norm, t1, t2, 1.0 / 3.0);
        let g_norm = channel_simd(h_norm, t1, t2, 0.0);
        let b_norm = channel_simd(h_norm, t1, t2, -1.0 / 3.0);

        // Achromatic guard: when s == 0, force r=g=b=l (in [0,1])
        let mask_achromatic = s_norm.simd_eq(zero);
        let achromatic_val = l_norm; // l in [0,1], matches r=g=b when s==0
        let r_val = mask_achromatic.blend(achromatic_val, r_norm);
        let g_val = mask_achromatic.blend(achromatic_val, g_norm);
        let b_val = mask_achromatic.blend(achromatic_val, b_norm);

        // Scale to [0,255]
        let r255 = r_val * s255;
        let g255 = g_val * s255;
        let b255 = b_val * s255;

        let r_arr = r255.to_array();
        let g_arr = g255.to_array();
        let b_arr = b255.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::hsl::rgb([hsl[i][0] as f64, hsl[i][1] as f64, hsl[i][2] as f64]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
