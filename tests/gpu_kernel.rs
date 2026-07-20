//! Tests for the CubeCL GPU batch conversion kernels (issues #22, #123).
//!
//! Because this host has no GPU adapter, GPU client creation is guarded
//! with `std::panic::catch_unwind` (AGENTS.md Rule 5 — never panic on a
//! GPU-less host).  When no GPU is present, the kernel function returns
//! `None` cleanly and the test skips the correctness assertion with a
//! documented early return.
//!
//! When a GPU IS available (not on this host), the function returns
//! `Some(Vec<[f32; N]>)` — each tuple is the converted value for the
//! corresponding input pixel.  Tolerance is documented inline per route.

use color_convert_rs::gpu;

/// RED: The kernel function `rgb_to_lab_gpu_batch` must exist and MUST NOT
/// panic on a GPU-less host (this host).  The return is `Option<Vec<[f32; 3]>>`:
/// `None` when the GPU client is unavailable; `Some(results)` otherwise.
///
/// Source: reference vectors `tests/vectors/rgb_to_lab.json` (JS
/// color-convert@3.1.3), but the test does NOT yet compare values — it
/// only asserts the function compiles, runs, and never panics.
#[test]
fn gpu_kernel_does_not_panic_on_gpu_less_host() {
    let input: Vec<[u8; 3]> = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
    let result: Option<Vec<[f32; 3]>> = gpu::rgb_to_lab_gpu_batch(&input);

    match result {
        None => {
            // No GPU was available — expected on this host.
            // The function returned cleanly without panicking. Green-by-skip.
        }
        Some(lab_vec) => {
            // A GPU was present — kernel ran successfully.
            assert_eq!(
                lab_vec.len(),
                input.len(),
                "output must have one LAB per input RGB"
            );
            // TODO(#22): add correctness assertion against reference vectors
            // once the tolerance gate behavior is tested.
        }
    }
}

/// RED → GREEN behaviour 2 (correctness gate):
/// When a GPU is available, the GPU output must match the CPU scalar
/// `rgb::lab` within a documented per-channel absolute tolerance of 0.5
/// CIELAB units.  The tolerance accommodates the f32 GPU path vs the f64
/// CPU path — identical formulas, but f32 rounding on the piecewise LAB
/// transfer and matrix multiply introduces a small delta.
///
/// When no GPU is present (this CI/dev host), the test returns early
/// with a green-by-skip — the function returned `None` cleanly, so no
/// correctness assertion can be made, but structural correctness is
/// verified by the kernel compiling and the launch path being reachable
/// on a GPU-present host.
///
/// Tolerance: ≤ 0.5 per channel (l, a, b), f32 abs diff.
/// Vectors: hand-picked RGB primaries that exercise all LAB path branches
/// (gamma, piecewise transfer, negative a/b channels).
#[test]
fn gpu_kernel_matches_cpu_lab_within_tolerance() {
    // Test vectors: pure red, green, blue, white, black.
    // Expected LAB values from the scalar CPU path (f64).
    let test_vectors: Vec<[u8; 3]> = vec![
        [255, 0, 0],     // pure red   → approx LAB [53, 80, 67]
        [0, 255, 0],     // pure green → approx LAB [88, -86, 83]
        [0, 0, 255],     // pure blue  → approx LAB [32, 79, -108]
        [255, 255, 255], // white     → approx LAB [100, 0, 0]
        [0, 0, 0],       // black     → approx LAB [0, 0, 0]
    ];

    let result = gpu::rgb_to_lab_gpu_batch(&test_vectors);

    match result {
        None => {
            // No GPU available — expected on this host.
            // The function returned cleanly without panicking.
            // Green-by-skip: structural correctness is verified by
            // the kernel compiling and the launch harness being
            // reachable on a GPU-present host.
            //
            // When a GPU IS present, this branch is never taken and
            // the Some branch below validates pixel-for-pixel
            // correctness against the CPU reference.
        }
        Some(gpu_lab) => {
            assert_eq!(
                gpu_lab.len(),
                test_vectors.len(),
                "output length must match input length"
            );

            for (i, lab_gpu) in gpu_lab.iter().enumerate() {
                let cpu_ref = color_convert_rs::rgb::lab(test_vectors[i]);
                let cpu_lab: [f32; 3] = [cpu_ref[0] as f32, cpu_ref[1] as f32, cpu_ref[2] as f32];

                let tol: f32 = 0.5;
                assert!(
                    (lab_gpu[0] - cpu_lab[0]).abs() <= tol,
                    "pixel {i} L channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    lab_gpu[0],
                    cpu_lab[0],
                    (lab_gpu[0] - cpu_lab[0]).abs()
                );
                assert!(
                    (lab_gpu[1] - cpu_lab[1]).abs() <= tol,
                    "pixel {i} a channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    lab_gpu[1],
                    cpu_lab[1],
                    (lab_gpu[1] - cpu_lab[1]).abs()
                );
                assert!(
                    (lab_gpu[2] - cpu_lab[2]).abs() <= tol,
                    "pixel {i} b channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    lab_gpu[2],
                    cpu_lab[2],
                    (lab_gpu[2] - cpu_lab[2]).abs()
                );
            }
        }
    }
}

