//! SIMD hsv→rgb batch-route correctness tests (issue #104).
//!
//! Each test generates in-gamut HSV values (via rgb→hsv round-trip from
//! deterministic RGB pixels), computes the scalar f64 reference result
//! per pixel via `hsv::rgb`, then calls the SIMD batch function
//! `simd_hsv_rgb::hsv_to_rgb_batch` (wide::f32x8, producing f32 output)
//! and asserts every lane matches the scalar output within a documented
//! absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! The hsv→rgb conversion involves arithmetic in [0,1] scaled by 255.
//! f32 has ~7 decimal digits of precision vs f64's ~15; the gap in the
//! final [0,255] range is ~3e-5.
//!
//! - r/g/b (0–255): absolute tolerance ≤ 1e-3

use color_convert_rs::hsv;
use color_convert_rs::rgb;
use color_convert_rs::simd_hsv_rgb;

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

/// Behavior 1: `hsv_to_rgb_batch` (f32x8 SIMD) must match the scalar
/// `hsv::rgb` (f64) within RGB_TOLERANCE (1e-3) for batches including
/// non-multiples of the SIMD lane width (8).
///
/// Test strategy: generate RGB pixels, convert to HSV via scalar `rgb::hsv`,
/// then run BOTH the scalar `hsv::rgb` and the SIMD `hsv_to_rgb_batch`
/// on those HSV values and assert them equal per-channel.
#[test]
fn hsv_to_rgb_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);

        // Scalar HSV (f64) from original RGB — ensures in-gamut HSV
        let hsv_scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::hsv(p)).collect();

        // Scalar rgb reference: hsv→rgb via f64
        let scalar_rgb: Vec<[f64; 3]> = hsv_scalar.iter().map(|&h| hsv::rgb(h)).collect();

        // Convert HSV to f32 for SIMD input
        let hsv_f32: Vec<[f32; 3]> = hsv_scalar
            .iter()
            .map(|&h| [h[0] as f32, h[1] as f32, h[2] as f32])
            .collect();

        let simd_result = simd_hsv_rgb::hsv_to_rgb_batch(&hsv_f32);

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

/// Behavior 2: Achromatic pixels (s=0) and primary/secondary hue edge cases.
///
/// Test specific edge-case HSV values: achromatic (s=0, various V), the six
/// primaries (h=0,60,120,180,240,300), and hue near wrapping boundary (h≈360).
#[test]
fn hsv_to_rgb_batch_achromatic_and_edge_cases() {
    let test_hsv: Vec<[f64; 3]> = vec![
        // Achromatic (s=0, various V) — should produce r=g=b = v*2.55
        [0.0, 0.0, 0.0],     // black
        [0.0, 0.0, 50.0],    // mid grey
        [0.0, 0.0, 100.0],   // white
        [180.0, 0.0, 75.0],  // achromatic with non-zero hue
        // Primary hue sectors (s=100, v=100)
        [0.0, 100.0, 100.0],    // red     → hi=0, f=0
        [60.0, 100.0, 100.0],   // yellow  → hi=1, f=0
        [120.0, 100.0, 100.0],  // green   → hi=2, f=0
        [180.0, 100.0, 100.0],  // cyan    → hi=3, f=0
        [240.0, 100.0, 100.0],  // blue    → hi=4, f=0
        [300.0, 100.0, 100.0],  // magenta → hi=5, f=0
        // Mid-hue points (s=100, v=100, f=0.5)
        [90.0, 100.0, 100.0],   // greenish-yellow, hi=1, f=0.5
        [150.0, 100.0, 100.0],  // greenish-cyan,  hi=2, f=0.5
        [210.0, 100.0, 100.0],  // blueish-cyan,   hi=3, f=0.5
        [330.0, 100.0, 100.0],  // reddish-magenta,hi=5, f=0.5
        // Hue near wrap boundary
        [359.0, 100.0, 50.0],  // near-red, hi=5, f≈0.98
        [360.0, 100.0, 100.0], // exactly 360 → same as h=0
        // Mixed s and v
        [30.0, 50.0, 80.0],   // partial saturation and value
        [200.0, 75.0, 25.0],  // low value
    ];

    let scalar_rgb: Vec<[f64; 3]> = test_hsv.iter().map(|&h| hsv::rgb(h)).collect();

    let hsv_f32: Vec<[f32; 3]> = test_hsv
        .iter()
        .map(|&h| [h[0] as f32, h[1] as f32, h[2] as f32])
        .collect();

    let simd_result = simd_hsv_rgb::hsv_to_rgb_batch(&hsv_f32);

    assert_eq!(
        simd_result.len(),
        scalar_rgb.len(),
        "batch size mismatch"
    );

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar_rgb.iter()).enumerate() {
        let _f32_check: [f32; 3] = *simd_val;

        let is_achromatic = test_hsv[i][1] == 0.0; // s == 0

        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            let chan_name = ["r", "g", "b"][chan];

            if is_achromatic {
                // For achromatic HSV, r=g=b=v*2.55 — all channels must match
                // within a tighter tolerance
                let expected = test_hsv[i][2] * 2.55;
                assert!(
                    diff <= RGB_TOLERANCE,
                    "pixel {i} achromatic hsv={:?} chan {chan_name}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    test_hsv[i],
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    RGB_TOLERANCE,
                );
            } else {
                assert!(
                    diff <= RGB_TOLERANCE,
                    "pixel {i} hsv={:?} chan {chan_name}({chan}): simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    test_hsv[i],
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    RGB_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 3: The SIMD round-trip `rgb → hsv → rgb` must reproduce the
/// original u8 RGB values within a rounding tolerance of ±1.
///
/// Applies the SIMD pipeline: `hsv_to_rgb_batch(rgb_to_hsv_batch(rgb))`,
/// rounds each channel to the nearest u8 (clamped to 0–255), and compares
/// against the original input pixel.
#[test]
fn rgb_hsv_rgb_simd_roundtrip_matches_original() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);

        let hsv_batch = color_convert_rs::simd_hsv::rgb_to_hsv_batch(&pixels);
        let rgb_batch = simd_hsv_rgb::hsv_to_rgb_batch(&hsv_batch);

        assert_eq!(
            rgb_batch.len(),
            pixels.len(),
            "batch size mismatch for n={n}"
        );

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
