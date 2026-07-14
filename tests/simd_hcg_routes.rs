//! SIMD HCG batch-route correctness tests (issue #87).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! f64 reference result per pixel via `rgb::hcg`, then calls the SIMD
//! batch function `simd_hcg::rgb_to_hcg_batch` (wide::f32x8, producing
//! f32 output) and asserts every lane matches the scalar output within
//! a documented absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! f32 has ~7 decimal digits of precision vs f64's ~15. The hue
//! calculation involves division by chroma (which can be as small as
//! 1/255 ≈ 0.004), amplifying the initial f32 representation error of
//! the normalised channel values (~1e-8) by up to ~250×. After scaling
//! by 360, the worst-case f32→f64 gap in hue is ~3e-4. Chroma and
//! grayscale involve simpler arithmetic with less amplification.
//!
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - c (0–100): absolute tolerance ≤ 1e-3
//! - g (0–100): absolute tolerance ≤ 1e-3

use color_convert_rs::rgb;
use color_convert_rs::simd_hcg;

const HCG_TOLERANCE: f64 = 1e-3;

// ── Deterministic PRNG (mulberry32, seed=42) ─────────────────────────
fn mulberry32(state: &mut u32) -> f64 {
    *state = state.wrapping_add(0x6d2b79f5);
    let mut t = *state;
    t = (t ^ (t >> 15)).wrapping_mul(1 | t);
    t = (t ^ (t >> 7)).wrapping_mul(61 | t);
    t ^= t >> 14;
    (t as f64) / 4_294_967_296.0
}

fn generate_rgb_pixels(n: usize) -> Vec<[u8; 3]> {
    let mut state: u32 = 42;
    let mut pixels = Vec::with_capacity(n);
    for _ in 0..n {
        let r = (mulberry32(&mut state) * 256.0) as u8;
        let g = (mulberry32(&mut state) * 256.0) as u8;
        let b = (mulberry32(&mut state) * 256.0) as u8;
        pixels.push([r, g, b]);
    }
    pixels
}

/// Behavior 1: `rgb_to_hcg_batch` (f32x8 SIMD) must match the scalar
/// `rgb::hcg` (f64) within HCG_TOLERANCE (1e-3) for batches including
/// non-multiples of the SIMD lane width (8).
#[test]
fn rgb_to_hcg_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::hcg(p)).collect();
        let simd_result = simd_hcg::rgb_to_hcg_batch(&pixels);

        assert_eq!(
            simd_result.len(),
            scalar.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
            // Compile-time type check: simd_val MUST be [f32; 3], not [f64; 3].
            let _f32_check: [f32; 3] = *simd_val;

            for chan in 0..3 {
                let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
                let chan_name = ["h", "c", "g"][chan];
                assert!(
                    diff <= HCG_TOLERANCE,
                    "pixel {i} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    HCG_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 2: Achromatic pixels (r==g==b) must produce h=0. Also test
/// primary-color edge cases, black (0,0,0), and white (255,255,255) which
/// has chroma=0 and exercises the chroma==1 grayscale-guard case.
#[test]
fn rgb_to_hcg_batch_achromatic_and_edge_cases() {
    let test_pixels: Vec<[u8; 3]> = vec![
        // Achromatic (various grey levels)
        [0, 0, 0],       // black  → h=0, c=0, g=0
        [128, 128, 128], // grey   → h=0, c=0, g≈50
        [255, 255, 255], // white  → h=0, c=0, g=100
        // Primary colours
        [255, 0, 0], // red    → h=0,   c=100, g=0
        [0, 255, 0], // green  → h=120, c=100, g=0
        [0, 0, 255], // blue   → h=240, c=100, g=0
        // Secondary colours
        [255, 255, 0], // yellow → h=60,  c=100, g=0
        [0, 255, 255], // cyan   → h=180, c=100, g=0
        [255, 0, 255], // magenta→ h=300, c=100, g=0
        // Pure-color with some gray (chroma=100, grayscale≠0 — not possible
        // from rgb; but test saturated + near-saturated edge cases)
        [1, 0, 0],       // near-black red — chroma=1/255≈0.004, g≈0.004
        [254, 255, 255], // near-white with hint — chroma=1/255≈0.004
        // Chroma==1 edge case (white has chroma=0, but test pure colors
        // where max=255, min=0): chroma=1.0
        [255, 0, 64],  // mixed, max=255 min=0 → chroma=1.0, grayscale=0
    ];

    let scalar: Vec<[f64; 3]> = test_pixels.iter().map(|&p| rgb::hcg(p)).collect();
    let simd_result = simd_hcg::rgb_to_hcg_batch(&test_pixels);

    assert_eq!(simd_result.len(), scalar.len(), "batch size mismatch");

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
        let _f32_check: [f32; 3] = *simd_val;

        let r = test_pixels[i][0];
        let g = test_pixels[i][1];
        let b_val = test_pixels[i][2];
        let is_achromatic = r == g && g == b_val;

        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            let chan_name = ["h", "c", "g"][chan];

            if is_achromatic && chan == 0 {
                // hue must be zero for achromatic pixels
                assert!(
                    simd_val[chan].abs() < 1e-6,
                    "pixel {i} achromatic [{r},{g},{b_val}] chan {chan_name}: simd(f32)={} must be ~0",
                    simd_val[chan],
                );
            } else {
                assert!(
                    diff <= HCG_TOLERANCE,
                    "pixel {i} [{r},{g},{b_val}] chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    HCG_TOLERANCE,
                );
            }
        }
    }
}
