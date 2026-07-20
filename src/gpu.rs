//! CubeCL GPU compute kernel for batch colour-space conversion.
//!
//! ## Behaviour
//!
//! The `rgb→lab` route is implemented as a 1-D kernel: each thread
//! processes exactly one `[u8; 3]` input pixel and writes one `[f32; 3]`
//! CIELAB output triplet.  The data buffer is a flat `f32` array of
//! length `3 * n_pixels`.
//!
//! ## Reference behaviour
//!
//! The kernel mirrors the scalar `rgb::lab` (see `src/rgb.rs`) exactly,
//! but computed in `f32` on the GPU rather than `f64` on the CPU.  This
//! means the GPU result will differ from the CPU result by a small f32
//! rounding delta — the correctness gate uses a documented tolerance.
//!
//! ## Host safety (Rule 5)
//!
//! On a GPU-less host (like this CI/dev server), `WgpuRuntime::client`
//! panics with "No possible adapter available".  The host-side wrapper
//! guards this with [`std::panic::catch_unwind`] and additionally checks
//! [`crate::probe::probe`] — the function returns `Option::None` cleanly
//! without panicking, aborting, or logging.
//!
//! ## Unsafe
//!
//! Two `unsafe` blocks exist: the [`rgb_to_lab_kernel::launch`] call and
//! [`ArrayArg::from_raw_parts`] — both are required by the CubeCL API.
//! Each is documented with a `// SAFETY:` comment justifying the length
//! invariant.  No other `unsafe` is used.
//!
//! ## Tolerance
//!
//! CIELAB `[l, a, b]` f32 GPU output is compared against the f64 CPU
//! reference with an absolute tolerance ≤ 0.5 in each channel.  This
//! accounts for the f32→f64 precision loss on the piecewise LAB transfer
//! function and the matrix multiply.  See the correctness-gate test.

use std::time::Instant;

use cubecl::prelude::*;

// ── CubeCL wgpu runtime types ──────────────────────────────────────────
use cubecl::wgpu::{WgpuDevice, WgpuRuntime};

// ── sRGB gamma expansion constants (f32, same as src/rgb.rs f64) ───────
const SRGB_THRESHOLD: f32 = 0.04045;
const SRGB_A: f32 = 0.055;
const SRGB_DIV: f32 = 1.055;
const SRGB_SLOPE: f32 = 1.0 / 12.92;
const SRGB_GAMMA: f32 = 2.4;

// ── sRGB→XYZ matrix coefficients (D65, 2° observer) ───────────────────
const M00: f32 = 0.4124564;
const M01: f32 = 0.3575761;
const M02: f32 = 0.1804375;
const M10: f32 = 0.2126729;
const M11: f32 = 0.7151522;
const M12: f32 = 0.0721750;
const M20: f32 = 0.0193339;
const M21: f32 = 0.119_192;
const M22: f32 = 0.9503041;

// ── D65 reference white point (CIE XYZ tristimulus) ────────────────────
const D65_X: f32 = 95.047;
const D65_Y: f32 = 100.0;
const D65_Z: f32 = 108.883;

// ── CIELAB piecewise-transfer constants ────────────────────────────────
const LAB_FT_DENOM: f32 = 29.0;
// LAB_FT = (6/29)³ ≈ 0.008856…  —  computed in the kernel for clarity.
const LAB_SLOPE: f32 = 7.787;
const LAB_OFFSET: f32 = 16.0 / 116.0; // = 4/29

// ── Kernel: rgb → lab (1 thread per pixel) ────────────────────────────

