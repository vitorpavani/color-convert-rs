//! Vector tests for the `cmyk` source routes (issue #10).
//!
//! Each test drives one `color_convert_rs::cmyk::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/cmyk_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `cmyk::rgb(cmyk: [f64; 4]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[r (0-255), g (0-255), b (0-255)]`, mirroring
//! `convert.cmyk.rgb` in color-convert's conversions.js (lines 475–486). The
//! signature takes `[f64; 4]` (four channels: c, m, y, k each 0-100). The
//! signature is infallible (`[f64; 3]`, not `Result`) because every valid CMYK
//! quadruple converts to a valid RGB triple. The vectors store the *observable*
//! output of the JS public wrapper, which applies `Math.round` per channel — so
//! the test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all rgb channels are
//! non-negative, so the semantics coincide on this route.

mod harness;

use color_convert_rs::cmyk;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 4]` CMYK quadruple from a `VecValue::Nums` input.
/// Channels are (c: 0-100, m: 0-100, y: 0-100, k: 0-100) per the JS reference.
fn cmyk_input(value: &VecValue) -> [f64; 4] {
    let VecValue::Nums(nums) = value else {
        panic!("cmyk vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("cmyk input must have exactly 4 channels, got {c:?}"))
}

// ── cmyk → rgb ───────────────────────────────────────────────────────────────

#[test]
fn cmyk_to_rgb_matches_js_vectors() {
    let vectors = load_route("cmyk", "rgb");
    assert_cases("cmyk_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = cmyk::rgb(cmyk_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}
