//! Vector tests for the `ansi256` decoder route (issue #15).
//!
//! Each test drives `color_convert_rs::ansi256::rgb` against the committed
//! JS-generated vectors (`tests/vectors/ansi256_to_rgb.json`, source:
//! color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! ANSI‑256 inputs are integer codes (`VecValue::Num`).  Output channels may
//! be **negative** for codes 0–15 (faithful to the JS reference).  The
//! closure rounds the raw `[f64; 3]` before comparing against integer-vector
//! expectations at tolerance 0.0.

mod harness;

use color_convert_rs::ansi256;
use harness::{VecValue, assert_cases, load_route};

/// API pinned for GREEN: `ansi256::rgb(code: u16) -> [f64; 3]` returning raw
/// (unrounded) RGB floats, mirroring `convert.ansi256.rgb` in color-convert's
/// conversions.js (lines 727–745).
///
/// Algorithm:
///
/// ```text
/// 1. Greyscale (code ≥ 232):
///     c = (code - 232) * 10 + 8
///     return [c, c, c]
/// 2. Colour cube (code 16–231):
///     v = code - 16
///     r = floor(v / 36) / 5 * 255
///     g = floor((v % 36) / 6) / 5 * 255
///     b = (v % 6) / 5 * 255
///     return [r, g, b]
/// 3. Codes 0–15 use the same cube formula (v is negative), yielding
///    negative channel values that match the JS reference.
/// ```
///
/// The output channels are integer-valued after rounding — comparison is
/// exact at tolerance 0.0.
///
/// Vector: `tests/vectors/ansi256_to_rgb.json` (10 cases, including negative
/// outputs for codes 0 and 8).
#[test]
fn ansi256_to_rgb_matches_js_vectors() {
    let vectors = load_route("ansi256", "rgb");
    assert_cases("ansi256_to_rgb", &vectors.cases, 0.0, |input| {
        let VecValue::Num(code) = input else {
            panic!("ansi256 input must be VecValue::Num, got {input:?}");
        };
        let rgb = ansi256::rgb(*code as u16);
        VecValue::Nums(vec![rgb[0].round(), rgb[1].round(), rgb[2].round()])
    });
}