/// GPU kernel: converts `n_pixels` RGB triples (as a flat f32 buffer of
/// length `3 × n_pixels`) into CIELAB floats `[l, a, b]` in the output
/// buffer of the same length, one pixel per thread.
///
/// The calculation mirrors `rgb::lab` (f64, scalar) but runs in f32:
/// 1. Normalise `/255` → [0, 1]
/// 2. sRGB inverse nonlinear transform (gamma expansion, piecewise)
/// 3. Linear sRGB → XYZ (D65 matrix)
/// 4. XYZ / D65 white point
/// 5. CIELAB transfer (cbrt or linear segment)
/// 6. L = 116·y − 16,  a = 500·(x−y),  b = 200·(y−z)
///
/// # Note on intrinsics
///
/// CubeCL 0.10 exposes `Float::powf(exp)` for arbitrary exponents but
/// does NOT provide a dedicated `cbrt` intrinsic.  The cube root in
/// step 5 is computed as `t.powf(1.0 / 3.0)`, which is correct for the
/// non-negative input values produced by the XYZ normalisation step
/// (D65 white-point division never yields negative XYZ from valid sRGB).
#[cube(launch)]
fn rgb_to_lab_kernel(input: &Array<f32>, output: &mut Array<f32>, n_pixels: usize) {
    let pos = ABSOLUTE_POS; // global 1-D thread index

    // Bounds check — if/else instead of `return` (CubeCL 0.10 does not
    // support early return in #[cube] functions).
    if pos < n_pixels {
        let i = pos * 3;
        let r = input[i] / 255.0_f32;
        let g = input[i + 1] / 255.0_f32;
        let b = input[i + 2] / 255.0_f32;

        // ── Step 2: sRGB inverse nonlinear transform (gamma expansion) ──
        let r = if r > SRGB_THRESHOLD {
            ((r + SRGB_A) / SRGB_DIV).powf(SRGB_GAMMA)
        } else {
            r * SRGB_SLOPE
        };
        let g = if g > SRGB_THRESHOLD {
            ((g + SRGB_A) / SRGB_DIV).powf(SRGB_GAMMA)
        } else {
            g * SRGB_SLOPE
        };
        let b = if b > SRGB_THRESHOLD {
            ((b + SRGB_A) / SRGB_DIV).powf(SRGB_GAMMA)
        } else {
            b * SRGB_SLOPE
        };

        // ── Step 3: Linear sRGB → XYZ (D65 matrix) ─────────────────────
        let x = r * M00 + g * M01 + b * M02;
        let y = r * M10 + g * M11 + b * M12;
        let z = r * M20 + g * M21 + b * M22;

        // ── Step 4: XYZ / D65 white point ──────────────────────────────
        let x = x * (100.0_f32 / D65_X);
        let y = y * (100.0_f32 / D65_Y);
        let z = z * (100.0_f32 / D65_Z);

        // ── Step 5: CIELAB piecewise transfer ──────────────────────────
        // Cube root computed via powf(1/3) — correct for non-negative t.
        // CubeCL 0.10 does not expose a dedicated cbrt intrinsic.
        let ft_num = 6.0_f32 * 6.0_f32 * 6.0_f32; // 6³
        let ft_den = LAB_FT_DENOM * LAB_FT_DENOM * LAB_FT_DENOM; // 29³
        let ft = ft_num / ft_den; // (6/29)³

        let x = if x > ft {
            x.powf(1.0_f32 / 3.0_f32)
        } else {
            LAB_SLOPE * x + LAB_OFFSET
        };
        let y = if y > ft {
            y.powf(1.0_f32 / 3.0_f32)
        } else {
            LAB_SLOPE * y + LAB_OFFSET
        };
        let z = if z > ft {
            z.powf(1.0_f32 / 3.0_f32)
        } else {
            LAB_SLOPE * z + LAB_OFFSET
        };

        // ── Step 6: L, a, b formulas ───────────────────────────────────
        let l = 116.0_f32 * y - 16.0_f32;
        let a = 500.0_f32 * (x - y);
        let b_out = 200.0_f32 * (y - z);

        output[i] = l;
        output[i + 1] = a;
        output[i + 2] = b_out;
    }
}

// NOTE: Helper functions (`srgb_inv`, `lab_f`) were inlined into the kernel
// body below because CubeCL 0.10 does not support calling regular Rust fn
// items from a `#[cube]` kernel context — the kernel needs to be a single
// compilation unit for the WGSL/SPIR-V codegen.

// ── Launch grid helper ─────────────────────────────────────────────────
// wgpu limits each dispatch dimension to 65535. For large N we split into
// a 2-D grid so the per-dimension block count stays within the limit.
// ABSOLUTE_POS in the kernel is a scalar linear index that works correctly
// with any grid dimensionality.

const MAX_DISPATCH_DIM: u32 = 65535;
const BLOCK_SIZE: u32 = 64;

