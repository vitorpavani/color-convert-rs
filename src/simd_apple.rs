//! CPU-SIMD batch conversion for rgb→apple using per-channel linear scale.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→apple ([`rgb_to_apple_batch`])
//!
//! The scalar reference is [`crate::rgb::apple`], which computes
//! `(channel / 255.0) × 65535.0 = channel × 257.0` per channel.
//! This matches the JS `convert.rgb.apple` (color-convert@3.1.3,
//! conversions.js line 939).
//!
//! This SIMD path loads 8 pixels' worth of each RGB channel into
//! separate `f32x8` lanes (AoS→SoA gather), multiplies by 257.0
//! (exact f32), and scatters back to AoS output.
//!
//! ## Tolerance
//!
//! All apple output values are integer multiples of 257.0, maximum
//! 65535. Both f32 (24-bit mantissa, 16,777,216 exact-integer range)
//! and f64 represent every possible output exactly. Tolerance is
//! effectively 0.0; 1e-6 accounts for f32→f64 cast noise in the
//! comparison pipeline.
//!
//! See `tests/simd_apple_routes.rs`.

use wide::f32x8;

/// Process a batch of RGB pixels into Apple RGB (16-bit channels) via SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. The apple
/// conversion is `channel × (65535.0 / 255.0) = channel × 257.0` —
/// a trivial per-channel linear scale. Remainder pixels (final 0–7)
/// fall back to the scalar [`crate::rgb::apple`], converting its f64
/// output to f32.
///
/// # Memory access pattern
///
/// Each 8-pixel tile performs 24 scalar u8→f32 loads (AoS→SoA gather),
/// one f32x8 multiply per channel, and 24 f32 stores (SoA→AoS scatter).
/// For a single arithmetic op per channel this is strongly
/// memory-bandwidth-bound — the gather/scatter overhead may dominate.
///
/// # Accuracy
///
/// Outputs are exact for all u8 inputs (tol ≤ 1e-6 vs scalar f64).
/// See module-level docs for the tolerance analysis.
pub fn rgb_to_apple_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let factor = f32x8::splat(257.0);

    while i + 7 < n {
        // Load 8 pixels' channels into separate SIMD lanes (AoS → SoA gather)
        let r = f32x8::new([
            rgb[i][0] as f32,
            rgb[i + 1][0] as f32,
            rgb[i + 2][0] as f32,
            rgb[i + 3][0] as f32,
            rgb[i + 4][0] as f32,
            rgb[i + 5][0] as f32,
            rgb[i + 6][0] as f32,
            rgb[i + 7][0] as f32,
        ]);
        let g = f32x8::new([
            rgb[i][1] as f32,
            rgb[i + 1][1] as f32,
            rgb[i + 2][1] as f32,
            rgb[i + 3][1] as f32,
            rgb[i + 4][1] as f32,
            rgb[i + 5][1] as f32,
            rgb[i + 6][1] as f32,
            rgb[i + 7][1] as f32,
        ]);
        let b = f32x8::new([
            rgb[i][2] as f32,
            rgb[i + 1][2] as f32,
            rgb[i + 2][2] as f32,
            rgb[i + 3][2] as f32,
            rgb[i + 4][2] as f32,
            rgb[i + 5][2] as f32,
            rgb[i + 6][2] as f32,
            rgb[i + 7][2] as f32,
        ]);

        // Apply linear scale: channel * 257.0
        let r_out = r * factor;
        let g_out = g * factor;
        let b_out = b * factor;

        // Scatter back to AoS
        let r_arr = r_out.to_array();
        let g_arr = g_out.to_array();
        let b_arr = b_out.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32
    while i < n {
        let scalar = crate::rgb::apple(rgb[i]);
        result.push([scalar[0] as f32, scalar[1] as f32, scalar[2] as f32]);
        i += 1;
    }

    result
}
