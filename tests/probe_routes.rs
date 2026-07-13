//! Integration tests for the runtime GPU capability probe.
//!
//! These tests verify that the probe:
//! - Returns a valid `Backend` (Gpu or CpuSimd)
//! - NEVER panics on any host, GPU-present or GPU-less (Rule 5)

use color_convert_rs::{Backend, probe};

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
