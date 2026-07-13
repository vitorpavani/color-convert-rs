//! Vector tests for the `gray` source routes (issue #16).
//!
//! Each test drives one `color_convert_rs::gray::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/gray_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! Gray takes a single channel (0..=100) representing the gray intensity.
//! API for all routes: functions accept `[f64; 1]` and return the appropriate
//! output type (`[f64; N]` for numeric, `String` for hex).
//!
//! Tolerance: 0.0. All channels are integer-valued after rounding, mirroring
//! the JS public wrapper's per-channel `Math.round`.

mod harness;

use color_convert_rs::gray;
use harness::{VecValue, assert_cases, load_route};

/// Extracts a `[f64; 1]` gray channel from a `VecValue::Nums` input.
fn gray_input(value: &VecValue) -> [f64; 1] {
    let VecValue::Nums(nums) = value else {
        panic!("gray vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("gray input must have exactly 1 channel, got {c:?}"))
}

// ── gray → rgb ───────────────────────────────────────────────────────────────

#[test]
fn gray_to_rgb_matches_js_vectors() {
    let vectors = load_route("gray", "rgb");
    assert_cases("gray_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = gray::rgb(gray_input(input));
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}

// ── gray → hsl ───────────────────────────────────────────────────────────────

#[test]
fn gray_to_hsl_matches_js_vectors() {
    let vectors = load_route("gray", "hsl");
    assert_cases("gray_to_hsl", &vectors.cases, 0.0, |input| {
        let [h, s, l] = gray::hsl(gray_input(input));
        VecValue::Nums(vec![h.round(), s.round(), l.round()])
    });
}

// ── gray → hsv ───────────────────────────────────────────────────────────────

#[test]
fn gray_to_hsv_matches_js_vectors() {
    let vectors = load_route("gray", "hsv");
    assert_cases("gray_to_hsv", &vectors.cases, 0.0, |input| {
        let [h, s, v] = gray::hsv(gray_input(input));
        VecValue::Nums(vec![h.round(), s.round(), v.round()])
    });
}

// ── gray → hwb ───────────────────────────────────────────────────────────────

#[test]
fn gray_to_hwb_matches_js_vectors() {
    let vectors = load_route("gray", "hwb");
    assert_cases("gray_to_hwb", &vectors.cases, 0.0, |input| {
        let [h, w, b] = gray::hwb(gray_input(input));
        VecValue::Nums(vec![h.round(), w.round(), b.round()])
    });
}

// ── gray → cmyk ──────────────────────────────────────────────────────────────

#[test]
fn gray_to_cmyk_matches_js_vectors() {
    let vectors = load_route("gray", "cmyk");
    assert_cases("gray_to_cmyk", &vectors.cases, 0.0, |input| {
        let [c, m, y, k] = gray::cmyk(gray_input(input));
        VecValue::Nums(vec![c.round(), m.round(), y.round(), k.round()])
    });
}
