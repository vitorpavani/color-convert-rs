//! Vector tests for the `keyword` source routes (issue #16).
//!
//! Each test drives one `color_convert_rs::keyword::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/keyword_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `keyword::rgb(name: &str) -> [f64; 3]` returning
//! `[r, g, b]` as f64 channels in 0-255, mirroring `convert.keyword.rgb` in
//! color-convert's conversions.js. The function looks up `name` in the
//! vendored `crate::color_name::CSS_COLORS` table.
//!
//! Tolerance: 0.0. All channels are integer-valued, and f64::round() produces
//! exact results.

mod harness;

use color_convert_rs::keyword;
use harness::{VecValue, assert_cases, load_route};

/// Extracts a keyword string from a `VecValue::Text` input.
fn keyword_input(value: &VecValue) -> &str {
    let VecValue::Text(s) = value else {
        panic!("keyword vector input must be VecValue::Text, got {value:?}");
    };
    s.as_str()
}

// ── keyword → rgb ────────────────────────────────────────────────────────────

#[test]
fn keyword_to_rgb_matches_js_vectors() {
    let vectors = load_route("keyword", "rgb");
    assert_cases("keyword_to_rgb", &vectors.cases, 0.0, |input| {
        let [r, g, b] = keyword::rgb(keyword_input(input));
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}
