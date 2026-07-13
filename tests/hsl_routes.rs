//! Vector tests for the `hsl` source routes (issue #7).
//!
//! Each test drives one `color_convert_rs::hsl::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/hsl_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `hsl::rgb(hsl: [f64; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[r (0-255), g (0-255), b (0-255)]`, mirroring
//! `convert.hsl.rgb` in color-convert's conversions.js (lines 304–345). The
//! signature takes `[f64; 3]` (not `[u8; 3]`) because hue ranges 0-360 and
//! saturation/lightness range 0-100 — these exceed u8 bounds. The signature is
//! infallible (`[f64; 3]`, not `Result`) because every valid HSL triple
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

use color_convert_rs::hsl;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` HSL triple from a `VecValue::Nums` input.
/// Channels are (h: 0-360, s: 0-100, l: 0-100) per the JS reference.
fn hsl_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("hsl vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("hsl input must have exactly 3 channels, got {c:?}"))
}

#[test]
fn hsl_to_rgb_matches_js_vectors() {
    let vectors = load_route("hsl", "rgb");
    assert_cases("hsl_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = hsl::rgb(hsl_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}

// ── hsl → hsv ───────────────────────────────────────────────────────────────

/// HSL→HSV conversion, pinning API `hsl::hsv(hsl: [f64; 3]) -> [f64; 3]`.
///
/// Returns raw (unrounded) floats `[h (0-360), s (0-100), v (0-100)]`.  The
/// JS public wrapper applies per-channel `Math.round`, so the test rounds here.
/// HSV channels are non-negative, therefore Rust's `f64::round` (half away from
/// zero) and JS `Math.round` (half toward +∞) coincide — tolerance 0.0 after
/// rounding.
///
/// Reference: `convert.hsl.hsv` (color-convert@3.1.3, conversions.js 347–361).
#[test]
fn hsl_to_hsv_matches_js_vectors() {
    let vectors = load_route("hsl", "hsv");
    assert_cases("hsl_to_hsv", &vectors.cases, 0.0, |input| {
        let [h, s, v] = hsl::hsv(hsl_input(input));
        VecValue::Nums(vec![h.round(), s.round(), v.round()])
    });
}

// ── hsl → hcg ───────────────────────────────────────────────────────────────

/// HSL→HCG conversion, pinning API `hsl::hcg(hsl: [f64; 3]) -> [f64; 3]`.
///
/// Returns raw (unrounded) floats `[h (0-360), c (0-100), g (0-100)]`. The
/// JS public wrapper applies per-channel `Math.round`, so the test rounds here.
/// HCG channels are non-negative, therefore Rust's `f64::round` (half away from
/// zero) and JS `Math.round` (half toward +∞) coincide — tolerance 0.0 after
/// rounding.
///
/// Reference: `convert.hsl.hcg` (color-convert@3.1.3, conversions.js 806–818):
///   s = hsl[1]/100; l = hsl[2]/100;
///   c = l < 0.5 ? 2*s*l : 2*s*(1-l);
///   f = 0; if c < 1 { f = (l - 0.5*c) / (1-c) };
///   return [hsl[0], c*100, f*100];
#[test]
fn hsl_to_hcg_matches_js_vectors() {
    let vectors = load_route("hsl", "hcg");
    assert_cases("hsl_to_hcg", &vectors.cases, 0.0, |input| {
        let [h, c, g] = hsl::hcg(hsl_input(input));
        VecValue::Nums(vec![h.round(), c.round(), g.round()])
    });
}
