//! SIMD batch-route correctness test for oklab→rgb (inverse) — issue #100.
//!
//! Generates deterministic in-gamut Oklab inputs by first producing RGB
//! pixels, then converting them to Oklab via `rgb::oklab`, then tests the
//! SIMD batch oklab→rgb function against the scalar `oklab::rgb` reference.
//!
//! ## Tolerance
//!
//! The inverse Oklab route chains: inverse Oklab matrix (L,a,b → LMS) →
//! cube (`powi(3)`) → inverse LMS matrix (→ linear sRGB) → forward sRGB
//! gamma (`powf(1/2.4)`) → ×255.  This dual matrix + cube³ + gamma chain
//! through f32 precision vs f64 reference accumulates detectable rounding
//! differences per component.  Absolute tolerance: **1e-2** per channel
//! (output range [0, 255]), sufficient to catch real bugs (wrong coefficient,
//! wrong branch, wrong matrix) while accepting the inherent f32→f64 gap
//! through the full Oklab→RGB pipeline.

use color_convert_rs::oklab;
use color_convert_rs::rgb;
use color_convert_rs::simd_oklab_rgb;

/// Tolerance for f32-SIMD `oklab→rgb` vs scalar f64 `oklab::rgb`.
///
/// Output range is [0, 255] per channel. The dual matrix + cube³ +
/// forward sRGB gamma `powf(1/2.4)` + ×255 chain through f32 vs f64
/// precision accumulates rounding in every stage.
const OKLAB_RGB_TOLERANCE: f64 = 1e-2;

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

/// Behavior: `oklab_to_rgb_batch` (f32x8 SIMD) must match the scalar
/// `oklab::rgb` (f64) within OKLAB_RGB_TOLERANCE (1e-2) for batches
/// including non-multiples of the SIMD lane width (8).
#[test]
fn oklab_to_rgb_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let rgb_pixels = generate_rgb_pixels(n);

        // Generate in-gamut Oklab inputs via the forward (scalar) path
        let oklab_input: Vec<[f32; 3]> = rgb_pixels
            .iter()
            .map(|&p| {
                let o = rgb::oklab(p); // [f64; 3]
                [o[0] as f32, o[1] as f32, o[2] as f32]
            })
            .collect();

        // Scalar reference: oklab→rgb via f64 scalar
        let scalar: Vec<[f64; 3]> = rgb_pixels
            .iter()
            .map(|&p| {
                let o = rgb::oklab(p); // f64 Oklab
                oklab::rgb([o[0], o[1], o[2]]) // oklab::rgb takes [f64; 3]
            })
            .collect();

        // SIMD result — call the not-yet-existing function
        let simd_result = simd_oklab_rgb::oklab_to_rgb_batch(&oklab_input);

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
                assert!(
                    diff <= OKLAB_RGB_TOLERANCE,
                    "pixel {i} n={n} chan={chan}: SIMD={}, scalar={}, diff={:.6e} > tol={:.0e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    OKLAB_RGB_TOLERANCE
                );
            }
        }
    }
}
