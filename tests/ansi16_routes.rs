//! Vector tests for the `ansi16` decoder route (issue #15).
//!
//! Each test drives `color_convert_rs::ansi16::rgb` against the committed
//! JS-generated vectors (`tests/vectors/ansi16_to_rgb.json`, source:
//! color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! ANSI‑16 inputs are integer codes (`VecValue::Num`).  The closure rounds
//! the raw `[f64; 3]` channels before comparing against integer-vector
//! expectations at tolerance 0.0.

mod harness;

use color_convert_rs::ansi16;
use harness::{VecValue, assert_cases, load_route};

/// API pinned for GREEN: `ansi16::rgb(code: u16) -> [f64; 3]` returning raw
/// (unrounded) RGB floats, mirroring `convert.ansi16.rgb` in color-convert's
/// conversions.js (lines 701–725).
///
/// Algorithm:
///
/// ```text
/// 1. color = code % 10
/// 2. Greyscale (color 0 or 7):
///     if code > 50 → color += 3.5  (bright)
///     channel = color / 10.5 * 255
///     return [channel, channel, channel]
/// 3. Chromatic:
///     mult = ((code > 50 ? 1 : 0) + 1) * 0.5
///     r = ((color & 1)     * mult) * 255
///     g = (((color >> 1) & 1) * mult) * 255
///     b = (((color >> 2) & 1) * mult) * 255
///     return [r, g, b]
/// ```
///
/// The output channels are integer-valued after rounding — comparison is
/// exact at tolerance 0.0.
///
/// Vector: `tests/vectors/ansi16_to_rgb.json` (32 cases).
#[test]
fn ansi16_to_rgb_matches_js_vectors() {
    let vectors = load_route("ansi16", "rgb");
    assert_cases("ansi16_to_rgb", &vectors.cases, 0.0, |input| {
        let VecValue::Num(code) = input else {
            panic!("ansi16 input must be VecValue::Num, got {input:?}");
        };
        let rgb = ansi16::rgb(*code as u16);
        VecValue::Nums(vec![rgb[0].round(), rgb[1].round(), rgb[2].round()])
    });
}
