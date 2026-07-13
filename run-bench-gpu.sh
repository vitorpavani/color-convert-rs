#!/usr/bin/env bash
# Run the GPU-tier benchmark (CubeCL/wgpu) on a NixOS host with an NVIDIA card.
#
# WHY THIS WRAPPER EXISTS
# -----------------------
# On NixOS the Vulkan loader and the NVIDIA ICD manifest are NOT on the default
# library/search path. Without them, wgpu enumerates ZERO adapters and CubeCL's
# `WgpuRuntime::client()` fails with `NotFound { active_backends: 0x0 }` — which
# the library correctly treats as "no GPU" and falls back to CPU-SIMD (Rule 5).
#
# That fallback is correct for genuinely GPU-less hosts, but on a machine that
# DOES have an NVIDIA GPU it means the GPU tier is silently skipped. This wrapper
# points the process at the system Vulkan loader + NVIDIA ICD so the RTX card is
# discoverable, then runs the GPU benchmark. Mirrors gpu-matmul-bench/run.sh.
#
# On a non-NixOS host with the Vulkan loader on the default path, you can skip
# this wrapper and run `cargo run --release --bin bench_gpu` directly.
#
# The library binary itself needs NO changes: with the env set, the runtime
# probe resolves to Gpu and the existing gated code path executes.
set -euo pipefail
cd "$(dirname "$0")"

# 1. Build (NixOS needs a C linker; there is no global `cc`).
RUSTFLAGS="-Clinker-features=-lld" \
    nix shell nixpkgs#gcc --command cargo build --release --bin bench_gpu

# 2. Resolve the Vulkan loader from nixpkgs (cached after first fetch).
VKLIB="$(nix build nixpkgs#vulkan-loader --print-out-paths --no-link)/lib"

# 3. Run with the loader + NVIDIA ICD on the path so wgpu finds the adapter.
#    /run/opengl-driver is the NixOS OpenGL/Vulkan driver tree; the NVIDIA ICD
#    manifest lives under its share/vulkan/icd.d.
LD_LIBRARY_PATH="${VKLIB}:/run/opengl-driver/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
    VK_ICD_FILENAMES="/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json" \
    ./target/release/bench_gpu "$@"
