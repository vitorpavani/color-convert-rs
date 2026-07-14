//! SIMD Apple batch-route correctness tests (issue #86).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! f64 reference result per pixel via `rgb::apple`, then calls the SIMD
//! batch function `simd_apple::rgb_to_apple_batch` (wide::f32x8, producing
//! f32 output) and asserts every lane matches the scalar output within
//! a documented absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! The apple route is `u8_value * 257.0` per channel. All inputs are u8
//! (0–255), so outputs are integer multiples of 257, max 65535. Both f32
//! (24-bit mantissa, 16,777,216 range) and f64 represent these values
//! exactly. Tolerance is effectively 0.0; 1e-6 accounts for f32→f64 cast
//! noise in the comparison pipeline.

use color_convert_rs::rgb;
use color_convert_rs::simd_apple;

const APPLE_TOLERANCE: f64 = 1e-6;

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

/// Behavior 1: `rgb_to_apple_batch` (f32x8 SIMD) must match the scalar
/// `rgb::apple` (f64) within APPLE_TOLERANCE for batches including
/// non-multiples of the SIMD lane width (8).
#[test]
fn rgb_to_apple_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::apple(p)).collect();
        let simd_result = simd_apple::rgb_to_apple_batch(&pixels);

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
                let chan_name = ["R", "G", "B"][chan];
                assert!(
                    diff <= APPLE_TOLERANCE,
                    "pixel {i} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    APPLE_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 2: Edge-case and primary-color pixels must produce exact
/// known values.
#[test]
fn rgb_to_apple_batch_edge_cases() {
    let test_pixels: Vec<[u8; 3]> = vec![
        // Black → all zeros
        [0, 0, 0],
        // White → all 65535
        [255, 255, 255],
        // Mid-grey: 128*257 = 32896
        [128, 128, 128],
        // Primary colours
        [255, 0, 0],   // red    → [65535, 0, 0]
        [0, 255, 0],   // green  → [0, 65535, 0]
        [0, 0, 255],   // blue   → [0, 0, 65535]
        // Secondary colours
        [255, 255, 0], // yellow → [65535, 65535, 0]
        [0, 255, 255], // cyan   → [0, 65535, 65535]
        [255, 0, 255], // magenta→ [65535, 0, 65535]
        // Near-edge values
        [1, 0, 0],     // 1*257 = 257
        [254, 255, 255], // 254*257=65278
    ];

    let scalar: Vec<[f64; 3]> = test_pixels.iter().map(|&p| rgb::apple(p)).collect();
    let simd_result = simd_apple::rgb_to_apple_batch(&test_pixels);

    assert_eq!(simd_result.len(), scalar.len(), "batch size mismatch");

    let expected_values: [([u8; 3], [f64; 3]); 11] = [
        ([0, 0, 0], [0.0, 0.0, 0.0]),
        ([255, 255, 255], [65535.0, 65535.0, 65535.0]),
        ([128, 128, 128], [32896.0, 32896.0, 32896.0]),
        ([255, 0, 0], [65535.0, 0.0, 0.0]),
        ([0, 255, 0], [0.0, 65535.0, 0.0]),
        ([0, 0, 255], [0.0, 0.0, 65535.0]),
        ([255, 255, 0], [65535.0, 65535.0, 0.0]),
        ([0, 255, 255], [0.0, 65535.0, 65535.0]),
        ([255, 0, 255], [65535.0, 0.0, 65535.0]),
        ([1, 0, 0], [257.0, 0.0, 0.0]),
        ([254, 255, 255], [65278.0, 65535.0, 65535.0]),
    ];

    for (_idx, ((expected_input, expected_output), (simd_val, scalar_val))) in
        expected_values
            .iter()
            .zip(simd_result.iter().zip(scalar.iter()))
            .enumerate()
    {
        let _f32_check: [f32; 3] = *simd_val;

        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            let chan_name = ["R", "G", "B"][chan];
            // Also verify against known exact expected value
            let expected_diff = (simd_val[chan] as f64 - expected_output[chan]).abs();
            assert!(
                diff <= APPLE_TOLERANCE,
                "pixel [{},{},{}] chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                expected_input[0], expected_input[1], expected_input[2],
                simd_val[chan],
                scalar_val[chan],
                diff,
                APPLE_TOLERANCE,
            );
            assert!(
                expected_diff <= APPLE_TOLERANCE,
                "pixel [{},{},{}] chan {chan_name}({chan}): simd(f32)={} expected={} diff={:.2e} > tol={:.2e}",
                expected_input[0], expected_input[1], expected_input[2],
                simd_val[chan],
                expected_output[chan],
                expected_diff,
                APPLE_TOLERANCE,
            );
        }
    }
}
