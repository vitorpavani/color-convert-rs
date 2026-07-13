//! Vector tests for the public `convert` API's single-hop (native) routes.
//!
//! Each test drives `color_convert_rs::convert_rounded` against committed
//! JS-generated vectors — AGENTS.md Rule 8.  `convert_rounded` mirrors the
//! observable output of the JS public wrapper: raw conversion + per-channel
//! `Math.round`.  Comparisons are exact at tolerance 0.0.
//!
//! Also exercises the `convert` (unrounded) API on a representative route.

mod harness;

use color_convert_rs::{Color, Model, convert, convert_rounded};
use harness::{VecValue, assert_cases, load_route};

/// Build a `Color` from a `VecValue` according to the model.
fn color_from_vecvalue(model: Model, value: &VecValue) -> Color {
    match value {
        VecValue::Nums(nums) => {
            let arr3 = |n: usize| {
                [nums[n], nums[1.min(n)], nums[2.min(n)]] // pad short arrays
            };
            match model {
                Model::Rgb => Color::Rgb([nums[0], nums[1], nums[2]]),
                Model::Hsl => Color::Hsl(arr3(0)),
                Model::Hsv => Color::Hsv(arr3(0)),
                Model::Hwb => Color::Hwb(arr3(0)),
                Model::Cmyk => Color::Cmyk([nums[0], nums[1], nums[2], nums[3]]),
                Model::Xyz => Color::Xyz(arr3(0)),
                Model::Lab => Color::Lab(arr3(0)),
                Model::Lch => Color::Lch(arr3(0)),
                Model::Oklab => Color::Oklab(arr3(0)),
                Model::Oklch => Color::Oklch(arr3(0)),
                Model::Hcg => Color::Hcg(arr3(0)),
                Model::Apple => Color::Apple(arr3(0)),
                Model::Gray => Color::Gray([nums[0]]),
                Model::Hex => Color::Hex(nums.iter().map(|n| (*n as u8) as char).collect()),
                Model::Keyword => Color::Keyword(String::new()),
                Model::Ansi16 => Color::Ansi16(nums[0] as u16),
                Model::Ansi256 => Color::Ansi256(nums[0] as u16),
            }
        }
        VecValue::Text(s) => match model {
            Model::Hex => Color::Hex(s.clone()),
            Model::Keyword => Color::Keyword(s.clone()),
            _ => panic!("text input for non-string model {model:?}"),
        },
        VecValue::Num(n) => match model {
            Model::Ansi16 => Color::Ansi16(*n as u16),
            Model::Ansi256 => Color::Ansi256(*n as u16),
            _ => panic!("numeric input for non-u16 model {model:?}"),
        },
    }
}

/// Extract a `VecValue` from a `Color` for comparison with vectors.
fn vecvalue_from_color(c: &Color) -> VecValue {
    match c {
        Color::Rgb(v) => VecValue::Nums(v.to_vec()),
        Color::Hsl(v) => VecValue::Nums(v.to_vec()),
        Color::Hsv(v) => VecValue::Nums(v.to_vec()),
        Color::Hwb(v) => VecValue::Nums(v.to_vec()),
        Color::Cmyk(v) => VecValue::Nums(v.to_vec()),
        Color::Xyz(v) => VecValue::Nums(v.to_vec()),
        Color::Lab(v) => VecValue::Nums(v.to_vec()),
        Color::Lch(v) => VecValue::Nums(v.to_vec()),
        Color::Oklab(v) => VecValue::Nums(v.to_vec()),
        Color::Oklch(v) => VecValue::Nums(v.to_vec()),
        Color::Hcg(v) => VecValue::Nums(v.to_vec()),
        Color::Apple(v) => VecValue::Nums(v.to_vec()),
        Color::Gray(v) => VecValue::Nums(v.to_vec()),
        Color::Hex(s) => VecValue::Text(s.clone()),
        Color::Keyword(s) => VecValue::Text(s.clone()),
        Color::Ansi16(n) => VecValue::Num(f64::from(*n)),
        Color::Ansi256(n) => VecValue::Num(f64::from(*n)),
    }
}

/// Parse a model string from vector metadata (e.g. "rgb" → Model::Rgb).
fn model_from_str(s: &str) -> Model {
    match s {
        "rgb" => Model::Rgb,
        "hsl" => Model::Hsl,
        "hsv" => Model::Hsv,
        "hwb" => Model::Hwb,
        "cmyk" => Model::Cmyk,
        "xyz" => Model::Xyz,
        "lab" => Model::Lab,
        "lch" => Model::Lch,
        "oklab" => Model::Oklab,
        "oklch" => Model::Oklch,
        "hcg" => Model::Hcg,
        "apple" => Model::Apple,
        "gray" => Model::Gray,
        "hex" => Model::Hex,
        "keyword" => Model::Keyword,
        "ansi16" => Model::Ansi16,
        "ansi256" => Model::Ansi256,
        _ => panic!("unknown model: {s}"),
    }
}

// ── single-hop native route: rgb → hsl ─────────────────────────────────────

#[test]
fn convert_rounded_rgb_to_hsl_matches_native_vectors() {
    let vectors = load_route("rgb", "hsl");
    let from = model_from_str(&vectors.from);
    let to = model_from_str(&vectors.to);

    assert_cases("convert_rounded(rgb→hsl)", &vectors.cases, 0.0, |input| {
        let src = color_from_vecvalue(from, input);
        let result = convert_rounded(from, to, src).expect("convert_rounded rgb→hsl should succeed");
        vecvalue_from_color(&result)
    });
}

#[test]
fn convert_raw_rgb_to_hsl_returns_unrounded_values() {
    // Pure red: raw hsl should be [0.0, 100.0, 50.0] (unrounded, coincidentally integer).
    let result =
        convert(Model::Rgb, Model::Hsl, Color::Rgb([255.0, 0.0, 0.0])).expect("convert should succeed");
    assert_eq!(result, Color::Hsl([0.0, 100.0, 50.0]));
}
