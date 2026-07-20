//! Native Node.js addon exports via napi-rs.
//!
//! Unlike the wasm path (`src/wasm.rs`), napi calls cross the V8↔Rust boundary
//! directly (~10-50ns) instead of the JS→wasm marshal boundary (~500ns). This
//! eliminates the 7-25× single-color overhead seen with wasm.
//!
//! Built with `@napi-rs/cli` or `cargo build --features napi --release`.

use napi::bindgen_prelude::{Float32Array, Float64Array, Uint8Array};
use napi_derive::napi;

use crate::{Color, Model, convert, convert_rounded};

fn parse_model_napi(s: &str) -> napi::Result<Model> {
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
        other => Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("unknown model: {other}"),
        )),
    }
}

fn vec_to_color(model: Model, v: &[f64]) -> napi::Result<Color> {
    match model {
        Model::Rgb => {
            if v.len() != 3 {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    format!("rgb expects 3 channels, got {}", v.len()),
                ));
            }
            Ok(Color::Rgb([v[0], v[1], v[2]]))
        }
        Model::Hsl => Ok(Color::Hsl([v[0], v[1], v[2]])),
        Model::Hsv => Ok(Color::Hsv([v[0], v[1], v[2]])),
        Model::Hwb => Ok(Color::Hwb([v[0], v[1], v[2]])),
        Model::Cmyk => Ok(Color::Cmyk([v[0], v[1], v[2], v[3]])),
        Model::Xyz => Ok(Color::Xyz([v[0], v[1], v[2]])),
        Model::Lab => Ok(Color::Lab([v[0], v[1], v[2]])),
        Model::Lch => Ok(Color::Lch([v[0], v[1], v[2]])),
        Model::Oklab => Ok(Color::Oklab([v[0], v[1], v[2]])),
        Model::Oklch => Ok(Color::Oklch([v[0], v[1], v[2]])),
        Model::Hcg => Ok(Color::Hcg([v[0], v[1], v[2]])),
        Model::Apple => Ok(Color::Apple([v[0], v[1], v[2]])),
        Model::Gray => Ok(Color::Gray([v[0]])),
        Model::Ansi16 => {
            if v.is_empty() {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "ansi16 expects at least 1 value",
                ));
            }
            Ok(Color::Ansi16(v[0] as u16))
        }
        Model::Ansi256 => {
            if v.is_empty() {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "ansi16 expects at least 1 value",
                ));
            }
            Ok(Color::Ansi256(v[0] as u16))
        }
        Model::Hex | Model::Keyword => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "use convert_from_string for hex/keyword source models",
        )),
    }
}

fn color_to_vec(color: Color) -> Vec<f64> {
    match color {
        Color::Rgb(v) => v.to_vec(),
        Color::Hsl(v) => v.to_vec(),
        Color::Hsv(v) => v.to_vec(),
        Color::Hwb(v) => v.to_vec(),
        Color::Cmyk(v) => v.to_vec(),
        Color::Xyz(v) => v.to_vec(),
        Color::Lab(v) => v.to_vec(),
        Color::Lch(v) => v.to_vec(),
        Color::Oklab(v) => v.to_vec(),
        Color::Oklch(v) => v.to_vec(),
        Color::Hcg(v) => v.to_vec(),
        Color::Apple(v) => v.to_vec(),
        Color::Gray(v) => v.to_vec(),
        Color::Hex(_) | Color::Keyword(_) | Color::Ansi16(_) | Color::Ansi256(_) => Vec::new(),
    }
}

fn napi_err(e: crate::Error) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, e.to_string())
}

#[napi]
pub fn convert_route(from: String, to: String, input: Vec<f64>) -> napi::Result<Vec<f64>> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = vec_to_color(from_model, &input)?;
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    Ok(color_to_vec(result))
}

#[napi]
pub fn convert_route_raw(from: String, to: String, input: Vec<f64>) -> napi::Result<Vec<f64>> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = vec_to_color(from_model, &input)?;
    let result = convert(from_model, to_model, color).map_err(napi_err)?;
    Ok(color_to_vec(result))
}

#[napi]
pub fn convert_to_string(from: String, to: String, input: Vec<f64>) -> napi::Result<String> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = vec_to_color(from_model, &input)?;
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    match result {
        Color::Hex(s) | Color::Keyword(s) => Ok(s),
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "target model does not produce a string",
        )),
    }
}

#[napi]
pub fn convert_to_number(from: String, to: String, input: Vec<f64>) -> napi::Result<u32> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = vec_to_color(from_model, &input)?;
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    match result {
        Color::Ansi16(n) => Ok(n as u32),
        Color::Ansi256(n) => Ok(n as u32),
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "target model does not produce a number",
        )),
    }
}

#[napi]
pub fn convert_from_string(from: String, to: String, input: String) -> napi::Result<Vec<f64>> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = match from_model {
        Model::Hex => Color::Hex(input),
        Model::Keyword => Color::Keyword(input),
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "use convert_route for numeric models",
            ));
        }
    };
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    Ok(color_to_vec(result))
}

