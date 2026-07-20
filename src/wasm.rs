//! wasm-bindgen exports for the npm drop-in replacement of `color-convert`.
//!
//! Exposes two entry points:
//! - [`convert_route`] — wraps [`crate::convert_rounded`] (matches the JS public wrapper).
//! - [`convert_route_raw`] — wraps [`crate::convert`] (matches the JS `.raw` variant).
//!
//! Both accept `(from: &str, to: &str, input: JsValue)` and return `JsValue`,
//! so the same function handles every route — numeric arrays for rgb/hsl/etc.,
//! strings for hex/keyword, and numbers for ansi16/ansi256.
//!
//! See `js/index.js` for the nested `convert.rgb.hsl(r,g,b)` API built on top.

use js_sys::Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use crate::{Color, Model, convert, convert_rounded};

/// Convert a single colour from one model to another, applying per-channel
/// `Math.round` to numeric results (matches `color-convert`'s public wrapper).
///
/// Returns a JS array for numeric models, a string for `hex`/`keyword`, or a
/// number for `ansi16`/`ansi256`.
///
/// # Errors
///
/// Returns a JS `Error` if `from`/`to` are unknown model names, the input
/// does not match the `from` model, or no conversion path exists.
#[wasm_bindgen]
pub fn convert_route(from: &str, to: &str, input: &JsValue) -> Result<JsValue, String> {
    let from_model = parse_model(from)?;
    let to_model = parse_model(to)?;
    let color = jsvalue_to_color(from_model, input)?;
    let result = convert_rounded(from_model, to_model, color).map_err(|e| e.to_string())?;
    Ok(color_to_jsvalue(result))
}

/// Like [`convert_route`] but without per-channel rounding (matches the
/// `.raw` variant on every `color-convert` route).
#[wasm_bindgen]
pub fn convert_route_raw(from: &str, to: &str, input: &JsValue) -> Result<JsValue, String> {
    let from_model = parse_model(from)?;
    let to_model = parse_model(to)?;
    let color = jsvalue_to_color(from_model, input)?;
    let result = convert(from_model, to_model, color).map_err(|e| e.to_string())?;
    Ok(color_to_jsvalue(result))
}

// ── Batch SIMD exports ──────────────────────────────────────────────
//
// Single-color `convert_route` calls go through JS↔wasm boundary overhead
// (string parsing, JsValue marshalling) that makes them ~10× slower than
// the original JS library per-call. The batch path eliminates this: pass a
// flat typed array, get a flat typed array back, amortise the boundary cost
// across thousands of colours with f32x8 SIMD inside.
//
// Input:  flat `[r,g,b, r,g,b, ...]` as Uint8Array (rgb routes) or
//         flat `[h,s,l, h,s,l, ...]` as Float32Array (inverse routes)
// Output: flat Float32Array of the target model's channels

macro_rules! batch_rgb_export {
    ($fn_name:ident, $simd_fn:path, $out_chans:expr) => {
        #[wasm_bindgen]
        pub fn $fn_name(input: &[u8]) -> Vec<f32> {
            let pixels: Vec<[u8; 3]> = input.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            let result = $simd_fn(&pixels);
            let mut flat = Vec::with_capacity(result.len() * $out_chans);
            for px in &result {
                for &ch in px.iter() {
                    flat.push(ch);
                }
            }
            flat
        }
    };
}

macro_rules! batch_f32_export {
    ($fn_name:ident, $simd_fn:path, $out_chans:expr) => {
        #[wasm_bindgen]
        pub fn $fn_name(input: &[f32]) -> Vec<f32> {
            let input_arr: Vec<[f32; 3]> =
                input.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            let result = $simd_fn(&input_arr);
            let mut flat = Vec::with_capacity(result.len() * $out_chans);
            for px in &result {
                for &ch in px.iter() {
                    flat.push(ch);
                }
            }
            flat
        }
    };
}

batch_rgb_export!(rgb_to_hsl_batch, crate::simd_hsl::rgb_to_hsl_batch, 3);
batch_rgb_export!(rgb_to_hsv_batch, crate::simd_hsv::rgb_to_hsv_batch, 3);
batch_rgb_export!(rgb_to_cmyk_batch, crate::simd_cmyk::rgb_to_cmyk_batch, 4);
batch_rgb_export!(rgb_to_lab_batch, crate::simd::rgb_to_lab_batch, 3);
batch_rgb_export!(rgb_to_xyz_batch, crate::simd::rgb_to_xyz_batch, 3);
batch_rgb_export!(rgb_to_oklab_batch, crate::simd_oklab::rgb_to_oklab_batch, 3);

batch_f32_export!(hsl_to_rgb_batch, crate::simd_hsl::hsl_to_rgb_batch, 3);
batch_f32_export!(hsv_to_rgb_batch, crate::simd_hsv_rgb::hsv_to_rgb_batch, 3);

fn parse_model(s: &str) -> Result<Model, String> {
    match s {
        "rgb" => Ok(Model::Rgb),
        "hsl" => Ok(Model::Hsl),
        "hsv" => Ok(Model::Hsv),
        "hwb" => Ok(Model::Hwb),
        "cmyk" => Ok(Model::Cmyk),
        "xyz" => Ok(Model::Xyz),
        "lab" => Ok(Model::Lab),
        "lch" => Ok(Model::Lch),
        "oklab" => Ok(Model::Oklab),
        "oklch" => Ok(Model::Oklch),
        "hex" => Ok(Model::Hex),
        "keyword" => Ok(Model::Keyword),
        "ansi16" => Ok(Model::Ansi16),
        "ansi256" => Ok(Model::Ansi256),
        "hcg" => Ok(Model::Hcg),
        "apple" => Ok(Model::Apple),
        "gray" => Ok(Model::Gray),
        other => Err(format!("unknown model name: {other}")),
    }
}

