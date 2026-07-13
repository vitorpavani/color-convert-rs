//! Vector tests for the `hwb` source routes (issue #9).
//!
//! Each test drives one `color_convert_rs::hwb::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/hwb_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `hwb::rgb(hwb: [f64; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[r (0-255), g (0-255), b (0-255)]`, mirroring
//! `convert.hwb.rgb` in color-convert's conversions.js (lines 421–473). The
//! signature takes `[f64; 3]` (not `[u8; 3]`) because hue ranges 0-360 and
//! whiteness/blackness range 0-100 — these exceed u8 bounds. The signature is
//! infallible (`[f64; 3]`, not `Result`) because every valid HWB triple
//! converts to a valid RGB triple. The vectors store the *observable* output
//! of the JS public wrapper, which applies `Math.round` per channel — so the
//! test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all rgb channels are
//! non-negative, so the semantics coincide on this route.

mod harness;

use color_convert_rs::hwb;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` HWB triple from a `VecValue::Nums` input.
/// Channels are (h: 0-360, w: 0-100, b: 0-100) per the JS reference.
fn hwb_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("hwb vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("hwb input must have exactly 3 channels, got {c:?}"))
}

// ── hwb → rgb ───────────────────────────────────────────────────────────────

#[test]
fn hwb_to_rgb_matches_js_vectors() {
    let vectors = load_route("hwb", "rgb");
    assert_cases("hwb_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = hwb::rgb(hwb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}

// ── hwb → hcg ───────────────────────────────────────────────────────────────
//
// API pinned for GREEN: `hwb::hcg(hwb: [f64; 3]) -> [f64; 3]` returning raw
// floats `[h (0-360), c (0-100), g (0-100)]`, mirroring `convert.hwb.hcg`
// in color-convert's conversions.js (lines 923-935).
// The JS reference computes:
//   w = hwb[1] / 100; b = hwb[2] / 100; v = 1 - b; c = v - w;
//   g = 0; if c < 1 { g = (v - c) / (1 - c) }
//   return [hwb[0], c * 100, g * 100]
// Tolerance: 0.0. After per-channel rounding the output must match the
// rounded JS vector EXACTLY (same rounding-mode note as hwb→rgb applies).

#[test]
fn hwb_to_hcg_matches_js_vectors() {
    let vectors = load_route("hwb", "hcg");
    assert_cases("hwb_to_hcg", &vectors.cases, 0.0, |input| {
        let [h, c, g] = hwb::hcg(hwb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round.
        VecValue::Nums(vec![h.round(), c.round(), g.round()])
    });
}
