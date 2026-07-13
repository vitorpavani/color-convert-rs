//! Vector tests for the `hex` decoder route (issue #15).
//!
//! Each test drives `color_convert_rs::hex::rgb` against the committed
//! JS-generated vectors (`tests/vectors/hex_to_rgb.json`, source:
//! color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! Hex inputs are strings (`VecValue::Text`).  The closure rounds the
//! raw `[f64; 3]` channels before comparing against integer-vector
//! expectations at tolerance 0.0.

mod harness;

use color_convert_rs::hex;
use harness::{VecValue, assert_cases, load_route};

/// API pinned for GREEN: `hex::rgb(hex: &str) -> [f64; 3]` returning raw
/// (unrounded) RGB floats, mirroring `convert.hex.rgb` in color-convert's
/// conversions.js (lines 757–777).
///
/// Algorithm:
///
/// ```text
/// 1. Scan for the first run of 6 hex digits (case-insensitive).
/// 2. If none found, scan for the first run of 3 hex digits and double
///    each character.
/// 3. Parse the run as base‑16; extract red (bits 16–23), green (bits
///    8–15), and blue (bits 0–7).
/// 4. If no hex run is found, return [0, 0, 0] (matches JS fallback).
/// ```
///
/// The output channels are integer-valued after rounding — comparison is
/// exact at tolerance 0.0.
///
/// Vector: `tests/vectors/hex_to_rgb.json` (4 cases).
///   - `"000000"`  → `[  0,   0,   0]`
///   - `"8CC864"`  → `[140, 200, 100]`
///   - `"ABC"`     → `[170, 187, 204]`  (3‑char shorthand → doubled)
///   - `"FFFFFF"`  → `[255, 255, 255]`
#[test]
fn hex_to_rgb_matches_js_vectors() {
    let vectors = load_route("hex", "rgb");
    assert_cases("hex_to_rgb", &vectors.cases, 0.0, |input| {
        let VecValue::Text(hex) = input else {
            panic!("hex input must be VecValue::Text, got {input:?}");
        };
        let rgb = hex::rgb(hex);
        VecValue::Nums(vec![rgb[0].round(), rgb[1].round(), rgb[2].round()])
    });
}
