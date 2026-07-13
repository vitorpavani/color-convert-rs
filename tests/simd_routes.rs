//! SIMD batch-route correctness tests (issue #20).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! reference result per pixel, then calls the SIMD batch function and
//! asserts every lane matches the scalar output exactly (tolerance 0.0,
//! because each SIMD lane performs the same sequence of IEEE 754 f64 ops
//! as the scalar route on the same pixel).
//!
//! Tolerance: 0.0 (bit-identical). If simd and scalar disagree, the simd
//! implementation is buggy — see `src/simd.rs` module doc.

use color_convert_rs::rgb;
use color_convert_rs::simd;

// ── Deterministic PRNG (mulberry32, seed=42) ─────────────────────────
// Mirrors benchmarks/js/bench.mjs for reproducible input generation.
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

/// Behavior 1: `rgb_to_xyz_batch` must produce bit-identical results to
/// calling `rgb::xyz` on each pixel in the batch, for batches of various
/// sizes including non-multiples of the SIMD lane width (4).
#[test]
fn rgb_to_xyz_batch_matches_scalar() {
    // Test multiple batch sizes to exercise remainder paths
    for n in [1, 3, 4, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::xyz(p)).collect();
        let simd_result = simd::rgb_to_xyz_batch(&pixels);

        assert_eq!(
            simd_result.len(),
            scalar.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate()
        {
            for chan in 0..3 {
                assert!(
                    (simd_val[chan] - scalar_val[chan]).abs() <= 0.0,
                    "pixel {i} channel {chan}: simd={} scalar={} diff={}",
                    simd_val[chan],
                    scalar_val[chan],
                    (simd_val[chan] - scalar_val[chan]).abs()
                );
            }
        }
    }
}
