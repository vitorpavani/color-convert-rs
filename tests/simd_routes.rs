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
use color_convert_rs::simd_oklab;
use color_convert_rs::simd_xyz;
use color_convert_rs::xyz;

const XYZ_TOLERANCE: f64 = 5e-4;
const LAB_TOLERANCE: f64 = 1e-3;
const OKLAB_TOLERANCE: f64 = 1e-3;
/// Tolerance for f32-SIMD `xyz→rgb` vs scalar f64 `xyz::rgb`.
///
/// Output range is [0, 255] per channel. The forward sRGB gamma uses
/// `powf(1.0/2.4)` — gentler than the inverse `powf(2.4)` in rgb→xyz —
/// but the 3×3 matrix + gamma + ×255 chain accumulates f32 vs f64
/// rounding.  0.1 absolute (≈4e-4 relative at 255) catches real bugs
/// while accepting the f32→f64 gap.
const XYZ_RGB_TOLERANCE: f64 = 0.1;

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

/// Behavior 3: `rgb_to_lab_batch` (fused f32x8 SIMD, no intermediate XYZ
/// Vec) must match the two-step chain `xyz_to_lab_batch(rgb_to_xyz_batch(…))`
/// within f32 epsilon. Both paths perform the same arithmetic on the same
/// lanes; the only difference is that the fused pass feeds XYZ registers
/// directly into the LAB transform instead of storing/loading a Vec.
///
/// Tested across sizes that exercise SIMD-block (multiples of 8), remainder
/// tails (1–7 leftovers), and empty-edge cases.
#[test]
fn rgb_to_lab_batch_matches_two_step() {
    // Tolerance: f32::EPSILON × 10. Both paths use the same f32 operations
    // (same inline helpers, identical coefficients), so results should be
    // bit-for-bit identical.  The generous ×10 multiplier guards against any
    // subnormal or micro-architectural quirk without masking real bugs.
    const FUSED_TOLERANCE: f32 = f32::EPSILON * 10.0;

    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);

        let two_step = simd::xyz_to_lab_batch(&simd::rgb_to_xyz_batch(&pixels));
        let fused = simd::rgb_to_lab_batch(&pixels);

        assert_eq!(fused.len(), two_step.len(), "length mismatch for n={n}");

        for (i, (f, t)) in fused.iter().zip(two_step.iter()).enumerate() {
            let _f32_check: [f32; 3] = *f;

            for chan in 0..3 {
                let diff = (f[chan] - t[chan]).abs();
                assert!(
                    diff <= FUSED_TOLERANCE,
                    "pixel {i} channel {chan}: fused={} two_step={} diff={:.2e} > tol={:.2e}",
                    f[chan],
                    t[chan],
                    diff,
                    FUSED_TOLERANCE,
                );
            }
        }
    }
}

/// Behavior 4: `rgb_to_oklab_batch` (f32x8 SIMD) must match the scalar
/// `rgb::oklab` (f64) within OKLAB_TOLERANCE (1e-3) for batches including
/// non-multiples of the SIMD lane width (8), plus edge cases: black, white,
/// and the three primaries (red, green, blue).
///
/// The oklab pipeline is transcendental-heavy: sRGB inverse gamma
/// (powf 2.4) → LMS matrix → cbrt³ → Oklab matrix → ×100. f32's ~7
/// decimal digits of precision vs f64's ~15 allow a detectable gap after
/// three transcendental steps (one powf branch + three cbrt lanes).
/// OKLAB_TOLERANCE is set to 1e-3 — matching LAB_TOLERANCE from the
/// existing xyz→lab route which has similar computational depth.
///
/// Edge cases exercise dark values (black → cbrt of near-zero), bright
/// saturated primaries (clamped sRGB inverse gamma), and white (all
/// channels at maximum, a/b near zero).
#[test]
fn rgb_to_oklab_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let pixels = generate_rgb_pixels(n);
        let scalar: Vec<[f64; 3]> = pixels.iter().map(|&p| rgb::oklab(p)).collect();
        let simd_result = simd_oklab::rgb_to_oklab_batch(&pixels);

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
                    diff <= OKLAB_TOLERANCE,
                    "pixel {i} channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    OKLAB_TOLERANCE,
                );
            }
        }
    }

    // Edge cases: black, white, primaries
    let edge_cases: [[u8; 3]; 5] = [
        [0, 0, 0],       // black → L≈0, a≈0, b≈0
        [255, 255, 255], // white → L≈100, a≈0, b≈0
        [255, 0, 0],     // red primary
        [0, 255, 0],     // green primary
        [0, 0, 255],     // blue primary
    ];

    let scalar: Vec<[f64; 3]> = edge_cases.iter().map(|&p| rgb::oklab(p)).collect();
    let simd_result = simd_oklab::rgb_to_oklab_batch(&edge_cases);

    assert_eq!(
        simd_result.len(),
        scalar.len(),
        "edge-case batch size mismatch"
    );

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
        let _f32_check: [f32; 3] = *simd_val;
        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            assert!(
                diff <= OKLAB_TOLERANCE,
                "edge pixel {i} [{},{},{}] channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                edge_cases[i][0],
                edge_cases[i][1],
                edge_cases[i][2],
                simd_val[chan],
                scalar_val[chan],
                diff,
                OKLAB_TOLERANCE,
            );
        }
    }
}