fn compute_launch_grid(n_pixels: usize) -> (CubeCount, CubeDim) {
    let n_u32: u32 = u32::try_from(n_pixels).unwrap_or(u32::MAX);
    let total_blocks = n_u32.div_ceil(BLOCK_SIZE);

    if total_blocks <= MAX_DISPATCH_DIM {
        (CubeCount::new_1d(total_blocks), CubeDim::new_1d(BLOCK_SIZE))
    } else {
        // 2-D split: both dims ≤ 65535, product ≥ total_blocks
        let grid_y = total_blocks
            .div_ceil(MAX_DISPATCH_DIM)
            .min(MAX_DISPATCH_DIM);
        let grid_x = total_blocks.div_ceil(grid_y).min(MAX_DISPATCH_DIM);
        (
            CubeCount::new_2d(grid_x, grid_y),
            CubeDim::new_2d(BLOCK_SIZE, 1),
        )
    }
}

// ── Host-side launch harness ──────────────────────────────────────────

/// Converts a batch of `[u8; 3]` RGB pixels to CIELAB `[f32; 3]` using
/// the CubeCL/wgpu GPU kernel.
///
/// # Return value
///
/// - `None` when no GPU is available (this host).  The function returned
///   cleanly — no panic, no abort, no diagnostic output.
/// - `Some(Vec<[f32; 3]>)` when a GPU was acquired and the kernel completed
///   successfully.  The vector is pixel-for-pixel with the input: `result[i]`
///   is the `[l, a, b]` for `rgb[i]`.
///
/// # Safety (Rule 5)
///
/// GPU client creation is guarded with `catch_unwind` because
/// `WgpuRuntime::client` panics on a GPU-less host (CubeCL 0.10 does not
/// degrade gracefully).  The additional `probe()` check short-circuits the
/// client creation entirely when the host is already known to have no GPU.
pub fn rgb_to_lab_gpu_batch(rgb: &[[u8; 3]]) -> Option<Vec<[f32; 3]>> {
    let n = rgb.len();
    if n == 0 {
        return Some(Vec::new());
    }

    // Fast-path — probe already told us no GPU is present.
    if crate::probe::probe() != crate::probe::Backend::Gpu {
        return None;
    }

    // catch_unwind: WgpuRuntime::client() panics when no adapter is found
    // (CubeCL 0.10 does not gracefully degrade).  On panic, return None.
    let client =
        std::panic::catch_unwind(|| WgpuRuntime::client(&WgpuDevice::DefaultDevice)).ok()?;

    // ── Upload: flat f32 buffer ─────────────────────────────────────
    let n_float = n * 3;
    let mut flat_input: Vec<f32> = Vec::with_capacity(n_float);
    for pixel in rgb {
        flat_input.push(f32::from(pixel[0]));
        flat_input.push(f32::from(pixel[1]));
        flat_input.push(f32::from(pixel[2]));
    }

    // create_from_slice expects &[u8]; bytemuck reinterprets f32 as u8.
    let in_handle = client.create_from_slice(bytemuck::cast_slice(&flat_input));
    let out_handle = client.empty(n_float * size_of::<f32>());

    // ── Launch configuration ────────────────────────────────────────
    let (cube_count, cube_dim) = compute_launch_grid(n);

    // SAFETY: The kernel launch is unsafe per the CubeCL API because the
    // runtime cannot statically verify that the ArrayArg handles match the
    // kernel's expected buffer lengths.  We guarantee:
    //   - `in_handle` has `n_float` (= 3 × n) f32 elements
    //   - `out_handle` has `n_float` f32 elements (allocated via `empty`)
    //   - `n_pixels` (= n) is the per-thread bounds-check constant
    //   - `cube_count` covers all n pixels (div_ceil ensures no underflow)
    unsafe {
        rgb_to_lab_kernel::launch(
            &client,
            cube_count,
            cube_dim,
            ArrayArg::from_raw_parts(in_handle.clone(), n_float),
            ArrayArg::from_raw_parts(out_handle.clone(), n_float),
            n,
        );
    }

    // ── Read back ────────────────────────────────────────────────────
    // read_one returns Result<Bytes, ServerError>.  In library code we
    // cannot unwrap — map to None on error.
    let bytes = match client.read_one(out_handle) {
        Ok(b) => b,
        Err(_) => return None,
    };
    let result_f32: &[f32] = bytemuck::cast_slice(&bytes);

    let mut result: Vec<[f32; 3]> = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 3;
        result.push([result_f32[base], result_f32[base + 1], result_f32[base + 2]]);
    }

    Some(result)
}

