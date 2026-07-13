//! Integration tests for the runtime GPU capability probe.
//!
//! These tests verify that the probe:
//! - Returns a valid `Backend` (Gpu or CpuSimd)
//! - NEVER panics on any host, GPU-present or GPU-less (Rule 5)

use color_convert_rs::{Backend, gpu_present, probe};

/// Behavior 1: `probe()` returns one of the two `Backend` variants and
/// does NOT panic. A panic would cause the test harness to exit non-zero,
/// so simply calling the function and matching both arms is the assertion.
#[test]
fn probe_returns_valid_backend_and_does_not_panic() {
    let backend = probe();

    match backend {
        Backend::Gpu => {
            // GPU was detected — valid outcome on a GPU-present host.
        }
        Backend::CpuSimd => {
            // CPU-SIMD fallback — valid outcome on a GPU-less host,
            // or when GPU init fails for any reason.
        }
    }

    // If we reach here, probe() completed without panicking.
    // Both arms are valid — no further assertion needed.
}

/// Behavior 2: `gpu_present()` must be a pure boolean query that never
/// panics. The CpuSimd fallback path is exercised by the error-handling
/// branches inside `try_probe()` — on this NixOS host wgpu cannot load
/// Vulkan through Nix store paths, so the probe resolves to `CpuSimd`
/// and `gpu_present()` returns `false`.
#[test]
fn gpu_present_is_pure_boolean_query_no_panic() {
    // Call repeatedly to prove determinism and lack of side-effects.
    let first = gpu_present();
    for _ in 0..10 {
        assert_eq!(gpu_present(), first, "gpu_present() must be deterministic per run");
    }
}
