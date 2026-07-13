//! Vector tests for the `rgb` source routes (issue #5).
//!
//! Each test drives one `color_convert_rs::rgb::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/rgb_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `rgb::hsl(rgb: [u8; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[h (0-360), s (0-100), l (0-100)]`, mirroring
//! `convert.rgb.hsl` in color-convert's conversions.js. The signature is
//! infallible (`[f64; 3]`, not `Result`) because every `[u8; 3]` input is a
//! valid RGB triple; `Result<_, Error>` is reserved for fallible parses such
//! as hex→rgb. The vectors store the *observable* output of the JS public
//! wrapper, which applies `Math.round` per channel — so the test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all hsl channels are
//! non-negative, so the semantics coincide on this route.

mod harness;

use color_convert_rs::rgb;
use harness::{VecValue, assert_cases, load_route};

/// Extracts a `[u8; 3]` RGB triple from a `VecValue::Nums` input.
fn rgb_input(value: &VecValue) -> [u8; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("rgb vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<u8> = nums
        .iter()
        .map(|&n| {
            assert!(
                n.fract() == 0.0 && (0.0..=255.0).contains(&n),
                "rgb channel out of u8 range: {n}"
            );
            n as u8
        })
        .collect();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("rgb input must have exactly 3 channels, got {c:?}"))
}

#[test]
fn rgb_to_hsl_matches_js_vectors() {
    let vectors = load_route("rgb", "hsl");
    assert_cases("rgb_to_hsl", &vectors.cases, 0.0, |input| {
        let [h, s, l] = rgb::hsl(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![h.round(), s.round(), l.round()])
    });
}

/// API pinned for GREEN: `rgb::hsv(rgb: [u8; 3]) -> [f64; 3]` returning RAW
/// (unrounded) floats `[h (0-360), s (0-100), v (0-100)]`, mirroring
/// `convert.rgb.hsv` in color-convert's conversions.js (lines 128-186).
///
/// Tolerance: 0.0 after per-channel rounding, exactly as rgb→hsl above. All
/// hsv channels are non-negative, so Rust's half-away-from-zero `f64::round`
/// coincides with JS `Math.round` (half toward +infinity) on this route.
#[test]
fn rgb_to_hsv_matches_js_vectors() {
    let vectors = load_route("rgb", "hsv");
    assert_cases("rgb_to_hsv", &vectors.cases, 0.0, |input| {
        let [h, s, v] = rgb::hsv(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![h.round(), s.round(), v.round()])
    });
}
