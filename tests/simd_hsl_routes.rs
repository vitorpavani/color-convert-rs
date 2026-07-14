//! SIMD HSL batch-route correctness tests (issue #58).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! f64 reference result per pixel via `rgb::hsl`, then calls the SIMD
//! batch function `simd_hsl::rgb_to_hsl_batch` (wide::f32x8, producing
//! f32 output) and asserts every lane matches the scalar output within
//! a documented absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! f32 has ~7 decimal digits of precision vs f64's ~15. The hue
//! calculation involves division by delta (which can be as small as
//! 1/255 ≈ 0.004), amplifying the initial f32 representation error of
//! the normalised channel values (~1e-8) by up to ~250×. After scaling
//! by 60, the worst-case f32→f64 gap in hue is ~3e-4. Sat and light
//! involve simpler arithmetic with less amplification.
//!
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - s (0–100): absolute tolerance ≤ 1e-3
//! - l (0–100): absolute tolerance ≤ 1e-3

use color_convert_rs::hsl;
use color_convert_rs::rgb;
use color_convert_rs::simd_hsl;

const HSL_TOLERANCE: f64 = 1e-3;
/// Tolerance for RGB channel values (0–255) when comparing SIMD f32 against scalar f64.
/// The `channel` function's piecewise linear interpolation involves values in [0,1]
/// scaled by 255; f32 vs f64 precision gap is ~1e-7 in [0,1] → ~3e-5 in [0,255],
/// well within this tolerance.
const RGB_TOLERANCE: f64 = 1e-3;

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

/// Behavior 1: `rgb_to_hsl_batch` (f32x8 SIMD) must match the scalar
/// `rgb::hsl` (f64) within HSL_TOLERANCE (1e-3) for batches including
/// non-multiples of the SIMD lane width (8).
#[test]
fn rgb_to_hsl_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::hsl(p)).collect();
        let simd_result = simd_hsl::rgb_to_hsl_batch(&pixels);

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
                let chan_name = ["h", "s", "l"][chan];
                assert!(
                    diff <= HSL_TOLERANCE,
                    "pixel {i} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    HSL_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 2: Achromatic pixels (r==g==b) must produce h=0, s=0.
/// Also test primary-color edge cases.
#[test]
fn rgb_to_hsl_batch_achromatic_and_edge_cases() {
    // Build a diverse batch of edge-case pixels including achromatic,
    // primaries, and extreme values.
    let test_pixels: Vec<[u8; 3]> = vec![
        // Achromatic (various grey levels)
        [0, 0, 0],       // black  → h=0, s=0, l=0
        [128, 128, 128], // grey   → h=0, s=0, l≈50
        [255, 255, 255], // white  → h=0, s=0, l=100
        // Primary colours
        [255, 0, 0], // red    → h=0,   s=100, l=50
        [0, 255, 0], // green  → h=120, s=100, l=50
        [0, 0, 255], // blue   → h=240, s=100, l=50
        // Secondary colours
        [255, 255, 0], // yellow → h=60,  s=100, l=50
        [0, 255, 255], // cyan   → h=180, s=100, l=50
        [255, 0, 255], // magenta→ h=300, s=100, l=50
        // Edge cases
        [1, 0, 0],       // near-black red
        [254, 255, 255], // near-white with hint
    ];

    let scalar: Vec<[f64; 3]> = test_pixels.iter().map(|&p| rgb::hsl(p)).collect();
    let simd_result = simd_hsl::rgb_to_hsl_batch(&test_pixels);

    assert_eq!(simd_result.len(), scalar.len(), "batch size mismatch");

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
        let _f32_check: [f32; 3] = *simd_val;

        // Achromatic pixels: h and s must be exactly 0 (within f32 epsilon)
        let r = test_pixels[i][0];
        let g = test_pixels[i][1];
        let b = test_pixels[i][2];
        let is_achromatic = r == g && g == b;

        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            let chan_name = ["h", "s", "l"][chan];

            if is_achromatic && chan <= 1 {
                // hue and saturation must be zero (within tighter tolerance)
                assert!(
                    simd_val[chan].abs() < 1e-6,
                    "pixel {i} achromatic [{r},{g},{b}] chan {chan_name}: simd(f32)={} must be ~0",
                    simd_val[chan],
                );
            } else {
                assert!(
                    diff <= HSL_TOLERANCE,
                    "pixel {i} [{r},{g},{b}] chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    HSL_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 3: `hsl_to_rgb_batch` (f32x8 SIMD) must match the scalar
/// `hsl::rgb` (f64) within RGB_TOLERANCE (1e-3) for batches including
/// non-multiples of the SIMD lane width (8).
///
/// Test strategy: generate RGB pixels, convert to HSL via scalar `rgb::hsl`,
/// then run BOTH the scalar `hsl::rgb` and the SIMD `hsl_to_rgb_batch`
/// on those HSL values and assert them equal per-channel.
#[test]
fn hsl_to_rgb_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);

        // Scalar HSL (f64) from original RGB
        let hsl_scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::hsl(p)).collect();

        // Scalar rgb reference: hsl→rgb via f64
        let scalar_rgb: Vec<[f64; 3]> = hsl_scalar.iter().map(|&h| hsl::rgb(h)).collect();

        // Convert HSL to f32 for SIMD input
        let hsl_f32: Vec<[f32; 3]> = hsl_scalar
            .iter()
            .map(|&h| [h[0] as f32, h[1] as f32, h[2] as f32])
            .collect();

        let simd_result = simd_hsl::hsl_to_rgb_batch(&hsl_f32);

        assert_eq!(
            simd_result.len(),
            scalar_rgb.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar_rgb.iter()).enumerate() {
            // Compile-time type check: simd_val MUST be [f32; 3]
            let _f32_check: [f32; 3] = *simd_val;

            for chan in 0..3 {
                let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
                let chan_name = ["r", "g", "b"][chan];
                assert!(
                    diff <= RGB_TOLERANCE,
                    "pixel {i} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    RGB_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 4: The simd round-trip `rgb → hsl → rgb` must reproduce the
/// original u8 RGB values within a rounding tolerance of ±1.
///
/// Applies the SIMD pipeline: `hsl_to_rgb_batch(rgb_to_hsl_batch(rgb))`,
/// rounds each channel to the nearest u8 (clamped to 0–255), and compares
/// against the original input pixel.
#[test]
fn rgb_hsl_rgb_simd_roundtrip_matches_original() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);

        let hsl_batch = simd_hsl::rgb_to_hsl_batch(&pixels);
        let rgb_batch = simd_hsl::hsl_to_rgb_batch(&hsl_batch);

        assert_eq!(rgb_batch.len(), pixels.len(), "batch size mismatch for n={n}");

        for (i, (simd_rgb, orig)) in rgb_batch.iter().zip(pixels.iter()).enumerate() {
            for chan in 0..3 {
                let rounded = (simd_rgb[chan].round() as i32).clamp(0, 255) as u8;
                let diff = (rounded as i32 - orig[chan] as i32).abs();
                let chan_name = ["r", "g", "b"][chan];
                assert!(
                    diff <= 1,
                    "pixel {i} chan {chan_name}({chan}): roundtrip={} original={} diff={} > 1 (simd_raw={})",
                    rounded,
                    orig[chan],
                    diff,
                    simd_rgb[chan],
                );
            }
        }
    }
}
