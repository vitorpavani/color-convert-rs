//! Vector tests for the `lch` source routes (issue #12).
//!
//! Each test drives one `color_convert_rs::lch::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/lch_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `lch::lab(lch: [f64; 3]) -> [f64; 3]` returning raw
//! floats `[l (0-100), a (~-128..127), b (~-128..127)]`, mirroring
//! `convert.lch.lab` in color-convert's conversions.js (lines 631–642).
//! The JS reference computes:
//!   l = lch[0]; c = lch[1]; h = lch[2];
//!   hr = h / 360 * 2 * PI;
//!   a = c * cos(hr); b = c * sin(hr);
//!   return [l, a, b]
//!
//! Tolerance: 0.0. The `a` and `b` channels can be negative, so the test
//! uses `js_round` (matching JS `Math.round`) instead of `f64::round`.

mod harness;

use harness::{VecValue, assert_cases, load_route};

/// JS-compatible rounding: `Math.floor(x + 0.5)`, matching JS `Math.round`
/// for negative values (e.g. `-0.5` → `0` instead of `-1`).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Extracts an `[f64; 3]` LCH triple from a `VecValue::Nums` input.
/// Channels are (l: 0-100, c: 0-~134, h: 0-360) per the JS reference.
fn lch_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("lch vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("lch input must have exactly 3 channels, got {c:?}"))
}

// ── lch → lab ────────────────────────────────────────────────────────────────

#[test]
fn lch_to_lab_matches_js_vectors() {
    let vectors = load_route("lch", "lab");
    assert_cases("lch_to_lab", &vectors.cases, 0.0, |input| {
        let lch = lch_input(input);
        let [l, a, b] = color_convert_rs::lch::lab(lch);
        // Mirror the JS public wrapper's per-channel Math.round.
        // a and b can be negative, so use js_round (not f64::round).
        VecValue::Nums(vec![l.round(), js_round(a), js_round(b)])
    });
}
