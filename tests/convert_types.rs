//! Tests for the `convert` API's `Color` and `Model` types (issue #17).
//!
//! `Color::round` must mirror JavaScript `Math.round` semantics:
//! half toward positive infinity (not Rust's default half-away-from-zero).
//! For non-negative floats the two coincide, but negative values diverge:
//! JS `Math.round(-1.5) === -1`, Rust `(-1.5_f64).round() === -2.0`.
//!
//! Tolerance: 0.0 (exact comparison after manual JS-round emulation).

use color_convert_rs::{Color, Model};

/// JS `Math.round` emulation: rounds half toward +infinity.
/// This matches JavaScript behaviour: `Math.round(-1.5) === -1`.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[test]
fn color_round_rounds_num_arrays_js_faithfully() {
    // Pure-red HSL: clean values, no round-off dust.
    let hsl = Color::Hsl([0.0, 100.0, 50.0]);
    let rounded = hsl.round();
    assert_eq!(rounded, Color::Hsl([0.0, 100.0, 50.0]));

    // Half values — JS Math.round(-1.5) = -1.
    let lab = Color::Lab([-1.5, 0.7, 2.3]);
    let rounded = lab.round();
    assert_eq!(rounded, Color::Lab([js_round(-1.5), js_round(0.7), js_round(2.3)]));
    // Verify the critical negative-half case: must be -1.0, not -2.0.
    assert!((rounded).eq_array(&[-1.0, 1.0, 2.0]));
}

#[test]
fn color_round_passes_string_variants_through() {
    let hex = Color::Hex("8CC864".to_string());
    assert_eq!(hex.round(), Color::Hex("8CC864".to_string()));

    let kw = Color::Keyword("teal".to_string());
    assert_eq!(kw.round(), Color::Keyword("teal".to_string()));
}

#[test]
fn color_round_passes_u16_variants_through() {
    let a16 = Color::Ansi16(32);
    assert_eq!(a16.round(), Color::Ansi16(32));

    let a256 = Color::Ansi256(196);
    assert_eq!(a256.round(), Color::Ansi256(196));
}

// Helper extension for comparing Color array variant contents.
impl Color {
    fn eq_array(&self, expected: &[f64]) -> bool {
        match self {
            Color::Rgb(v) => v == expected,
            Color::Hsl(v) => v == expected,
            Color::Hsv(v) => v == expected,
            Color::Hwb(v) => v == expected,
            Color::Cmyk(v) => v == expected,
            Color::Xyz(v) => v == expected,
            Color::Lab(v) => v == expected,
            Color::Lch(v) => v == expected,
            Color::Oklab(v) => v == expected,
            Color::Oklch(v) => v == expected,
            Color::Hcg(v) => v == expected,
            Color::Apple(v) => v == expected,
            Color::Gray(v) => v == expected,
            _ => false,
        }
    }
}