fn jsvalue_to_color(model: Model, input: &JsValue) -> Result<Color, String> {
    match model {
        Model::Rgb => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("rgb expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Rgb([v[0], v[1], v[2]]))
        }
        Model::Hsl => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("hsl expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Hsl([v[0], v[1], v[2]]))
        }
        Model::Hsv => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("hsv expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Hsv([v[0], v[1], v[2]]))
        }
        Model::Hwb => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("hwb expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Hwb([v[0], v[1], v[2]]))
        }
        Model::Cmyk => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 4 {
                return Err(format!("cmyk expects 4 channels, got {}", v.len()));
            }
            Ok(Color::Cmyk([v[0], v[1], v[2], v[3]]))
        }
        Model::Xyz => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("xyz expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Xyz([v[0], v[1], v[2]]))
        }
        Model::Lab => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("lab expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Lab([v[0], v[1], v[2]]))
        }
        Model::Lch => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("lch expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Lch([v[0], v[1], v[2]]))
        }
        Model::Oklab => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("oklab expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Oklab([v[0], v[1], v[2]]))
        }
        Model::Oklch => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("oklch expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Oklch([v[0], v[1], v[2]]))
        }
        Model::Hcg => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("hcg expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Hcg([v[0], v[1], v[2]]))
        }
        Model::Apple => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 3 {
                return Err(format!("apple expects 3 channels, got {}", v.len()));
            }
            Ok(Color::Apple([v[0], v[1], v[2]]))
        }
        Model::Gray => {
            let v: Vec<f64> = js_array_to_f64s(input)?;
            if v.len() != 1 {
                return Err(format!("gray expects 1 channel, got {}", v.len()));
            }
            Ok(Color::Gray([v[0]]))
        }
        Model::Hex => {
            let s: String = input
                .as_string()
                .ok_or_else(|| "hex input must be a string".to_string())?;
            Ok(Color::Hex(s))
        }
        Model::Keyword => {
            let s: String = input
                .as_string()
                .ok_or_else(|| "keyword input must be a string".to_string())?;
            Ok(Color::Keyword(s))
        }
        Model::Ansi16 => {
            let n = js_number_to_u16(input, "ansi16")?;
            Ok(Color::Ansi16(n))
        }
        Model::Ansi256 => {
            let n = js_number_to_u16(input, "ansi256")?;
            Ok(Color::Ansi256(n))
        }
    }
}

fn js_array_to_f64s(input: &JsValue) -> Result<Vec<f64>, String> {
    if input.is_array() {
        let arr: Array = Array::from(input);
        let mut out = Vec::with_capacity(arr.length() as usize);
        for i in 0..arr.length() {
            let v = arr.get(i);
            if let Some(n) = v.as_f64() {
                out.push(n);
            } else if let Some(s) = v.as_string() {
                let parsed: f64 = s
                    .parse()
                    .map_err(|_| format!("non-numeric string in array: {s}"))?;
                out.push(parsed);
            } else {
                return Err(format!("non-numeric value in array at index {i}"));
            }
        }
        Ok(out)
    } else if let Some(n) = input.as_f64() {
        Ok(vec![n])
    } else if let Some(s) = input.as_string() {
        let parsed: f64 = s
            .parse()
            .map_err(|_| format!("cannot parse {s} as number"))?;
        Ok(vec![parsed])
    } else {
        Err("input must be an array, number, or numeric string".to_string())
    }
}

fn js_number_to_u16(input: &JsValue, model: &str) -> Result<u16, String> {
    if let Some(n) = input.as_f64() {
        Ok(n as u16)
    } else if let Some(s) = input.as_string() {
        s.parse::<u16>()
            .map_err(|_| format!("{model} input cannot parse {s} as u16"))
    } else {
        Err(format!("{model} input must be a number"))
    }
}

fn color_to_jsvalue(color: Color) -> JsValue {
    match color {
        Color::Rgb(v) => f64s_to_js_array(&v),
        Color::Hsl(v) => f64s_to_js_array(&v),
        Color::Hsv(v) => f64s_to_js_array(&v),
        Color::Hwb(v) => f64s_to_js_array(&v),
        Color::Cmyk(v) => f64s_to_js_array(&v),
        Color::Xyz(v) => f64s_to_js_array(&v),
        Color::Lab(v) => f64s_to_js_array(&v),
        Color::Lch(v) => f64s_to_js_array(&v),
        Color::Oklab(v) => f64s_to_js_array(&v),
        Color::Oklch(v) => f64s_to_js_array(&v),
        Color::Hcg(v) => f64s_to_js_array(&v),
        Color::Apple(v) => f64s_to_js_array(&v),
        Color::Gray(v) => f64s_to_js_array(&v),
        Color::Hex(s) => JsValue::from_str(&s),
        Color::Keyword(s) => JsValue::from_str(&s),
        Color::Ansi16(n) => JsValue::from(n),
        Color::Ansi256(n) => JsValue::from(n),
    }
}

fn f64s_to_js_array(v: &[f64]) -> JsValue {
    let arr = Array::new();
    for &x in v {
        arr.push(&JsValue::from(x));
    }
    arr.into()
}
