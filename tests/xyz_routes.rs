//! Vector tests for the `xyz` source routes (issue #11).
//!
//! Each test drives one `color_convert_rs::xyz::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/xyz_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: all functions return **raw (unrounded) floats**;
//! the per-channel rounding (`.round()` or `js_round`) applied by the JS
//! public wrapper is performed in the test closure. Comparison tolerance is
//! 0.0 — the output must match EXACTLY after JS-equivalent rounding.
//!
//! Routes that produce possibly-negative channels (lab, oklab) use `js_round`
//! (half toward +∞, matching JS `Math.round`). Routes that produce only
//! non-negative outputs (rgb) use Rust's `f64::round` (equivalent for
//! non-negative values).

mod harness;

use color_convert_rs::xyz;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` XYZ triple from a `VecValue::Nums` input.
fn xyz_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("xyz vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("xyz input must have exactly 3 channels, got {c:?}"))
}

/// JS `Math.round` semantics: half toward +∞.
/// For non-negative inputs this is equivalent to Rust's `f64::round`,
/// but for negative inputs (lab/oklab a,b channels) the two diverge.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

// ── xyz → rgb ────────────────────────────────────────────────────────────────

#[test]
fn xyz_to_rgb_matches_js_vectors() {
    let vectors = load_route("xyz", "rgb");
    assert_cases("xyz_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = xyz::rgb(xyz_input(input));
        // All RGB channels are non-negative → Rust round == JS Math.round.
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}

// ── xyz → lab ────────────────────────────────────────────────────────────────

#[test]
fn xyz_to_lab_matches_js_vectors() {
    let vectors = load_route("xyz", "lab");
    assert_cases("xyz_to_lab", &vectors.cases, 0.0, |input| {
        let [l, a, b] = xyz::lab(xyz_input(input));
        // a and b may be negative → use JS-equivalent rounding (half toward +∞).
        VecValue::Nums(vec![js_round(l), js_round(a), js_round(b)])
    });
}
