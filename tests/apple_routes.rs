//! Vector tests for the `apple` source routes (issue #16).
//!
//! Each test drives one `color_convert_rs::apple::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/apple_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! Note: `rgb→apple` already exists in `src/rgb.rs` (issue #5) — this file
//! only covers the `apple→rgb` direction.
//!
//! API pinned for GREEN: `apple::rgb(apple: [f64; 3]) -> [f64; 3]` returning
//! raw (unrounded) floats `[r (0-255), g (0-255), b (0-255)]`. The formula is
//! `(channel / 65535.0) * 255.0` per channel, mirroring `convert.apple.rgb`
//! (conversions.js lines 937–939).
//!
//! Tolerance: 0.0. The test rounds each output channel (mirroring the JS
//! public wrapper's `Math.round`) before comparing against the integer-valued
//! vector expectations.

mod harness;

use color_convert_rs::apple;
use harness::{VecValue, assert_cases, load_route};

/// Extracts an `[f64; 3]` Apple RGB triple from a `VecValue::Nums` input.
/// Channels are 0..=65535 unsigned 16-bit values.
fn apple_input(value: &VecValue) -> [f64; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("apple vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<f64> = nums.to_vec();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("apple input must have exactly 3 channels, got {c:?}"))
}

// ── apple → rgb ──────────────────────────────────────────────────────────────

#[test]
fn apple_to_rgb_matches_js_vectors() {
    let vectors = load_route("apple", "rgb");
    assert_cases("apple_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = apple::rgb(apple_input(input));
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}
