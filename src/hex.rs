//! Hex colour-string decoder — ported from `convert.hex.rgb` in
//! color-convert@3.1.3 `conversions.js` (lines 757–777).
//!
//! ## Input
//!
//! Accepts a hex string (e.g. `"000000"`, `"8CC864"`, `"ABC"`). A leading
//! `#` is tolerated but not required. The scanner finds the *first* run of
//! 6 (or, failing that, 3) hex digits, case-insensitive — mirroring the JS
//! regex `/^(?:[a-f\d]{6}|[a-f\d]{3})/i` ("longest-leftmost" semantics).
//!
//! ## Output
//!
//! Returns **raw (unrounded) floats** `[r (0-255), g (0-255), b (0-255)]`.
//! Per-channel rounding is the caller's responsibility. If no hex run is
//! found the function returns `[0.0, 0.0, 0.0]`, matching `color-convert`'s
//! fallback.
//!
//! ## Tolerance
//!
//! Outputs are integer-valued after rounding — comparison is exact at
//! tolerance 0.0.

/// Converts a hex colour string to raw RGB floats.
///
/// Behaviour mirrors `convert.hex.rgb` (color-convert@3.1.3):
/// 1. Scan for the first run of 6 hex digits (e.g. `"8CC864"`).
/// 2. If none found, scan for the first run of 3 hex digits (e.g. `"ABC"`),
///    doubling each character to `"AABBCC"`.
/// 3. Parse the run as base‑16 and extract the red, green, and blue bytes.
/// 4. If no hex run is found, return `[0.0, 0.0, 0.0]`.
pub fn rgb(hex: &str) -> [f64; 3] {
    let bytes = hex.as_bytes();
    let len = bytes.len();

    // Mirror JS regex alternation `/^(?:[a-f\d]{6}|[a-f\d]{3})/i`:
    // try the longest match (6 hex digits) first at each position,
    // leftmost-wins.

    // --- 6-hex run ---
    let mut i = 0;
    while i + 5 < len {
        if (0..6).all(|j| bytes[i + j].is_ascii_hexdigit()) {
            // SAFETY: we just verified every byte in [i..i+6) is ASCII hex.
            let s = unsafe { std::str::from_utf8_unchecked(&bytes[i..i + 6]) };
            return match u32::from_str_radix(s, 16) {
                Ok(val) => [
                    ((val >> 16) & 0xFF) as f64,
                    ((val >> 8) & 0xFF) as f64,
                    (val & 0xFF) as f64,
                ],
                Err(_) => [0.0, 0.0, 0.0],
            };
        }
        i += 1;
    }

    // --- 3-hex run ---
    i = 0;
    while i + 2 < len {
        if (0..3).all(|j| bytes[i + j].is_ascii_hexdigit()) {
            let r = hex_digit_val(bytes[i]) * 17;
            let g = hex_digit_val(bytes[i + 1]) * 17;
            let b = hex_digit_val(bytes[i + 2]) * 17;
            return [f64::from(r), f64::from(g), f64::from(b)];
        }
        i += 1;
    }

    // JS fallback: no match → [0, 0, 0]
    [0.0, 0.0, 0.0]
}

/// Returns the integer value of a single ASCII hex digit (case-insensitive).
/// Non-hex inputs return 0 as a safe fallback; callers must pre-validate via
/// [`u8::is_ascii_hexdigit`].
fn hex_digit_val(byte: u8) -> u32 {
    match byte {
        b'0'..=b'9' => u32::from(byte - b'0'),
        b'a'..=b'f' => u32::from(byte - b'a' + 10),
        b'A'..=b'F' => u32::from(byte - b'A' + 10),
        _ => 0,
    }
}
