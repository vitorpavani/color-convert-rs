//! CPU-SIMD batch conversion for rgb→hsv using mask-blend selection.
//!
//! Uses [`wide::f32x8`] to process 8 pixels at once.
//!
//! ## rgb→hsv ([`rgb_to_hsv_batch`])
//!
//! The scalar reference is [`crate::rgb::hsv`], which normalises RGB to
//! [0,1], finds min/max/delta, computes v=max, s=delta/max (0 if diff==0),
//! hue via a 3-way branch on which channel is max, and scales to
//! h∈[0,360], s∈[0,100], v∈[0,100]. This SIMD path replaces branching
//! with mask-blend on all three candidate hue expressions. The achromatic
//! case (max==min) and max==0 case force hue and s to zero via blend.
//!
//! ## Tolerance
//!
//! Each SIMD lane uses f32 (~7 decimal digits) vs scalar f64 (~15).
//! The hue calculation involves division by delta (as small as 1/255≈0.004),
//! amplifying the initial f32 representation error.
//! - h (0–360): absolute tolerance ≤ 1e-3
//! - s (0–100): absolute tolerance ≤ 1e-3
//! - v (0–100): absolute tolerance ≤ 1e-3
//!
//! See `tests/simd_hsv_routes.rs`.

/// Process a batch of RGB pixels into HSV via mask-blend SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. All three
/// candidate hue expressions are computed concurrently and selected
/// with `blend` using the channel-maximum masks, avoiding per-pixel
/// branching. Remainder pixels (final 0–7) fall back to the scalar
/// [`crate::rgb::hsv`], converting its f64 output to f32.
///
/// # Mask-blend strategy
///
/// The JS reference uses an if/else-if chain to select the hue formula
/// based on which channel is the maximum. We compute all three for all
/// 8 lanes simultaneously and use SIMD blend to select:
///
/// 1. Compute `hue_r`, `hue_g`, `hue_b` — the three candidate expressions.
/// 2. Build masks `mask_r = (max == r)`, `mask_g = (max == g)`.
/// 3. Select: start with `hue_b` (JS "else"), blend in `hue_g` where
///    `mask_g` is true, then blend in `hue_r` where `mask_r` is true.
///    This mirrors the JS precedence: r checked first, then g, else b.
///
/// Achromatic pixels (max==min) have their hue AND saturation forced to
/// zero via a final blend, matching the JS `if (diff == 0) { h = 0; s = 0 }`.
#[allow(unused_variables)]
pub fn rgb_to_hsv_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    unimplemented!("rgb_to_hsv_batch — stub for RED phase")
}
