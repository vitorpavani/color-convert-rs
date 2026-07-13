//! Tests for the CubeCL GPU batch conversion kernel (issue #22).
//!
//! Because this host has no GPU adapter, GPU client creation is guarded
//! with `std::panic::catch_unwind` (AGENTS.md Rule 5 — never panic on a
//! GPU-less host).  When no GPU is present, the kernel function returns
//! `None` cleanly and the test skips the correctness assertion with a
//! documented early return.
//!
//! When a GPU IS available (not on this host), the function returns
//! `Some(Vec<[f32; 3]>)` — each triplet is the CIELAB `[l, a, b]` for
//! the corresponding input pixel.  Tolerance is documented inline.

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