// ── GPU batch timing breakdown ─────────────────────────────────────────

/// Phase timing breakdown for a GPU batch conversion.
/// All fields are milliseconds, measured via `std::time::Instant`.
#[derive(Debug, Clone, Copy)]
pub struct GpuBatchTimings {
    /// Host→device upload (create_from_slice + empty allocation)
    pub upload_ms: f64,
    /// Kernel launch + execution on the GPU (synchronous launch)
    pub compute_ms: f64,
    /// Device→host read-back (read_one)
    pub readback_ms: f64,
}

/// Like [`rgb_to_lab_gpu_batch`] but returns per-phase timings alongside
/// the result, so the benchmark harness can distinguish transfer-bound vs
/// compute-bound behaviour as N scales.
///
/// # Timing methodology
///
/// Timing is done with `std::time::Instant` around each phase. The kernel
/// launch is synchronous in CubeCL 0.10 (the launch call blocks until the
/// GPU completes), so `compute_ms` includes both dispatch overhead and
/// actual GPU execution time.  Host↔device transfers are PCIe-bus timing.
///
/// # Return value
///
/// - `None` when no GPU is available.
/// - `Some((Vec<[f32; 3]>, GpuBatchTimings))` when the kernel completed.
pub fn rgb_to_lab_gpu_batch_timed(rgb: &[[u8; 3]]) -> Option<(Vec<[f32; 3]>, GpuBatchTimings)> {
    let n = rgb.len();
    if n == 0 {
        return Some((
            Vec::new(),
            GpuBatchTimings {
                upload_ms: 0.0,
                compute_ms: 0.0,
                readback_ms: 0.0,
            },
        ));
    }

    if crate::probe::probe() != crate::probe::Backend::Gpu {
        return None;
    }

    let client =
        std::panic::catch_unwind(|| WgpuRuntime::client(&WgpuDevice::DefaultDevice)).ok()?;

    let n_float = n * 3;
    let mut flat_input: Vec<f32> = Vec::with_capacity(n_float);
    for pixel in rgb {
        flat_input.push(f32::from(pixel[0]));
        flat_input.push(f32::from(pixel[1]));
        flat_input.push(f32::from(pixel[2]));
    }

    // ── Upload ─────────────────────────────────────────────────────────
    let upload_start = Instant::now();
    let in_handle = client.create_from_slice(bytemuck::cast_slice(&flat_input));
    let out_handle = client.empty(n_float * size_of::<f32>());
    let upload_ms = upload_start.elapsed().as_secs_f64() * 1000.0;

    // ── Compute (kernel launch, async in CubeCL 0.10) ──────────────
    let (cube_count, cube_dim) = compute_launch_grid(n);

    let compute_start = Instant::now();
    // SAFETY: The kernel launch is unsafe per the CubeCL API because the
    // runtime cannot statically verify that the ArrayArg handles match the
    // kernel's expected buffer lengths.  We guarantee:
    //   - `in_handle` has `n_float` (= 3 × n) f32 elements
    //   - `out_handle` has `n_float` f32 elements (allocated via `empty`)
    //   - `n_pixels` (= n) is the per-thread bounds-check constant
    //   - `cube_count` covers all n pixels (div_ceil ensures no underflow)
    unsafe {
        rgb_to_lab_kernel::launch(
            &client,
            cube_count,
            cube_dim,
            ArrayArg::from_raw_parts(in_handle.clone(), n_float),
            ArrayArg::from_raw_parts(out_handle.clone(), n_float),
            n,
        );
    }
    let compute_ms = compute_start.elapsed().as_secs_f64() * 1000.0;

    // ── Read-back ──────────────────────────────────────────────────────
    let readback_start = Instant::now();
    let bytes = match client.read_one(out_handle) {
        Ok(b) => b,
        Err(_) => return None,
    };
    let readback_ms = readback_start.elapsed().as_secs_f64() * 1000.0;

    let result_f32: &[f32] = bytemuck::cast_slice(&bytes);
    let mut result: Vec<[f32; 3]> = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 3;
        result.push([result_f32[base], result_f32[base + 1], result_f32[base + 2]]);
    }

    Some((
        result,
        GpuBatchTimings {
            upload_ms,
            compute_ms,
            readback_ms,
        },
    ))
}

