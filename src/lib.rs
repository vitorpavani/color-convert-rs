//! A behavior-faithful Rust port of the npm `color-convert` library.
//!
//! Conversion modules are born from failing tests in the Red/Green/Blue TDD
//! loop — none exist until a test demands them.

pub mod apple;
pub mod cmyk;
mod color_name;
mod error;
pub mod gray;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod keyword;
pub mod rgb;

pub use error::Error;
