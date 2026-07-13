//! A behavior-faithful Rust port of the npm `color-convert` library.
//!
//! Conversion modules are born from failing tests in the Red/Green/Blue TDD
//! loop — none exist until a test demands them.

mod color_name;
mod error;
pub mod hsl;
pub mod hsv;
pub mod rgb;

pub use error::Error;
