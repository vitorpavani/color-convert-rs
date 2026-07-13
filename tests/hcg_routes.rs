//! Vector tests for the `hcg` source routes (issue #14).
//!
//! Each test drives one `color_convert_rs::hcg::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/hcg_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! ## API
//!
//! Each function takes `[f64; 3]` (h: 0-360, c: 0-100, g: 0-100) and returns
//! the target-space triple as raw (unrounded) floats. The vectors store the
//! *observable* output of the JS public wrapper, which applies `Math.round`
//! per channel — so the test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all output channels on every
//! HCG route are non-negative, so the semantics coincide.
//!
//! ## Routes (4)
//!
//! | Route   | JS reference                | conversions.js lines |
//! |---------|-----------------------------|----------------------|
//! | hcg→rgb | `convert.hcg.rgb`           | 834–884              |
//! | hcg→hsv | `convert.hcg.hsv`           | 886–898              |
//! | hcg→hsl | `convert.hcg.hsl`           | 900–914              |
//! | hcg→hwb | `convert.hcg.hwb`           | 916–921              |

mod harness;

use color_convert_rs::hcg;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` HCG triple from a `VecValue::Nums` input.
/// Channels are (h: 0-360, c: 0-100, g: 0-100) per the JS reference.
fn hcg_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("hcg vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("hcg input must have exactly 3 channels, got {c:?}"))
}

// ── hcg → rgb ───────────────────────────────────────────────────────────────

#[test]
fn hcg_to_rgb_matches_js_vectors() {
    let vectors = load_route("hcg", "rgb");
    assert_cases("hcg_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = hcg::rgb(hcg_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}

// ── hcg → hsv ───────────────────────────────────────────────────────────────

#[test]
fn hcg_to_hsv_matches_js_vectors() {
    let vectors = load_route("hcg", "hsv");
    assert_cases("hcg_to_hsv", &vectors.cases, 0.0, |input| {
        let [h, s, v] = hcg::hsv(hcg_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        VecValue::Nums(vec![h.round(), s.round(), v.round()])
    });
}

// ── hcg → hsl ───────────────────────────────────────────────────────────────

#[test]
fn hcg_to_hsl_matches_js_vectors() {
    let vectors = load_route("hcg", "hsl");
    assert_cases("hcg_to_hsl", &vectors.cases, 0.0, |input| {
        let [h, s, l] = hcg::hsl(hcg_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        VecValue::Nums(vec![h.round(), s.round(), l.round()])
    });
}

// ── hcg → hwb ───────────────────────────────────────────────────────────────

#[test]
fn hcg_to_hwb_matches_js_vectors() {
    let vectors = load_route("hcg", "hwb");
    assert_cases("hcg_to_hwb", &vectors.cases, 0.0, |input| {
        let [h, w, b] = hcg::hwb(hcg_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        VecValue::Nums(vec![h.round(), w.round(), b.round()])
    });
}