// ── Kernel: rgb → hsl (1 thread per pixel) ─────────────────────────────

/// GPU kernel: converts `n_pixels` RGB triples (as a flat f32 buffer of
/// length `3 × n_pixels`) into HSL floats `[h, s, l]` in the output
/// buffer of the same length, one pixel per thread.
///
/// The calculation mirrors `rgb::hsl_f64` (f64, scalar) but runs in f32.
/// Tolerance: ≤ 0.1 per channel (well-conditioned f32 formulas).
#[cube(launch)]
fn rgb_to_hsl_kernel(input: &Array<f32>, output: &mut Array<f32>, n_pixels: usize) {
    // Derive NativeExpand f32 constants from the input array so that all
    // if/else branches use the same CubeCL type (standalone f32 literals
    // are treated as Rust f32, not NativeExpand, and mixing the two in a
    // conditional produces a type mismatch).
    let zero = input[0] * 0.0_f32;
    let one = zero + 1.0_f32;
    let two = one + one;
    let four = two + two;
    let three_sixty = zero + 360.0_f32;
    let half = zero + 0.5_f32;
    let hundred = zero + 100.0_f32;
    let sixty = zero + 60.0_f32;
    let two_fifty_five = zero + 255.0_f32;

    let pos = ABSOLUTE_POS;

    if pos < n_pixels {
        let i = pos * 3;
        let r = input[i] / two_fifty_five;
        let g = input[i + 1] / two_fifty_five;
        let b = input[i + 2] / two_fifty_five;

        // Min of three channels
        let diff_rg = r - g;
        let min_rg = if diff_rg < zero { r } else { g };
        let diff_min_b = min_rg - b;
        let min_val = if diff_min_b < zero { min_rg } else { b };

        // Max of three channels
        let diff_rg = r - g;
        let max_rg = if diff_rg > zero { r } else { g };
        let diff_max_b = max_rg - b;
        let max_val = if diff_max_b > zero { max_rg } else { b };

        let delta = max_val - min_val;

        // Lightness
        let l = (min_val + max_val) / two;

        // Saturation
        let diff_l = l - half;
        let s_raw = if diff_l <= zero {
            delta / (max_val + min_val)
        } else {
            delta / (two - max_val - min_val)
        };
        let s = if delta > zero { s_raw } else { zero };

        // Hue: 3-way branch (r==max, g==max, else b==max)
        let diff_r_max = r - max_val;
        let diff_g_max = g - max_val;
        let h_r = if diff_r_max == zero {
            (g - b) / delta
        } else {
            zero
        };
        let h_g = if diff_g_max == zero {
            two + (b - r) / delta
        } else {
            zero
        };
        // b==max: neither r nor g is max
        let diff_r_max_sq = diff_r_max * diff_r_max;
        let h_b = if diff_r_max_sq > zero {
            four + (r - g) / delta
        } else {
            zero
        };
        // Sum the three mutually exclusive branches (only one is non-zero)
        let h_raw = h_r + h_g + h_b;
        let h_deg = h_raw * sixty;
        let diff_h = h_deg - three_sixty;
        let h_clamped = if diff_h > zero { three_sixty } else { h_deg };
        let h = if h_clamped < zero {
            h_clamped + three_sixty
        } else {
            h_clamped
        };

        output[i] = h;
        output[i + 1] = s * hundred;
        output[i + 2] = l * hundred;
    }
}

