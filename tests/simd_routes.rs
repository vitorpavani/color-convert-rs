//! SIMD batch-route correctness tests (issue #20 → #51).
//!
//! Each test generates a deterministic pixel batch, computes the scalar
//! f64 reference result per pixel, then calls the SIMD batch function
//! (now `wide::f32x8`, producing f32 output) and asserts every lane
//! matches the scalar output within a documented absolute tolerance.
//!
//! ## Tolerance (f32 vs f64)
//!
//! f32 has ~7 decimal digits of precision vs f64's ~15. The sRGB gamma
//! expansion (powf 2.4) and CIE LAB transfer (cbrt) amplify rounding
//! differences.  The tolerances below are chosen to catch real algorithmic
//! bugs (wrong matrix coefficient, wrong branch) while accepting the
//! inherent f32→f64 gap.
//!
//! - rgb→xyz: 5e-4 absolute per channel (XYZ ∈ [0, 100], f32 ulp ≈ 1e-6)
//! - xyz→lab: 1e-3 absolute per channel (L ∈ [0, 100], a,b ∈ [-100, 100])

use color_convert_rs::rgb;
use color_convert_rs::simd;
use color_convert_rs::xyz;

const XYZ_TOLERANCE: f64 = 5e-4;
const LAB_TOLERANCE: f64 = 1e-3;

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

/// Behavior 1: `rgb_to_xyz_batch` (f32x8 SIMD) must match the scalar
/// `rgb::xyz` (f64) within XYZ_TOLERANCE (5e-4) for batches including
/// non-multiples of the SIMD lane width (8).
#[test]
fn rgb_to_xyz_batch_matches_scalar() {
    for n in [1, 3, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::xyz(p)).collect();
        let simd_result = simd::rgb_to_xyz_batch(&pixels);

        assert_eq!(
            simd_result.len(),
            scalar.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
            // Compile-time type check: simd_val MUST be [f32; 3], not [f64; 3].
            // This FAILS to compile while simd::rgb_to_xyz_batch returns Vec<[f64; 3]>.
            let _f32_check: [f32; 3] = *simd_val;

            for chan in 0..3 {
                let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
                assert!(
                    diff <= XYZ_TOLERANCE,
                    "pixel {i} channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    XYZ_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 3: `rgb_to_xyz_batch_soa` (SoA SIMD) must produce identical
/// f32 output to `rgb_to_xyz_batch` (AoS SIMD) for the same pixels.
/// SoA input slices are de-interleaved from the same AoS pixel batch.
/// Tests non-multiples of the SIMD lane width (8) to verify tail handling.
#[test]
fn rgb_to_xyz_batch_soa_matches_aos() {
    for n in [1, 3, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        // De-interleave AoS → SoA
        let r: Vec<u8> = pixels.iter().map(|p| p[0]).collect();
        let g: Vec<u8> = pixels.iter().map(|p| p[1]).collect();
        let b: Vec<u8> = pixels.iter().map(|p| p[2]).collect();

        let aos_result = simd::rgb_to_xyz_batch(&pixels);
        let soa_result = simd::rgb_to_xyz_batch_soa(&r, &g, &b);

        assert_eq!(
            soa_result.len(),
            aos_result.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (soa_val, aos_val)) in soa_result.iter().zip(aos_result.iter()).enumerate() {
            for chan in 0..3 {
                // Exact f32 bit equality expected: same math, same types.
                assert_eq!(
                    soa_val[chan], aos_val[chan],
                    "pixel {i} channel {chan}: soa={} aos={} differ for n={n}",
                    soa_val[chan], aos_val[chan],
                );
            }
        }
    }
}

/// Behavior 4: `xyz_to_lab_batch_soa` (SoA SIMD) must produce identical
/// f32 output to `xyz_to_lab_batch` (AoS SIMD) for the same XYZ pixels.
#[test]
fn xyz_to_lab_batch_soa_matches_aos() {
    for n in [1, 3, 7, 8, 15, 16, 100, 257] {
        let rgb_pixels = generate_rgb_pixels(n);
        // Generate f32 XYZ via the AoS SIMD batch
        let xyz_aos = simd::rgb_to_xyz_batch(&rgb_pixels);
        // De-interleave XYZ AoS → SoA
        let x: Vec<f32> = xyz_aos.iter().map(|p| p[0]).collect();
        let y: Vec<f32> = xyz_aos.iter().map(|p| p[1]).collect();
        let z: Vec<f32> = xyz_aos.iter().map(|p| p[2]).collect();

        let aos_result = simd::xyz_to_lab_batch(&xyz_aos);
        let soa_result = simd::xyz_to_lab_batch_soa(&x, &y, &z);

        assert_eq!(
            soa_result.len(),
            aos_result.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (soa_val, aos_val)) in soa_result.iter().zip(aos_result.iter()).enumerate() {
            for chan in 0..3 {
                assert_eq!(
                    soa_val[chan], aos_val[chan],
                    "pixel {i} channel {chan}: soa={} aos={} differ for n={n}",
                    soa_val[chan], aos_val[chan],
                );
            }
        }
    }
}

/// Behavior 2: `xyz_to_lab_batch` (f32x8 SIMD) must match the scalar
/// `xyz::lab` (f64) within LAB_TOLERANCE (1e-3). XYZ inputs are generated
/// from the deterministic RGB batch via `rgb::xyz` to exercise the full
/// piecewise LAB transfer.
#[test]
fn xyz_to_lab_batch_matches_scalar() {
    for n in [1, 3, 7, 8, 15, 16, 100, 257] {
        let rgb_pixels = generate_rgb_pixels(n);
        // Generate f32 XYZ via the SIMD batch (the function under test for batch #1)
        let xyz_simd = simd::rgb_to_xyz_batch(&rgb_pixels);
        let scalar: Vec<[f64; 3]> = xyz_simd
            .iter()
            .map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
            .map(xyz::lab)
            .collect();
        let simd_result = simd::xyz_to_lab_batch(&xyz_simd);

        assert_eq!(
            simd_result.len(),
            scalar.len(),
            "batch size mismatch for n={n}"
        );

        for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
            let _f32_check: [f32; 3] = *simd_val;

            for chan in 0..3 {
                let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
                assert!(
                    diff <= LAB_TOLERANCE,
                    "pixel {i} channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    LAB_TOLERANCE,
                );
            }
        }
    }
}
