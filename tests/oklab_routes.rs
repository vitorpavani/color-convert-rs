//! Vector tests for the `oklab` source routes (issue #13).
//!
//! Each test drives one `color_convert_rs::oklab::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/oklab_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `oklab::oklch(oklab: [f64; 3]) -> [f64; 3]` returning
//! raw (unrounded) floats `[l (0-100), c (non-neg), h (0-360)]`, mirroring
//! `convert.oklab.oklch = convert.lab.lch` in color-convert's conversions.js
//! (line 544–546). Since oklab→oklch delegates to the same lab.lch math, the
//! algorithm is:
//!   l = oklab[0]; c = sqrt(a² + b²); h = atan2(b,a)*360/2π; if h<0 { h+=360 }
//!
//! Tolerance: 0.0. All channels (l, c, h) are non-negative within their
//! observable ranges, so Rust's `f64::round` (half away from zero) matches JS
//! `Math.round` (half toward +infinity) semantics exactly. The test rounds here
//! to match the JS public wrapper's per-channel `Math.round`.

mod harness;

use color_convert_rs::oklab;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` oklab triple from a `VecValue::Nums` input.
/// Channels are (l: 0-100, a, b) per the JS reference.
fn oklab_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("oklab vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("oklab input must have exactly 3 channels, got {c:?}"))
}

// ── oklab → oklch ────────────────────────────────────────────────────────────

#[test]
fn oklab_to_oklch_matches_js_vectors() {
    let vectors = load_route("oklab", "oklch");
    assert_cases("oklab_to_oklch", &vectors.cases, 0.0, |input| {
        let [l, c, h] = oklab::oklch(oklab_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (all values are non-negative).
        VecValue::Nums(vec![l.round(), c.round(), h.round()])
    });
}