/// Converts a batch of `[u8; 3]` RGB pixels to HSL `[f32; 3]` using
/// the CubeCL/wgpu GPU kernel.
///
/// # Return value
///
/// - `None` when no GPU is available (this host).
/// - `Some(Vec<[f32; 3]>)` when a GPU was acquired and the kernel completed.
pub fn rgb_to_hsl_gpu_batch(rgb: &[[u8; 3]]) -> Option<Vec<[f32; 3]>> {
    let n = rgb.len();
    if n == 0 {
        return Some(Vec::new());
    }

    if crate::probe::probe() != crate::probe::Backend::Gpu {
        return None;
    }

    let client =
        std::panic::catch_unwind(|| WgpuRuntime::client(&WgpuDevice::DefaultDevice)).ok()?;

    let n_float = n * 3;
    let mut flat_input: Vec<f32> = Vec::with_capacity(n_float);
    for pixel in rgb {
        flat_input.push(f32::from(pixel[0]));
        flat_input.push(f32::from(pixel[1]));
        flat_input.push(f32::from(pixel[2]));
    }

    let in_handle = client.create_from_slice(bytemuck::cast_slice(&flat_input));
    let out_handle = client.empty(n_float * size_of::<f32>());

    let (cube_count, cube_dim) = compute_launch_grid(n);

    // SAFETY: The kernel launch is unsafe per the CubeCL API because the
    // runtime cannot statically verify that the ArrayArg handles match the
    // kernel's expected buffer lengths.  We guarantee:
    //   - `in_handle` has `n_float` (= 3 × n) f32 elements
    //   - `out_handle` has `n_float` f32 elements (allocated via `empty`)
    //   - `n_pixels` (= n) is the per-thread bounds-check constant
    //   - `cube_count` covers all n pixels (div_ceil ensures no underflow)
    unsafe {
        rgb_to_hsl_kernel::launch(
            &client,
            cube_count,
            cube_dim,
            ArrayArg::from_raw_parts(in_handle.clone(), n_float),
            ArrayArg::from_raw_parts(out_handle.clone(), n_float),
            n,
        );
    }

    let bytes = match client.read_one(out_handle) {
        Ok(b) => b,
        Err(_) => return None,
    };
    let result_f32: &[f32] = bytemuck::cast_slice(&bytes);

    let mut result: Vec<[f32; 3]> = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 3;
        result.push([result_f32[base], result_f32[base + 1], result_f32[base + 2]]);
    }

    Some(result)
}

// ── Kernel: rgb → hsv (1 thread per pixel) ─────────────────────────────

/// GPU kernel: converts `n_pixels` RGB triples (as a flat f32 buffer of
/// length `3 × n_pixels`) into HSV floats `[h, s, v]` in the output
/// buffer of the same length, one pixel per thread.
///
/// The calculation mirrors `rgb::hsv_f64` (f64, scalar) but runs in f32:
/// 1. Normalise `/255` → [0, 1]
/// 2. Value = max(r, g, b), diff = value - min
/// 3. Saturation = if diff==0 {0} else {diff/value}
/// 4. Hue via diffc = (v-c)/6/diff + 0.5
/// 5. 3-way branch on v==r, v==g, else v==b
#[cube(launch)]
fn rgb_to_hsv_kernel(input: &Array<f32>, output: &mut Array<f32>, n_pixels: usize) {
    // Derive NativeExpand f32 constants from the input array.
    let zero = input[0] * 0.0_f32;
    let one = zero + 1.0_f32;
    let two = one + one;
    let three = two + one;
    let six = three + three;
    let half = zero + 0.5_f32;
    let hundred = zero + 100.0_f32;
    let three_sixty = zero + 360.0_f32;
    let two_fifty_five = zero + 255.0_f32;
    let one_third = one / three;
    let two_thirds = two / three;

    let pos = ABSOLUTE_POS;

    if pos < n_pixels {
        let i = pos * 3;
        let r = input[i] / two_fifty_five;
        let g = input[i + 1] / two_fifty_five;
        let b = input[i + 2] / two_fifty_five;

        // Min and max of three channels
        let diff_rg = r - g;
        let min_rg = if diff_rg < zero { r } else { g };
        let diff_min_b = min_rg - b;
        let min_val = if diff_min_b < zero { min_rg } else { b };

        let max_rg = if diff_rg > zero { r } else { g };
        let diff_max_b = max_rg - b;
        let v = if diff_max_b > zero { max_rg } else { b };

        let diff = v - min_val;

        // Saturation
        let s = if diff > zero { diff / v } else { zero };

        // Hue via diffc (mirrors JS diffc = (v-c)/6/diff + 0.5)
        let h = if diff > zero {
            let rdif = (v - r) / six / diff + half;
            let gdif = (v - g) / six / diff + half;
            let bdif = (v - b) / six / diff + half;

            let diff_r_v = r - v;
            let diff_g_v = g - v;
            let h_r = if diff_r_v == zero { bdif - gdif } else { zero };
            let h_g = if diff_g_v == zero {
                one_third + rdif - bdif
            } else {
                zero
            };
            let h_b = if diff_r_v != zero && diff_g_v != zero {
                two_thirds + gdif - rdif
            } else {
                zero
            };
            let h_raw = h_r + h_g + h_b;

            if h_raw < zero {
                h_raw + one
            } else if h_raw > one {
                h_raw - one
            } else {
                h_raw
            }
        } else {
            zero
        };

        output[i] = h * three_sixty;
        output[i + 1] = s * hundred;
        output[i + 2] = v * hundred;
    }
}