/// Behavior 5: `xyz_to_rgb_batch` (f32x8 SIMD) must match the scalar
/// `xyz::rgb` (f64) within XYZZRGB_TOLERANCE (0.1) for batches including
/// non-multiples of the SIMD lane width (8). XYZ inputs are generated from
/// the deterministic RGB batch via `rgb::xyz` (f64), then fed to both the
/// scalar `xyz::rgb` and the SIMD batch.
///
/// Edge cases: pure black XYZ [0,0,0] → rgb [0,0,0], and D65 white XYZ
/// [95.047, 100.0, 108.883] → rgb [255,255,255] (within tolerance).
#[test]
fn xyz_to_rgb_batch_matches_scalar() {
    for n in [1, 7, 8, 15, 16, 100, 257] {
        let rgb_pixels = generate_rgb_pixels(n);
        // Generate XYZ inputs via the scalar f64 path
        let xyz_inputs: Vec<[f32; 3]> = rgb_pixels
            .iter()
            .map(|&p| rgb::xyz(p))
            .map(|[x, y, z]| [x as f32, y as f32, z as f32])
            .collect();

        let scalar: Vec<[f64; 3]> = xyz_inputs
            .iter()
            .map(|&p| [p[0] as f64, p[1] as f64, p[2] as f64])
            .map(xyz::rgb)
            .collect();
        let simd_result = simd_xyz::xyz_to_rgb_batch(&xyz_inputs);

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
                    diff <= XYZ_RGB_TOLERANCE,
                    "pixel {i} channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                    simd_val[chan],
                    scalar_val[chan],
                    diff,
                    XYZ_RGB_TOLERANCE,
                );
            }
        }
    }

    // Edge cases: black XYZ and D65 white XYZ
    let edge_xyz: [[f32; 3]; 2] = [
        [0.0, 0.0, 0.0],          // pure black
        [95.047, 100.0, 108.883], // D65 white
    ];

    let scalar: Vec<[f64; 3]> = edge_xyz
        .iter()
        .map(|&p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .map(xyz::rgb)
        .collect();
    let simd_result = simd_xyz::xyz_to_rgb_batch(&edge_xyz);

    assert_eq!(
        simd_result.len(),
        scalar.len(),
        "edge-case batch size mismatch"
    );

    for (i, (simd_val, scalar_val)) in simd_result.iter().zip(scalar.iter()).enumerate() {
        let _f32_check: [f32; 3] = *simd_val;
        for chan in 0..3 {
            let diff = (simd_val[chan] as f64 - scalar_val[chan]).abs();
            assert!(
                diff <= XYZ_RGB_TOLERANCE,
                "edge xyz {:?} channel {chan}: simd(f32)={} scalar(f64)={} diff={:.2e} > tol={:.2e}",
                edge_xyz[i],
                simd_val[chan],
                scalar_val[chan],
                diff,
                XYZ_RGB_TOLERANCE,
            );
        }
    }
}
