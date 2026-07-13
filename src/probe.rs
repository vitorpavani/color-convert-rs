//! Runtime GPU capability probe.
//!
//! At startup, enumerates physical GPU adapters through `wgpu` (the backend
//! CubeCL uses under the hood). If at least one non-CPU adapter is found the
//! probe returns [`Backend::Gpu`]; otherwise it falls back to
//! [`Backend::CpuSimd`] so the library runs on any server without a panic.
//!
//! # Integration with other modules
//!
//! - **Benchmark ledger (#19):** calls [`gpu_present`] to populate the
//!   `gpu_present` field in `benchmarks/results.jsonl`, recording whether
//!   the GPU tier was available at measurement time.
//! - **GPU compute kernel (#22):** checks [`probe`] at startup to decide
//!   whether to route pixel batches through the CubeCL/wgpu kernel or the
//!   CPU-SIMD path.
//!
//! # Safety (Rule 5)
//!
//! The probe MUST NEVER panic on a GPU-less host. Every GPU-init path is
//! wrapped so failure → `CpuSimd`. No `unwrap()`/`expect()`/`unsafe` in this
//! module — the entire body of [`probe`] is additionally guarded with
//! [`std::panic::catch_unwind`] as a last-resort safety net against
//! unexpected panics from upstream graphics backends.

/// The resolved compute backend for this runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// GPU compute via CubeCL / wgpu — a physical adapter was detected.
    Gpu,
    /// CPU fallback with SIMD — no GPU adapter found, or GPU init failed.
    CpuSimd,
}

/// Probe the current host for a physical GPU adapter.
///
/// Uses `wgpu` to enumerate adapters. Returns [`Backend::Gpu`] when at least
/// one [`wgpu::DeviceType::DiscreteGpu`] or [`wgpu::DeviceType::IntegratedGpu`]
/// adapter is found. Falls back to [`Backend::CpuSimd`] on any failure,
/// including a host with no GPU hardware, missing Vulkan drivers, or an
/// unexpected panic from the `wgpu` crate itself.
pub fn probe() -> Backend {
    std::panic::catch_unwind(try_probe).unwrap_or(Backend::CpuSimd)
}

/// Returns `true` when the probe resolved to [`Backend::Gpu`].
///
/// This is the flag used by the benchmark ledger (`benchmarks/results.jsonl`)
/// to populate the `gpu_present` field. Issue #22 (GPU compute kernel) and
/// the 3-tier bench harness (#19) call this to decide which path to measure.
pub fn gpu_present() -> bool {
    probe() == Backend::Gpu
}

/// The actual probing logic, separated so [`std::panic::catch_unwind`] can
/// guard the entire wgpu interaction.
fn try_probe() -> Backend {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());

    // Block on the async adapter-enumeration future.
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));

    for adapter in adapters {
        let info = adapter.get_info();
        match info.device_type {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => {
                return Backend::Gpu;
            }
            _ => continue,
        }
    }

    Backend::CpuSimd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test: the probe returns a valid variant and does not panic.
    /// The integration test in `tests/probe_routes.rs` covers the same
    /// contract from the public-API side.
    #[test]
    fn probe_does_not_panic() {
        let backend = probe();
        match backend {
            Backend::Gpu => {}
            Backend::CpuSimd => {}
        }
        // Both arms are valid; the test passes as long as we reach here.
    }

    /// `gpu_present()` is a pure boolean query — must not panic.
    #[test]
    fn gpu_present_does_not_panic() {
        let _present = gpu_present();
    }
}