/// Converts a batch of `[u8; 3]` RGB pixels to HSV `[f32; 3]` using
/// the CubeCL/wgpu GPU kernel.
///
/// # Return value
///
/// - `None` when no GPU is available (this host).
/// - `Some(Vec<[f32; 3]>)` when a GPU was acquired and the kernel completed.
pub fn rgb_to_hsv_gpu_batch(rgb: &[[u8; 3]]) -> Option<Vec<[f32; 3]>> {
    let n = rgb.len();
    if n == 0 {
        return Some(Vec::new());
    }

    if crate::probe::probe() != crate::probe::Backend::Gpu {
        return None;
    }

    let client =
        std::panic::catch_unwind(|| WgpuRuntime::client(&WgpuDevice::DefaultDevice)).ok()?;

    let n_float = n * 3;
    let mut flat_input: Vec<f32> = Vec::with_capacity(n_float);
    for pixel in rgb {
        flat_input.push(f32::from(pixel[0]));
        flat_input.push(f32::from(pixel[1]));
        flat_input.push(f32::from(pixel[2]));
    }

    let in_handle = client.create_from_slice(bytemuck::cast_slice(&flat_input));
    let out_handle = client.empty(n_float * size_of::<f32>());

    let (cube_count, cube_dim) = compute_launch_grid(n);

    // SAFETY: The kernel launch is unsafe per the CubeCL API because the
    // runtime cannot statically verify that the ArrayArg handles match the
    // kernel's expected buffer lengths.  We guarantee:
    //   - `in_handle` has `n_float` (= 3 × n) f32 elements
    //   - `out_handle` has `n_float` f32 elements (allocated via `empty`)
    //   - `n_pixels` (= n) is the per-thread bounds-check constant
    //   - `cube_count` covers all n pixels (div_ceil ensures no underflow)
    unsafe {
        rgb_to_hsv_kernel::launch(
            &client,
            cube_count,
            cube_dim,
            ArrayArg::from_raw_parts(in_handle.clone(), n_float),
            ArrayArg::from_raw_parts(out_handle.clone(), n_float),
            n,
        );
    }

    let bytes = match client.read_one(out_handle) {
        Ok(b) => b,
        Err(_) => return None,
    };
    let result_f32: &[f32] = bytemuck::cast_slice(&bytes);

    let mut result: Vec<[f32; 3]> = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 3;
        result.push([result_f32[base], result_f32[base + 1], result_f32[base + 2]]);
    }

    Some(result)
}

// ── Kernel: rgb → cmyk (1 thread per pixel) ────────────────────────────

