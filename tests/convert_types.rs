//! Tests for the `convert` API's `Color` and `Model` types (issue #17).
//!
//! `Color::round` must mirror JavaScript `Math.round` semantics:
//! half toward positive infinity (not Rust's default half-away-from-zero).
//! For non-negative floats the two coincide, but negative values diverge:
//! JS `Math.round(-1.5) === -1`, Rust `(-1.5_f64).round() === -2.0`.
//!
//! Tolerance: 0.0 (exact comparison after manual JS-round emulation).

use color_convert_rs::Color;

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
    assert_eq!(
        rounded,
        Color::Lab([js_round(-1.5), js_round(0.7), js_round(2.3)])
    );
    // Verify the critical negative-half case: must be -1.0, not -2.0.
    assert_eq!(
        array_channels(&rounded),
        &[js_round(-1.5), js_round(0.7), js_round(2.3)][..]
    );
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

// Helper: extract the array channels from a Color variant, or panic.
fn array_channels(c: &Color) -> &[f64] {
    match c {
        Color::Rgb(v) => v,
        Color::Hsl(v) => v,
        Color::Hsv(v) => v,
        Color::Hwb(v) => v,
        Color::Cmyk(v) => v,
        Color::Xyz(v) => v,
        Color::Lab(v) => v,
        Color::Lch(v) => v,
        Color::Oklab(v) => v,
        Color::Oklch(v) => v,
        Color::Hcg(v) => v,
        Color::Apple(v) => v,
        Color::Gray(v) => v,
        _ => panic!("not an array variant: {c:?}"),
    }
}
