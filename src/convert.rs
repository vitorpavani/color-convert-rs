//! Public `convert` API — colour-space routing graph with BFS multi-hop
//! pathfinding, mirroring `route.js` in color-convert@3.1.3.
//!
//! ## `Color`
//!
//! The 17 colour models, each holding raw (unrounded) `f64` channel values
//! (or `String`/`u16` for terminal/label encodings). `Color::round` applies
//! JavaScript `Math.round` semantics (half toward +∞) to every numeric
//! channel, matching the public wrapper behaviour.
//!
//! ## `Model`
//!
//! A lightweight `Copy` discriminant for the 17 colour models, used as the
//! key type in the BFS routing graph.
//!
//! ## Graph & `convert`
//!
//! `Graph` builds an adjacency map over all 50 native routes. `convert(from,
//! to, input)` validates that the `Color` variant matches `from`, finds a
//! shortest path via BFS, and chains the native conversion functions.
//! `convert_rounded` additionally applies per-channel rounding to produce
//! the observable JS public-wrapper output.

/// A concrete colour value in one of the 17 supported colour models.
///
/// All numeric variants store **raw `f64` channels** in their natural range
/// (e.g. `Rgb` is 0–255, `Hsl` hues are 0–360).  The public `convert` API
/// returns raw values; callers use `convert_rounded` or `Color::round` to
/// reproduce the per-channel `Math.round` behaviour of the JS public wrapper.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Rgb([f64; 3]),
    Hsl([f64; 3]),
    Hsv([f64; 3]),
    Hwb([f64; 3]),
    Cmyk([f64; 4]),
    Xyz([f64; 3]),
    Lab([f64; 3]),
    Lch([f64; 3]),
    Oklab([f64; 3]),
    Oklch([f64; 3]),
    Hcg([f64; 3]),
    Apple([f64; 3]),
    Gray([f64; 1]),
    Hex(String),
    Keyword(String),
    Ansi16(u16),
    Ansi256(u16),
}

impl Color {
    /// Apply JavaScript `Math.round` semantics to every numeric channel in
    /// this colour value.
    ///
    /// `Math.round` rounds half toward positive infinity:
    /// `Math.round(0.5) === 1`, `Math.round(-1.5) === -1`.  This differs
    /// from Rust's `f64::round` which rounds half away from zero
    /// (`(-1.5_f64).round() === -2.0`).  The JS-semantic rounding is
    /// implemented as `(x + 0.5).floor()`.
    ///
    /// String and `u16` variants (Hex, Keyword, Ansi16, Ansi256) are passed
    /// through unchanged — rounding does not affect them.
    #[must_use]
    pub fn round(self) -> Self {
        match self {
            Color::Rgb(v) => Color::Rgb([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hsl(v) => Color::Hsl([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hsv(v) => Color::Hsv([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hwb(v) => Color::Hwb([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Cmyk(v) => Color::Cmyk([
                js_round(v[0]),
                js_round(v[1]),
                js_round(v[2]),
                js_round(v[3]),
            ]),
            Color::Xyz(v) => Color::Xyz([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Lab(v) => Color::Lab([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Lch(v) => Color::Lch([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Oklab(v) => Color::Oklab([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Oklch(v) => Color::Oklch([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hcg(v) => Color::Hcg([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Apple(v) => Color::Apple([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Gray(v) => Color::Gray([js_round(v[0])]),
            other => other, // Hex, Keyword, Ansi16, Ansi256 — pass through
        }
    }
}

/// JavaScript `Math.round` semantics: `(x + 0.5).floor()`.
///
/// Unlike Rust's `f64::round` (half away from zero), this rounds half
/// toward positive infinity, matching the observable behaviour of
/// `color-convert`'s public wrapper.
#[inline]
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// A lightweight `Copy` discriminant for the 17 supported colour models.
///
/// Used as the key type in the BFS routing graph and the `from`/`to`
/// parameters of the public `convert` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Rgb,
    Hsl,
    Hsv,
    Hwb,
    Cmyk,
    Xyz,
    Lab,
    Lch,
    Oklab,
    Oklch,
    Hcg,
    Apple,
    Gray,
    Hex,
    Keyword,
    Ansi16,
    Ansi256,
}
