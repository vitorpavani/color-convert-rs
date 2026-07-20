//! A behavior-faithful Rust port of the npm `color-convert` library.
//!
//! Conversion modules are born from failing tests in the Red/Green/Blue TDD
//! loop — none exist until a test demands them.

pub mod ansi16;
pub mod ansi256;
pub mod apple;
pub mod cmyk;
mod color_name;
pub mod convert;
mod error;
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
pub mod xyz;

pub use convert::{Color, Model, convert, convert_rounded};
pub use error::Error;
pub use probe::{Backend, gpu_present, probe};
