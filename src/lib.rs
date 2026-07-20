//! A behavior-faithful Rust port of the npm [`color-convert`](https://github.com/Qix-/color-convert)
//! library — CPU-SIMD accelerated with optional GPU (CubeCL) and npm drop-in
//! replacement (wasm-pack) paths.
//!
//! # Quick start
//!
//! ```
//! use color_convert_rs::{Color, Model, convert_rounded};
//!
//! let orange = Color::Rgb([255.0, 128.0, 0.0]);
//! let lab = convert_rounded(Model::Rgb, Model::Lab, orange).unwrap();
//! assert_eq!(lab, Color::Lab([67.0, 43.0, 74.0]));
//! ```
//!
//! # Features
//!
//! - **`gpu`** *(optional)*: CubeCL/wgpu GPU compute kernels with runtime
//!   capability probe. Without this feature, `probe()` always returns
//!   [`Backend::CpuSimd`].
//! - **`wasm`** *(optional)*: wasm-bindgen exports for building the npm
//!   drop-in replacement via `wasm-pack build --target nodejs --features wasm`.
//!
//! # Public API
//!
//! - [`convert`] / [`convert_rounded`]: single-color any-to-any conversion
//! - [`Model`]: the 17 supported color models
//! - [`Color`]: the color value type (array or string depending on model)
//! - [`Error`]: the library error type
//! - `simd::*_batch`: vectorized f32x8 SIMD batch functions for hot routes
//! - [`probe`] / [`gpu_present`] / [`Backend`]: runtime GPU capability probe

pub mod ansi16;
pub mod ansi256;
pub mod apple;
pub mod batch;
pub mod cmyk;
mod color_name;
pub mod convert;
mod error;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod gray;
pub mod hcg;
pub mod hex;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod keyword;
pub mod lab;
pub mod lch;
pub mod oklab;
pub mod oklch;
pub mod probe;
pub mod rgb;
pub mod simd;
pub mod simd_apple;
pub mod simd_cmyk;
pub mod simd_hcg;
pub mod simd_hsl;
pub mod simd_hsv;
pub mod simd_hsv_rgb;
pub mod simd_hwb;
pub mod simd_lab_xyz;
pub mod simd_oklab;
pub mod simd_oklab_rgb;
pub mod simd_parallel;
pub mod simd_xyz;
#[cfg(feature = "napi")]
pub mod napi;
pub mod xyz;

pub use convert::{Color, Model, convert, convert_rounded};
pub use error::Error;
pub use probe::{Backend, gpu_present, probe};