// ── Issue #123: GPU parity kernels for rgb→hsl, rgb→hsv, rgb→cmyk ───────

/// RED (issue #123): `rgb_to_hsl_gpu_batch` must exist and not panic on a
/// GPU-less host. When a GPU is available, the output must match the CPU
/// scalar `rgb::hsl` within an f32 vs f64 tolerance of 0.1 per channel.
///
/// The HSL computation involves divisions and multiplications that are
/// well-conditioned for f32 — the per-channel tolerance of 0.1 covers the
/// precision loss from f32 rounding on the hue/saturation/lightness formulas.
/// Test vectors: pure red, green, blue, white, black, and a mid-tone gray.
#[test]
fn gpu_kernel_rgb_to_hsl_matches_cpu_within_tolerance() {
    let test_vectors: Vec<[u8; 3]> = vec![
        [255, 0, 0],     // pure red   → approx HSL [0, 100, 50]
        [0, 255, 0],     // pure green → approx HSL [120, 100, 50]
        [0, 0, 255],     // pure blue  → approx HSL [240, 100, 50]
        [255, 255, 255], // white       → approx HSL [0, 0, 100]
        [0, 0, 0],       // black       → approx HSL [0, 0, 0]
        [128, 128, 128], // mid gray    → approx HSL [0, 0, 50]
    ];

    let result = gpu::rgb_to_hsl_gpu_batch(&test_vectors);

    match result {
        None => {
            // No GPU available — green-by-skip.
        }
        Some(gpu_hsl) => {
            assert_eq!(
                gpu_hsl.len(),
                test_vectors.len(),
                "output length must match input length"
            );

            for (i, hsl_gpu) in gpu_hsl.iter().enumerate() {
                let cpu_ref = color_convert_rs::rgb::hsl(test_vectors[i]);
                let cpu_hsl: [f32; 3] = [cpu_ref[0] as f32, cpu_ref[1] as f32, cpu_ref[2] as f32];

                let tol: f32 = 0.1;
                assert!(
                    (hsl_gpu[0] - cpu_hsl[0]).abs() <= tol,
                    "pixel {i} H channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsl_gpu[0],
                    cpu_hsl[0],
                    (hsl_gpu[0] - cpu_hsl[0]).abs()
                );
                assert!(
                    (hsl_gpu[1] - cpu_hsl[1]).abs() <= tol,
                    "pixel {i} S channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsl_gpu[1],
                    cpu_hsl[1],
                    (hsl_gpu[1] - cpu_hsl[1]).abs()
                );
                assert!(
                    (hsl_gpu[2] - cpu_hsl[2]).abs() <= tol,
                    "pixel {i} L channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsl_gpu[2],
                    cpu_hsl[2],
                    (hsl_gpu[2] - cpu_hsl[2]).abs()
                );
            }
        }
    }
}

/// RED (issue #123): `rgb_to_hsv_gpu_batch` must exist and not panic on a
/// GPU-less host. When a GPU is available, the output must match the CPU
/// scalar `rgb::hsv` within an f32 vs f64 tolerance of 0.1 per channel.
///
/// The HSV computation uses min/max/delta from normalized RGB channels and
/// a diffc-based hue derivation. All operations are well-conditioned for
/// f32, and the per-channel tolerance of 0.1 covers the precision loss.
/// Test vectors: pure red, green, blue, white, black, and a mid-tone gray.
#[test]
fn gpu_kernel_rgb_to_hsv_matches_cpu_within_tolerance() {
    let test_vectors: Vec<[u8; 3]> = vec![
        [255, 0, 0],     // pure red   → approx HSV [0, 100, 100]
        [0, 255, 0],     // pure green → approx HSV [120, 100, 100]
        [0, 0, 255],     // pure blue  → approx HSV [240, 100, 100]
        [255, 255, 255], // white       → approx HSV [0, 0, 100]
        [0, 0, 0],       // black       → approx HSV [0, 0, 0]
        [128, 128, 128], // mid gray    → approx HSV [0, 0, 50]
    ];

    let result = gpu::rgb_to_hsv_gpu_batch(&test_vectors);

    match result {
        None => {
            // No GPU available — green-by-skip.
        }
        Some(gpu_hsv) => {
            assert_eq!(
                gpu_hsv.len(),
                test_vectors.len(),
                "output length must match input length"
            );

            for (i, hsv_gpu) in gpu_hsv.iter().enumerate() {
                let cpu_ref = color_convert_rs::rgb::hsv(test_vectors[i]);
                let cpu_hsv: [f32; 3] = [cpu_ref[0] as f32, cpu_ref[1] as f32, cpu_ref[2] as f32];

                let tol: f32 = 0.1;
                assert!(
                    (hsv_gpu[0] - cpu_hsv[0]).abs() <= tol,
                    "pixel {i} H channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsv_gpu[0],
                    cpu_hsv[0],
                    (hsv_gpu[0] - cpu_hsv[0]).abs()
                );
                assert!(
                    (hsv_gpu[1] - cpu_hsv[1]).abs() <= tol,
                    "pixel {i} S channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsv_gpu[1],
                    cpu_hsv[1],
                    (hsv_gpu[1] - cpu_hsv[1]).abs()
                );
                assert!(
                    (hsv_gpu[2] - cpu_hsv[2]).abs() <= tol,
                    "pixel {i} V channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    hsv_gpu[2],
                    cpu_hsv[2],
                    (hsv_gpu[2] - cpu_hsv[2]).abs()
                );
            }
        }
    }
}

