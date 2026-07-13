//! Vector tests for the `oklch` source routes (issue #13).
//!
//! Each test drives one `color_convert_rs::oklch::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/oklch_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `oklch::oklab(oklch: [f64; 3]) -> [f64; 3]` returning
//! raw (unrounded) floats `[l (0-100), a, b]`, mirroring
//! `convert.oklch.oklab = convert.lch.lab` in color-convert's conversions.js
//! (line 581–583). Since oklch→oklab delegates to the same lch.lab math (lines
//! 631–641), the algorithm is:
//!   l = oklch[0]; hr = h/360*2π; a = c*cos(hr); b = c*sin(hr)
//!
//! Tolerance: 0.0. The `a` and `b` channels can be negative, so Rust's
//! `f64::round` (half away from zero) does NOT match JS `Math.round` (half
//! toward +infinity). Use `js_round` below for exact JS semantics.

mod harness;

use color_convert_rs::oklch;
use harness::{VecValue, assert_cases, load_route};

/// JS-compatible rounding: half toward +infinity (`Math.round` semantics),
/// needed when channels can be negative (oklab a, b).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Extracts an `[f64; 3]` oklch triple from a `VecValue::Nums` input.
/// Channels are (l: 0-100, c, h: 0-360) per the JS reference.
fn oklch_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("oklch vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("oklch input must have exactly 3 channels, got {c:?}"))
}

// ── oklch → oklab ────────────────────────────────────────────────────────────
//
// API pinned for GREEN: `oklch::oklab(oklch: [f64; 3]) -> [f64; 3]` returning
// raw floats `[l (0-100), a, b]`. See module doc for the math from
// `convert.oklch.oklab = convert.lch.lab` (conversions.js lines 581–583,
// 631–641). Tolerance: 0.0 with js_round (a, b can be negative).

#[test]
fn oklch_to_oklab_matches_js_vectors() {
    let vectors = load_route("oklch", "oklab");
    assert_cases("oklch_to_oklab", &vectors.cases, 0.0, |input| {
        let [l, a, b] = oklch::oklab(oklch_input(input));
        // Use js_round for JS Math.round semantics (a, b can be negative).
        VecValue::Nums(vec![js_round(l), js_round(a), js_round(b)])
    });
}
