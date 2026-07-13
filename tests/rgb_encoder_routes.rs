//! Vector tests for the `rgb` encoder routes (issue #6).
//!
//! Each test drives one `color_convert_rs::rgb::<encoder>` conversion against
//! the committed JS-generated vectors (`tests/vectors/rgb_to_<encoder>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! Encoder routes produce string or structured outputs (not numeric arrays)
//! and are tested here separately from the numeric decoder routes in
//! `tests/rgb_routes.rs` (issue #5).

mod harness;

use color_convert_rs::rgb;
use harness::{VecValue, assert_cases, load_route};

/// Extracts a `[u8; 3]` RGB triple from a `VecValue::Nums` input.
///
/// Local helper — `rgb_routes.rs` has an identical copy because it is a
/// separate test-crate binary. The harness does not currently expose a
/// shared `rgb_input` utility (and adding one would be a speculatively
/// broad change for a different agent/session).
fn rgb_input(value: &VecValue) -> [u8; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("rgb vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<u8> = nums
        .iter()
        .map(|&n| {
            assert!(
                n.fract() == 0.0 && (0.0..=255.0).contains(&n),
                "rgb channel out of u8 range: {n}"
            );
            n as u8
        })
        .collect();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("rgb input must have exactly 3 channels, got {c:?}"))
}

// ── rgb → hex ─────────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::hex(rgb: [u8; 3]) -> String` returning an
/// UPPERCASE 6-digit hex string (e.g. `"8CC864"`), mirroring
/// `convert.rgb.hex` in color-convert's conversions.js (lines 746–755).
///
/// The JS algorithm:
///
/// ```text
/// integer = ((round(r) & 0xFF) << 16)
///         + ((round(g) & 0xFF) << 8)
///         +  (round(b) & 0xFF);
/// string = integer.toString(16).toUpperCase();
/// pad-left with '0' to 6 characters
/// ```
///
/// The hex output is a **string** — comparison is exact (`==`).  Tolerance
/// is irrelevant for string comparison but is passed as `0.0` to satisfy
/// the `assert_cases` API.
///
/// The JS code rounds each channel before masking.  The `round` here is the
/// JS `Math.round` called inside `convert.rgb.hex`, not the public wrapper's
/// per-channel rounding (that wrapper is a pass‑through for hex). RGB
/// inputs are u8 integers so rounding is a no-op for all 32 vector cases —
/// this test accepts the raw `[u8; 3]` directly.
#[test]
fn rgb_to_hex_matches_js_vectors() {
    let vectors = load_route("rgb", "hex");
    assert_cases("rgb_to_hex", &vectors.cases, 0.0, |input| {
        let s = rgb::hex(rgb_input(input));
        // Hex output is a string — value comparison is exact (see harness
        // `matches_within` for `Text` variants).
        VecValue::Text(s)
    });
}