/// RED (issue #123): `rgb_to_cmyk_gpu_batch` must exist and not panic on a
/// GPU-less host. When a GPU is available, the output must match the CPU
/// scalar `rgb::cmyk` within an f32 vs f64 tolerance of 0.1 per channel.
///
/// The CMYK computation involves a black-key guard division (k == 1 → 0)
/// that mirrors the JS `|| 0` fallback. The per-channel tolerance of 0.1
/// covers the f32 precision loss on the `(1-r-k)/(1-k)` division chain.
/// CMYK has 4 output channels: `[c, m, y, k]` each in 0–100 range.
/// Test vectors: pure red, green, blue, white, black, and a mid-tone gray.
#[test]
fn gpu_kernel_rgb_to_cmyk_matches_cpu_within_tolerance() {
    let test_vectors: Vec<[u8; 3]> = vec![
        [255, 0, 0],     // pure red   → approx CMYK [0, 100, 100, 0]
        [0, 255, 0],     // pure green → approx CMYK [100, 0, 100, 0]
        [0, 0, 255],     // pure blue  → approx CMYK [100, 100, 0, 0]
        [255, 255, 255], // white       → approx CMYK [0, 0, 0, 0]
        [0, 0, 0],       // black       → approx CMYK [0, 0, 0, 100]
        [128, 128, 128], // mid gray    → approx CMYK [0, 0, 0, 50]
    ];

    let result = gpu::rgb_to_cmyk_gpu_batch(&test_vectors);

    match result {
        None => {
            // No GPU available — green-by-skip.
        }
        Some(gpu_cmyk) => {
            assert_eq!(
                gpu_cmyk.len(),
                test_vectors.len(),
                "output length must match input length"
            );

            for (i, cmyk_gpu) in gpu_cmyk.iter().enumerate() {
                let cpu_ref = color_convert_rs::rgb::cmyk(test_vectors[i]);
                let cpu_cmyk: [f32; 4] = [
                    cpu_ref[0] as f32,
                    cpu_ref[1] as f32,
                    cpu_ref[2] as f32,
                    cpu_ref[3] as f32,
                ];

                let tol: f32 = 0.1;
                assert!(
                    (cmyk_gpu[0] - cpu_cmyk[0]).abs() <= tol,
                    "pixel {i} C channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    cmyk_gpu[0],
                    cpu_cmyk[0],
                    (cmyk_gpu[0] - cpu_cmyk[0]).abs()
                );
                assert!(
                    (cmyk_gpu[1] - cpu_cmyk[1]).abs() <= tol,
                    "pixel {i} M channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    cmyk_gpu[1],
                    cpu_cmyk[1],
                    (cmyk_gpu[1] - cpu_cmyk[1]).abs()
                );
                assert!(
                    (cmyk_gpu[2] - cpu_cmyk[2]).abs() <= tol,
                    "pixel {i} Y channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    cmyk_gpu[2],
                    cpu_cmyk[2],
                    (cmyk_gpu[2] - cpu_cmyk[2]).abs()
                );
                assert!(
                    (cmyk_gpu[3] - cpu_cmyk[3]).abs() <= tol,
                    "pixel {i} K channel: gpu={}, cpu={}, diff={} > tol={tol}",
                    cmyk_gpu[3],
                    cpu_cmyk[3],
                    (cmyk_gpu[3] - cpu_cmyk[3]).abs()
                );
            }
        }
    }
}