/// GPU kernel: converts `n_pixels` RGB triples (as a flat f32 buffer of
/// length `3 × n_pixels`) into CMYK floats `[c, m, y, k]` in the output
/// buffer of length `4 × n_pixels`, one pixel per thread.
///
/// The calculation mirrors `rgb::cmyk_f64` (f64, scalar) but runs in f32:
/// 1. Normalise `/255` → [0, 1]
/// 2. Black key: k = 1 - max(r, g, b)
/// 3. Guard: if k == 1 → CMY = 0 (mirrors JS `|| 0` fallback)
/// 4. CMY = (1 - c - k) / (1 - k)
///
/// # Note on output stride
///
/// CMYK has **4** output channels per pixel, so the output buffer uses
/// stride 4: `output[pos * 4 + ch]` for ch ∈ {c, m, y, k}.
#[cube(launch)]
fn rgb_to_cmyk_kernel(input: &Array<f32>, output: &mut Array<f32>, n_pixels: usize) {
    // Derive NativeExpand f32 constants from the input array.
    let zero = input[0] * 0.0_f32;
    let one = zero + 1.0_f32;
    let hundred = zero + 100.0_f32;
    let two_fifty_five = zero + 255.0_f32;

    let pos = ABSOLUTE_POS;

    if pos < n_pixels {
        let i = pos * 3;
        let r = input[i] / two_fifty_five;
        let g = input[i + 1] / two_fifty_five;
        let b = input[i + 2] / two_fifty_five;

        let inv_r = one - r;
        let inv_g = one - g;
        let inv_b = one - b;

        // Black key: min of (1-r, 1-g, 1-b)
        let diff_rg = inv_r - inv_g;
        let k_rg = if diff_rg < zero { inv_r } else { inv_g };
        let diff_k_b = k_rg - inv_b;
        let k = if diff_k_b < zero { k_rg } else { inv_b };

        let denom = one - k;

        // Guard: k == 1 (pure black) → CMY = 0 (mirrors JS || 0)
        let c = if denom > zero {
            (inv_r - k) / denom
        } else {
            zero
        };
        let m = if denom > zero {
            (inv_g - k) / denom
        } else {
            zero
        };
        let y = if denom > zero {
            (inv_b - k) / denom
        } else {
            zero
        };

        // CMYK output uses stride 4 (4 channels per pixel)
        let o = pos * 4;
        output[o] = c * hundred;
        output[o + 1] = m * hundred;
        output[o + 2] = y * hundred;
        output[o + 3] = k * hundred;
    }
}
/// Converts a batch of `[u8; 3]` RGB pixels to CMYK `[f32; 4]` using
/// the CubeCL/wgpu GPU kernel.
///
/// # Return value
///
/// - `None` when no GPU is available (this host).
/// - `Some(Vec<[f32; 4]>)` when a GPU was acquired and the kernel completed.
///   CMYK has 4 channels: `[c, m, y, k]` each in 0–100 range.
pub fn rgb_to_cmyk_gpu_batch(rgb: &[[u8; 3]]) -> Option<Vec<[f32; 4]>> {
    let n = rgb.len();
    if n == 0 {
        return Some(Vec::new());
    }

    if crate::probe::probe() != crate::probe::Backend::Gpu {
        return None;
    }

    let client =
        std::panic::catch_unwind(|| WgpuRuntime::client(&WgpuDevice::DefaultDevice)).ok()?;

    let n_float_in = n * 3;
    let n_float_out = n * 4;
    let mut flat_input: Vec<f32> = Vec::with_capacity(n_float_in);
    for pixel in rgb {
        flat_input.push(f32::from(pixel[0]));
        flat_input.push(f32::from(pixel[1]));
        flat_input.push(f32::from(pixel[2]));
    }

    let in_handle = client.create_from_slice(bytemuck::cast_slice(&flat_input));
    let out_handle = client.empty(n_float_out * size_of::<f32>());

    let (cube_count, cube_dim) = compute_launch_grid(n);

    // SAFETY: The kernel launch is unsafe per the CubeCL API because the
    // runtime cannot statically verify that the ArrayArg handles match the
    // kernel's expected buffer lengths.  We guarantee:
    //   - `in_handle` has `n_float_in` (= 3 × n) f32 elements
    //   - `out_handle` has `n_float_out` (= 4 × n) f32 elements
    //   - `n_pixels` (= n) is the per-thread bounds-check constant
    //   - `cube_count` covers all n pixels (div_ceil ensures no underflow)
    unsafe {
        rgb_to_cmyk_kernel::launch(
            &client,
            cube_count,
            cube_dim,
            ArrayArg::from_raw_parts(in_handle.clone(), n_float_in),
            ArrayArg::from_raw_parts(out_handle.clone(), n_float_out),
            n,
        );
    }

    let bytes = match client.read_one(out_handle) {
        Ok(b) => b,
        Err(_) => return None,
    };
    let result_f32: &[f32] = bytemuck::cast_slice(&bytes);

    let mut result: Vec<[f32; 4]> = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 4;
        result.push([
            result_f32[base],
            result_f32[base + 1],
            result_f32[base + 2],
            result_f32[base + 3],
        ]);
    }

    Some(result)
}
