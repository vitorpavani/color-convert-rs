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
const M21: f32 = 0.1191920;
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
    let client = std::panic::catch_unwind(|| {
        WgpuRuntime::client(&WgpuDevice::DefaultDevice)
    })
    .ok()?;

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
    let cube_dim = CubeDim::new_1d(64);
    let n_u32: u32 = n.try_into().expect("n fits in u32 for cube count");
    let cube_count = CubeCount::new_1d(
        n_u32.div_ceil(cube_dim.x),
    );

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