#[napi]
pub fn convert_from_string_to_number(from: String, to: String, input: String) -> napi::Result<u32> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = match from_model {
        Model::Hex => Color::Hex(input),
        Model::Keyword => Color::Keyword(input),
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "source must be hex or keyword",
            ));
        }
    };
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    match result {
        Color::Ansi16(n) => Ok(n as u32),
        Color::Ansi256(n) => Ok(n as u32),
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "target must be ansi16 or ansi256",
        )),
    }
}

#[napi]
pub fn convert_from_string_to_string(
    from: String,
    to: String,
    input: String,
) -> napi::Result<String> {
    let from_model = parse_model_napi(&from)?;
    let to_model = parse_model_napi(&to)?;
    let color = match from_model {
        Model::Hex => Color::Hex(input),
        Model::Keyword => Color::Keyword(input),
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "source must be hex or keyword",
            ));
        }
    };
    let result = convert_rounded(from_model, to_model, color).map_err(napi_err)?;
    match result {
        Color::Hex(s) | Color::Keyword(s) => Ok(s),
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "target must be hex or keyword",
        )),
    }
}

macro_rules! napi_batch_rgb {
    ($fn_name:ident, $simd:path, $out_chans:expr) => {
        #[napi]
        pub fn $fn_name(input: Uint8Array) -> Float32Array {
            let raw = input.as_ref();
            let pixels: Vec<[u8; 3]> = raw.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            let result = $simd(&pixels);
            let mut flat = Vec::with_capacity(result.len() * $out_chans);
            for px in &result {
                for &ch in px.iter() {
                    flat.push(ch);
                }
            }
            Float32Array::from(flat)
        }
    };
}

napi_batch_rgb!(rgb_to_hsl_batch, crate::simd_hsl::rgb_to_hsl_batch, 3);
napi_batch_rgb!(rgb_to_hsv_batch, crate::simd_hsv::rgb_to_hsv_batch, 3);
napi_batch_rgb!(rgb_to_cmyk_batch, crate::simd_cmyk::rgb_to_cmyk_batch, 4);
napi_batch_rgb!(rgb_to_lab_batch, crate::simd::rgb_to_lab_batch, 3);
napi_batch_rgb!(rgb_to_xyz_batch, crate::simd::rgb_to_xyz_batch, 3);
napi_batch_rgb!(rgb_to_oklab_batch, crate::simd_oklab::rgb_to_oklab_batch, 3);

// ── Typed fast paths (no string parsing, no Vec allocation for input) ──

#[napi]
pub fn rgb_hsl(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Hsl, color).unwrap();
    color_to_vec(result)
}

#[napi]
pub fn rgb_hsv(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Hsv, color).unwrap();
    color_to_vec(result)
}

#[napi]
pub fn rgb_lab(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Lab, color).unwrap();
    color_to_vec(result)
}

#[napi]
pub fn rgb_xyz(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Xyz, color).unwrap();
    color_to_vec(result)
}

#[napi]
pub fn rgb_oklab(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Oklab, color).unwrap();
    color_to_vec(result)
}

#[napi]
pub fn rgb_cmyk(r: f64, g: f64, b: f64) -> Vec<f64> {
    let color = Color::Rgb([r, g, b]);
    let result = convert_rounded(Model::Rgb, Model::Cmyk, color).unwrap();
    color_to_vec(result)
}

macro_rules! napi_into_fn {
    ($fn_name:ident, $to_model:path) => {
        #[napi]
        pub fn $fn_name(r: f64, g: f64, b: f64, mut output: Float64Array) {
            let result = convert_rounded(Model::Rgb, $to_model, Color::Rgb([r, g, b])).unwrap();
            let out = output.as_mut();
            match result {
                Color::Rgb(v) => out[..3].copy_from_slice(&v[..]),
                Color::Hsl(v) => out[..3].copy_from_slice(&v[..]),
                Color::Hsv(v) => out[..3].copy_from_slice(&v[..]),
                Color::Hwb(v) => out[..3].copy_from_slice(&v[..]),
                Color::Cmyk(v) => out[..4].copy_from_slice(&v[..]),
                Color::Xyz(v) => out[..3].copy_from_slice(&v[..]),
                Color::Lab(v) => out[..3].copy_from_slice(&v[..]),
                Color::Lch(v) => out[..3].copy_from_slice(&v[..]),
                Color::Oklab(v) => out[..3].copy_from_slice(&v[..]),
                Color::Oklch(v) => out[..3].copy_from_slice(&v[..]),
                Color::Hcg(v) => out[..3].copy_from_slice(&v[..]),
                Color::Apple(v) => out[..3].copy_from_slice(&v[..]),
                Color::Gray(v) => out[..1].copy_from_slice(&v[..]),
                _ => {}
            }
        }
    };
}

napi_into_fn!(rgb_hsl_into, Model::Hsl);
napi_into_fn!(rgb_hsv_into, Model::Hsv);
napi_into_fn!(rgb_lab_into, Model::Lab);
napi_into_fn!(rgb_xyz_into, Model::Xyz);
napi_into_fn!(rgb_oklab_into, Model::Oklab);
napi_into_fn!(rgb_cmyk_into, Model::Cmyk);
