//! A behavior-faithful Rust port of the npm `color-convert` library.
//!
//! Conversion modules are born from failing tests in the Red/Green/Blue TDD
//! loop — none exist until a test demands them.

pub mod cmyk;
mod color_name;
mod error;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod lab;
pub mod lch;
pub mod oklab;
pub mod oklch;
pub mod rgb;
pub mod xyz;

pub use error::Error;
