//! SIMD CMYK batch-route correctness tests (issue #72).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! f64 reference result per pixel via `rgb::cmyk`, then calls the SIMD
//! batch function `simd_cmyk::rgb_to_cmyk_batch` (wide::f32x8, producing
//! f32 output) and asserts every lane matches the scalar output within
//! a documented absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! f32 has ~7 decimal digits of precision vs f64's ~15. The CMYK
//! calculation involves division by `(1-k)` which can be very small for
//! near-black pixels — though the black guard (mask-blend to zero when
//! k==1) eliminates the degenerate case, and the numerators also scale
//! proportionally with `(1-k)`, limiting amplification. The worst-case
//! f32→f64 gap after scaling by 100 remains below 1e-3.
//!
//! - c,m,y,k (0–100): absolute tolerance ≤ 1e-3

use color_convert_rs::rgb;
use color_convert_rs::simd_cmyk;

const CMYK_TOLERANCE: f64 = 1e-3;

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

/// Behavior 1: `rgb_to_cmyk_batch` (f32x8 SIMD) must match the scalar
/// `rgb::cmyk` (f64 → [f64; 4]) within CMYK_TOLERANCE (1e-3) for
/// batches including non-multiples of the SIMD lane width (8).
#[test]
fn rgb_to_cmyk_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 4]> = pixels.iter().map(|&p| rgb::cmyk(p)).collect();
        let simd_result = simd_cmyk::rgb_to_cmyk_batch(&pixels);

        assert_eq!(
            simd_result.len(),
            scalar.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
            // Compile-time type check: simd_val MUST be [f32; 4], not [f64; 4].
            let _f32_check: [f32; 4] = *simd_val;

            for chan in 0..4 {
                let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
                let chan_name = ["c", "m", "y", "k"][chan];
                assert!(
                    diff <= CMYK_TOLERANCE,
                    "pixel {i} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    CMYK_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 2: Edge cases — pure black (k=100, c=m=y=0), pure white
/// (all 0), and primary colors.
#[test]
fn rgb_to_cmyk_batch_edge_cases() {
    let test_pixels: Vec<[u8; 3]> = vec![
        // Pure black → k=100, c=m=y=0 (JS `|| 0` guard)
        [0, 0, 0],
        // Pure white → all 0
        [255, 255, 255],
        // Primary colors
        [255, 0, 0],   // red    → c=0, m=100, y=100, k=0
        [0, 255, 0],   // green  → c=100, m=0, y=100, k=0
        [0, 0, 255],   // blue   → c=100, m=100, y=0, k=0
        // Secondary colors
        [255, 255, 0], // yellow → c=0, m=0, y=100, k=0
        [0, 255, 255], // cyan   → c=100, m=0, y=0, k=0
        [255, 0, 255], // magenta→ c=0, m=100, y=0, k=0
        // Near-black (should NOT trigger the k==1 guard)
        [1, 1, 1],
        [1, 0, 0],
        // Near-white
        [254, 254, 254],
        [254, 255, 255],
    ];

    let scalar: Vec<[f64; 4]> = test_pixels.iter().map(|&p| rgb::cmyk(p)).collect();
    let simd_result = simd_cmyk::rgb_to_cmyk_batch(&test_pixels);

    assert_eq!(simd_result.len(), scalar.len(), "batch size mismatch");

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
        let _f32_check: [f32; 4] = *simd_val;

        for chan in 0..4 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            let chan_name = ["c", "m", "y", "k"][chan];
            assert!(
                diff <= CMYK_TOLERANCE,
                "pixel {i} [{},{},{}] chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                test_pixels[i][0],
                test_pixels[i][1],
                test_pixels[i][2],
                simd_val[chan],
                scalar_val[chan],
                diff,
                CMYK_TOLERANCE,
            );
        }
    }
}
