//! Vector tests for the `lab` source routes (issue #12).
//!
//! Each test drives one `color_convert_rs::lab::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/lab_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `lab::xyz(lab: [f64; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[x (0-95), y (0-100), z (0-108)]`, mirroring
//! `convert.lab.xyz` in color-convert's conversions.js (lines 585–610).
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. xyz channels are non-negative so `f64::round`
//! matches JS `Math.round`.

mod harness;

use color_convert_rs::lab;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` LAB triple from a `VecValue::Nums` input.
/// Channels are (l: 0-100, a: approx -128..127, b: approx -128..127)
/// per the JS reference.
fn lab_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("lab vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("lab input must have exactly 3 channels, got {c:?}"))
}

// ── lab → xyz ────────────────────────────────────────────────────────────────

#[test]
fn lab_to_xyz_matches_js_vectors() {
    let vectors = load_route("lab", "xyz");
    assert_cases("lab_to_xyz", &vectors.cases, 0.0, |input| {
        let [x, y, z] = lab::xyz(lab_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        // xyz channels are non-negative, so f64::round ≡ JS Math.round.
        VecValue::Nums(vec![x.round(), y.round(), z.round()])
    });
}

// ── lab → lch ────────────────────────────────────────────────────────────────
//
// API pinned for GREEN: `lab::lch(lab: [f64; 3]) -> [f64; 3]` returning raw
// floats `[l (0-100), c (0-approx 134), h (0-360)]`, mirroring
// `convert.lab.lch` in color-convert's conversions.js (lines 613–629).
// The JS reference computes:
//   l = lab[0]; a = lab[1]; b = lab[2];
//   hr = Math.atan2(b, a); h = hr * 360 / 2 / PI;
//   if h < 0 { h += 360 }
//   c = sqrt(a*a + b*b); return [l, c, h]
// Tolerance: 0.0. After per-channel rounding the output must match the
// rounded JS vector EXACTLY. All channels (l, c, h) are non-negative.

#[test]
fn lab_to_lch_matches_js_vectors() {
    let vectors = load_route("lab", "lch");
    assert_cases("lab_to_lch", &vectors.cases, 0.0, |input| {
        let [l, c, h] = lab::lch(lab_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        // All channels are non-negative, so f64::round ≡ JS Math.round.
        VecValue::Nums(vec![l.round(), c.round(), h.round()])
    });
}
