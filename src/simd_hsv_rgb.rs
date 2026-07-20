//! CPU-SIMD batch conversion for hsv→rgb using mask-blend hue-sector
//! selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## hsv→rgb ([`hsv_to_rgb_batch`])
//!
//! The scalar reference is [`crate::hsv::rgb`], which normalises HSV to
//! [0,1] (with h scaled to [0,6] sectors), computes p/q/t channel blends
//! from s, v, and fractional hue f, then selects per-channel RGB via a
//! 6-way `match` on the hue sector hi ∈ 0..=5. This SIMD path computes
//! all 6 candidate triples concurrently and selects via mask-blend,
//! avoiding per-pixel branching.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//! The arithmetic in [0,1] scaled by 255 has an f32→f64 gap of ~3e-5
//! in the final [0,255] range.
//! - r/g/b (0–255): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hsv_rgb_routes.rs`.

use wide::f32x8;

/// Public wrapper: auto-selects serial or parallel based on input size.
///
/// Delegates to [`crate::simd_parallel::auto_batch`] which chooses serial
/// SIMD for ≤ 4096 pixels and multi-core rayon for larger batches.
///
/// # Mask-blend strategy
///
/// The scalar reference uses a 6-way `match hi` to assign (r,g,b). We:
/// 1. Compute `p`, `q`, `t`, and `v255` for all 8 lanes simultaneously.
/// 2. Build 6 masks `mask_hi_0` through `mask_hi_5` via `simd_eq` on the
///    floor of h/60 wrapped with `.rem_euclid(6)`.
/// 3. For each channel, start with a sensible default (hi=5) and blend
///    through hi=4…0. Since masks are mutually exclusive, order doesn't
///    matter — we blend hi=0 last for r/g and use hi=5 as the initial
///    default.
///
/// The hue wrap (h=360 → h/60=6 → hi=0, f=0) is handled by
/// `hi_raw.rem_euclid(6)` and computing f from hi_raw before wrapping.
pub fn hsv_to_rgb_batch(hsv: &[[f32; 3]]) -> Vec<[f32; 3]> {
    crate::simd_parallel::auto_batch(hsv, hsv_to_rgb_batch_serial)
}

/// Serial single-core SIMD implementation — processes 8 pixels at a time.
pub(crate) fn hsv_to_rgb_batch_serial(hsv: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let n = hsv.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let inv60 = f32x8::splat(1.0 / 60.0);
    let inv100 = f32x8::splat(1.0 / 100.0);
    let s255 = f32x8::splat(255.0);
    let one = f32x8::splat(1.0);
    let zero = f32x8::splat(0.0);
    let six = f32x8::splat(6.0);

    while i + 7 < n {
        // Load 8 HSV triples into SoA SIMD lanes
        let h = f32x8::new([
            hsv[i][0],
            hsv[i + 1][0],
            hsv[i + 2][0],
            hsv[i + 3][0],
            hsv[i + 4][0],
            hsv[i + 5][0],
            hsv[i + 6][0],
            hsv[i + 7][0],
        ]);
        let s = f32x8::new([
            hsv[i][1],
            hsv[i + 1][1],
            hsv[i + 2][1],
            hsv[i + 3][1],
            hsv[i + 4][1],
            hsv[i + 5][1],
            hsv[i + 6][1],
            hsv[i + 7][1],
        ]);
        let v = f32x8::new([
            hsv[i][2],
            hsv[i + 1][2],
            hsv[i + 2][2],
            hsv[i + 3][2],
            hsv[i + 4][2],
            hsv[i + 5][2],
            hsv[i + 6][2],
            hsv[i + 7][2],
        ]);

        // Normalise: h to [0,6) sectors, s and v to [0,1]
        let h_norm = h * inv60;
        let s_norm = s * inv100;
        let v_norm = v * inv100;

        // Hue sector and fractional part
        // hi_raw ∈ [0, 6]; rem_euclid(6) wraps 6→0 for h=360 boundary
        let hi_raw = h_norm.floor();
        let f = h_norm - hi_raw; // f ∈ [0, 1)
        let hi = hi_raw.rem_euclid(six); // hi ∈ [0, 5]

        // Scale v to [0,255] and compute p, q, t intermediates
        let v255 = v_norm * s255;
        let p = v255 * (one - s_norm); // 255*v*(1-s)
        let q = v255 * (one - s_norm * f); // 255*v*(1-s*f)
        let t = v255 * (one - s_norm * (one - f)); // 255*v*(1-s*(1-f))

        // Build masks for each hue sector
        let one_f = f32x8::splat(1.0);
        let two = f32x8::splat(2.0);
        let three = f32x8::splat(3.0);
        let four = f32x8::splat(4.0);
        let five = f32x8::splat(5.0);

        let m0 = hi.simd_eq(zero);
        let m1 = hi.simd_eq(one_f);
        let m2 = hi.simd_eq(two);
        let m3 = hi.simd_eq(three);
        let m4 = hi.simd_eq(four);
        let m5 = hi.simd_eq(five);

        // ── R channel: hi=0→v255, hi=1→q, hi=2→p, hi=3→p, hi=4→t, hi=5→v255
        let mut r = v255;
        r = m1.blend(q, r);
        r = m2.blend(p, r);
        r = m3.blend(p, r);
        r = m4.blend(t, r);
        r = m5.blend(v255, r);

        // ── G channel: hi=0→t, hi=1→v255, hi=2→v255, hi=3→q, hi=4→p, hi=5→p
        let mut g = p;
        g = m0.blend(t, g);
        g = m1.blend(v255, g);
        g = m2.blend(v255, g);
        g = m3.blend(q, g);
        g = m4.blend(p, g);
        g = m5.blend(p, g);

        // ── B channel: hi=0→p, hi=1→p, hi=2→t, hi=3→v255, hi=4→v255, hi=5→q
        let mut b_chan = q;
        b_chan = m0.blend(p, b_chan);
        b_chan = m1.blend(p, b_chan);
        b_chan = m2.blend(t, b_chan);
        b_chan = m3.blend(v255, b_chan);
        b_chan = m4.blend(v255, b_chan);
        b_chan = m5.blend(q, b_chan);

        // Write results
        let r_arr = r.to_array();
        let g_arr = g.to_array();
        let b_arr = b_chan.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32.
    while i < n {
        let f64_result = crate::hsv::rgb([hsv[i][0] as f64, hsv[i][1] as f64, hsv[i][2] as f64]);
        result.push([
            f64_result[0] as f32,
            f64_result[1] as f32,
            f64_result[2] as f32,
        ]);
        i += 1;
    }

    result
}
