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
pub mod rgb;
pub mod xyz;

pub use convert::{Color, Model};
pub use error::Error;
