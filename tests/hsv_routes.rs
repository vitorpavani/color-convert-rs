//! Vector tests for the `hsv` source routes (issue #8).
//!
//! Each test drives one `color_convert_rs::hsv::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/hsv_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `hsv::rgb(hsv: [f64; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[r (0-255), g (0-255), b (0-255)]`, mirroring
//! `convert.hsv.rgb` in color-convert's conversions.js (lines 363–401). The
//! signature takes `[f64; 3]` (not `[u8; 3]`) because hue ranges 0-360 and
//! saturation/value range 0-100 — these exceed u8 bounds. The signature is
//! infallible (`[f64; 3]`, not `Result`) because every valid HSV triple
//! converts to a valid RGB triple; `Result<_, Error>` is reserved for fallible
//! parses. The vectors store the *observable* output of the JS public wrapper,
//! which applies `Math.round` per channel — so the test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all rgb channels are
//! non-negative, so the semantics coincide on this route.

mod harness;

use color_convert_rs::hsv;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` HSV triple from a `VecValue::Nums` input.
/// Channels are (h: 0-360, s: 0-100, v: 0-100) per the JS reference.
fn hsv_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("hsv vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("hsv input must have exactly 3 channels, got {c:?}"))
}

// ── hsv → rgb ───────────────────────────────────────────────────────────────

#[test]
fn hsv_to_rgb_matches_js_vectors() {
    let vectors = load_route("hsv", "rgb");
    assert_cases("hsv_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = hsv::rgb(hsv_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}
