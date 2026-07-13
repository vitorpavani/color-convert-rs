//! Keyword (CSS named color) source routes.
//!
//! Converts from CSS color names to other color spaces. The lookup table is
//! the vendored `color-name` table at `crate::color_name::CSS_COLORS` (148
//! named colors in `color-name@2.1.0` insertion order).
//!
//! Reference: `convert.keyword.*` in color-convert's `conversions.js` (lines 310–316).

/// Returns the `[r, g, b]` triple (as `f64` in 0..=255) for a CSS color name.
///
/// Mirror of `convert.keyword.rgb` — returns `[...cssKeywords[keyword]]`
/// (i.e. the RGB value from the table). Names must be lowercase and
/// case-sensitive (matching `CSS_COLORS`). An unknown name returns `[0, 0, 0]`
/// gracefully — the JS function returns `undefined` in that case, but Rust callers
/// in practice only use the 148 known names.
///
/// Tolerance: 0.0 (exact integer-to-f64 cast).
pub fn rgb(name: &str) -> [f64; 3] {
    for (n, [r, g, b]) in crate::color_name::CSS_COLORS {
        if n == name {
            return [r as f64, g as f64, b as f64];
        }
    }
    [0.0, 0.0, 0.0]
}
