//! Vector tests for the `rgb` source routes (issue #5).
//!
//! Each test drives one `color_convert_rs::rgb::<target>` conversion against
//! the committed JS-generated vectors (`tests/vectors/rgb_to_<target>.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8.
//!
//! API pinned for GREEN: `rgb::hsl(rgb: [u8; 3]) -> [f64; 3]` returning RAW
//! (unrounded) floats `[h (0-360), s (0-100), l (0-100)]`, mirroring
//! `convert.rgb.hsl` in color-convert's conversions.js. The signature is
//! infallible (`[f64; 3]`, not `Result`) because every `[u8; 3]` input is a
//! valid RGB triple; `Result<_, Error>` is reserved for fallible parses such
//! as hex→rgb. The vectors store the *observable* output of the JS public
//! wrapper, which applies `Math.round` per channel — so the test rounds here.
//!
//! Tolerance: 0.0. After per-channel rounding the output must match the
//! rounded JS vector EXACTLY. Rounding-mode note: Rust's `f64::round` rounds
//! half away from zero while JS `Math.round` rounds half toward +infinity;
//! these differ only for negative values, and all hsl channels are
//! non-negative, so the semantics coincide on this route.

mod harness;

use color_convert_rs::rgb;
use harness::{VecValue, assert_cases, load_route};

/// Extracts a `[u8; 3]` RGB triple from a `VecValue::Nums` input.
fn rgb_input(value: &VecValue) -> [u8; 3] {
    let VecValue::Nums(nums) = value else {
        panic!("rgb vector input must be VecValue::Nums, got {value:?}");
    };
    let channels: Vec<u8> = nums
        .iter()
        .map(|&n| {
            assert!(
                n.fract() == 0.0 && (0.0..=255.0).contains(&n),
                "rgb channel out of u8 range: {n}"
            );
            n as u8
        })
        .collect();
    channels
        .try_into()
        .unwrap_or_else(|c| panic!("rgb input must have exactly 3 channels, got {c:?}"))
}

#[test]
fn rgb_to_hsl_matches_js_vectors() {
    let vectors = load_route("rgb", "hsl");
    assert_cases("rgb_to_hsl", &vectors.cases, 0.0, |input| {
        let [h, s, l] = rgb::hsl(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![h.round(), s.round(), l.round()])
    });
}

/// API pinned for GREEN: `rgb::hsv(rgb: [u8; 3]) -> [f64; 3]` returning RAW
/// (unrounded) floats `[h (0-360), s (0-100), v (0-100)]`, mirroring
/// `convert.rgb.hsv` in color-convert's conversions.js (lines 128-186).
///
/// Tolerance: 0.0 after per-channel rounding, exactly as rgb→hsl above. All
/// hsv channels are non-negative, so Rust's half-away-from-zero `f64::round`
/// coincides with JS `Math.round` (half toward +infinity) on this route.
#[test]
fn rgb_to_hsv_matches_js_vectors() {
    let vectors = load_route("rgb", "hsv");
    assert_cases("rgb_to_hsv", &vectors.cases, 0.0, |input| {
        let [h, s, v] = rgb::hsv(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![h.round(), s.round(), v.round()])
    });
}

/// API pinned for GREEN: `rgb::hwb(rgb: [u8; 3]) -> [f64; 3]` returning RAW
/// (unrounded) floats `[h (0-360), w (0-100), b (0-100)]`, mirroring
/// `convert.rgb.hwb` in color-convert's conversions.js (lines 188-198).
///
/// The JS implementation derives h from rgb.hsl, then computes
/// w = min(r,g,b)/255 * 100, b = (1 - max(r,g,b)/255) * 100.
///
/// Tolerance: 0.0 after per-channel rounding, exactly as rgb→hsl/hsv above.
/// All hwb channels are non-negative, so Rust's half-away-from-zero
/// `f64::round` coincides with JS `Math.round` (half toward +infinity) on
/// this route.
#[test]
fn rgb_to_hwb_matches_js_vectors() {
    let vectors = load_route("rgb", "hwb");
    assert_cases("rgb_to_hwb", &vectors.cases, 0.0, |input| {
        let [h, w, b] = rgb::hwb(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![h.round(), w.round(), b.round()])
    });
}

/// API pinned for GREEN: `rgb::cmyk(rgb: [u8; 3]) -> [f64; 4]` returning RAW
/// (unrounded) floats `[c (0-100), m (0-100), y (0-100), k (0-100)]`, mirroring
/// `convert.rgb.cmyk` in color-convert's conversions.js (lines 217-228).
///
/// The JS algorithm normalizes r,g,b to /255 fractions, computes
/// k = min(1-r, 1-g, 1-b), then c = (1-r-k)/(1-k)||0 (similarly for m,y),
/// and returns [c*100, m*100, y*100, k*100]. The `||0` guards the k==1
/// (pure black) boundary where division by zero would produce NaN.
///
/// NOTE: Unlike hsl/hsv/hwb which return 3-channel `[f64; 3]`, cmyk returns
/// 4-channel `[f64; 4]`. All channels are non-negative, so Rust's half-away-
/// from-zero `f64::round` coincides with JS `Math.round` on this route.
///
/// Tolerance: 0.0 after per-channel rounding (exact match against rounded JS vectors).
#[test]
fn rgb_to_cmyk_matches_js_vectors() {
    let vectors = load_route("rgb", "cmyk");
    assert_cases("rgb_to_cmyk", &vectors.cases, 0.0, |input| {
        let [c, m, y, k] = rgb::cmyk(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![c.round(), m.round(), y.round(), k.round()])
    });
}

/// API pinned for GREEN: `rgb::xyz(rgb: [u8; 3]) -> [f64; 3]` returning RAW
/// (unrounded) floats `[x (0-100), y (0-100), z (0-100)]`, mirroring
/// `convert.rgb.xyz` in color-convert's conversions.js (lines 270-281).
///
/// The JS algorithm applies `srgbNonlinearTransformInv` to each channel/255,
/// then multiplies by the sRGB→XYZ matrix, and returns `[x*100, y*100, z*100]`.
/// The JS public wrapper applies `Math.round` per channel — so the test rounds here.
///
/// Tolerance: 0.0 after per-channel rounding (exact match against rounded JS vectors).
/// All xyz channels are non-negative tristimulus values scaled 0-100, so Rust's
/// half-away-from-zero `f64::round` coincides with JS `Math.round`
/// (half toward +infinity) on this route.
#[test]
fn rgb_to_xyz_matches_js_vectors() {
    let vectors = load_route("rgb", "xyz");
    assert_cases("rgb_to_xyz", &vectors.cases, 0.0, |input| {
        let [x, y, z] = rgb::xyz(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![x.round(), y.round(), z.round()])
    });
}

// ── shared test helpers ──────────────────────────────────────────────

/// Emulates JS `Math.round`: rounds half toward +infinity.
///
/// Rust's `f64::round` rounds half **away from zero**, which differs on
/// negative half-integers:
///
/// | value | JS `Math.round` | Rust `f64::round` |
/// |-------|-----------------|-------------------|
/// | -0.5  | 0               | -1                |
/// | -1.5  | -1              | -2                |
///
/// `(x + 0.5).floor()` reproduces `Math.round` exactly for all values
/// (including negatives). This helper is reused by routes with channels
/// that may be negative (lab, oklab, …).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

// ── rgb → lab ────────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::lab(rgb: [u8; 3]) -> [f64; 3]` returning RAW
/// (unrounded) floats `[l (0-100), a, b]`, mirroring
/// `convert.rgb.lab` in color-convert's conversions.js (lines 283-302).
///
/// The JS algorithm:
///   xyz  = rgb.xyz(rgb)
///   x   /= 95.047;  y /= 100;  z /= 108.883
///   f(v) = v > LAB_FT ? v^(1/3) : 7.787*v + 16/116     (LAB_FT = (6/29)^3)
///   l    = 116*y - 16
///   a    = 500*(x - y)
///   b    = 200*(y - z)
///
/// The `a` and `b` channels **can be negative**, which creates a
/// rounding-mode divergence: JS `Math.round` rounds half toward +infinity
/// while Rust's `f64::round` rounds half away from zero.  We therefore
/// apply `js_round` (defined above) to **every** channel in the closure
/// to faithfully reproduce the JS public wrapper's per-channel
/// `Math.round` behaviour.
///
/// Tolerance: 0.0 after per-channel `js_round` (exact match against the
/// JS-rounded vectors in `tests/vectors/rgb_to_lab.json`, 32 cases).
#[test]
fn rgb_to_lab_matches_js_vectors() {
    let vectors = load_route("rgb", "lab");
    assert_cases("rgb_to_lab", &vectors.cases, 0.0, |input| {
        let [l, a, b] = rgb::lab(rgb_input(input));
        // Apply JS-faithful rounding — a,b may be negative (see doc above).
        VecValue::Nums(vec![js_round(l), js_round(a), js_round(b)])
    });
}

// ── rgb → oklab ──────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::oklab(rgb: [u8; 3]) -> [f64; 3]` returning
/// RAW (unrounded) floats `[l (0-100), a, b]`, mirroring `convert.rgb.oklab`
/// in color-convert's conversions.js (lines 200-215).
///
/// The JS algorithm:
///   1. `srgbNonlinearTransformInv` each channel / 255 → linear sRGB
///   2. sRGB linear → LMS (linear cone response) via matrix
///   3. LMS → L'a'b' via cube-root (∛)
///   4. L'a'b' → Lab via oklab matrix
///   5. Return `[l * 100, a * 100, b * 100]`
///
/// The `a` and `b` channels **can be negative** (visible in the vectors:
/// e.g. [0,0,128] → [27, -2, -19]), which creates a rounding-mode
/// divergence between JS `Math.round` and Rust's `f64::round`.  As with
/// rgb→lab, we apply `js_round` (defined above) to **every** channel in
/// the closure to faithfully reproduce the JS public wrapper's per-channel
/// `Math.round` behaviour.
///
/// Tolerance: 0.0 after per-channel `js_round` (exact match against the
/// JS-rounded vectors in `tests/vectors/rgb_to_oklab.json`, 32 cases).
#[test]
fn rgb_to_oklab_matches_js_vectors() {
    let vectors = load_route("rgb", "oklab");
    assert_cases("rgb_to_oklab", &vectors.cases, 0.0, |input| {
        let [l, a, b] = rgb::oklab(rgb_input(input));
        // Apply JS-faithful rounding — a,b may be negative (see doc above).
        VecValue::Nums(vec![js_round(l), js_round(a), js_round(b)])
    });
}

// ── rgb → hcg ────────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::hcg(rgb: [u8; 3]) -> [f64; 3]` returning
/// RAW (unrounded) floats `[h (0-360), c (0-100), g (0-100)]`, mirroring
/// `convert.rgb.hcg` in color-convert's conversions.js (lines 779-803).
///
/// The JS algorithm:
///   chroma    = max - min
///   grayscale = chroma < 1 ? min / (1 - chroma) : 0
///   hue       = derived from max channel with `(g-b)/chroma % 6`
///               (JS remainder); then hue /= 6, hue %= 1
///   return [hue * 360, chroma * 100, grayscale * 100]
///
/// The JS public wrapper applies `Math.round` per channel — so the test
/// rounds here.
///
/// Tolerance: 0.0 after per-channel rounding (exact match against the
/// JS-rounded vectors in `tests/vectors/rgb_to_hcg.json`, 32 cases).
/// All hcg channels are non-negative, so Rust's half-away-from-zero
/// `f64::round` coincides with JS `Math.round` (half toward +infinity)
/// on this route.
#[test]
fn rgb_to_hcg_matches_js_vectors() {
    let vectors = load_route("rgb", "hcg");
    assert_cases("rgb_to_hcg", &vectors.cases, 0.0, |input| {
        let [h, c, g] = rgb::hcg(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see doc above).
        VecValue::Nums(vec![h.round(), c.round(), g.round()])
    });
}

// ── rgb → gray ────────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::gray(rgb: [u8; 3]) -> [f64; 1]` returning a
/// RAW (unrounded) float `[gray]`, mirroring `convert.rgb.gray` in
/// color-convert's conversions.js (lines 977-980).
///
/// The JS algorithm:
///   value = (r + g + b) / 3
///   return [value / 255 * 100]
///
/// Single-channel output `[f64; 1]` (unlike hsl/hsv/hwb/hcg which return
/// `[f64; 3]`, or cmyk which returns `[f64; 4]`). The JS public wrapper
/// applies `Math.round` to the single value — so the test rounds here.
///
/// Tolerance: 0.0 after rounding (exact match against the JS-rounded
/// vectors in `tests/vectors/rgb_to_gray.json`, 32 cases). The gray value
/// is non-negative, so Rust's half-away-from-zero `f64::round` coincides
/// with JS `Math.round` (half toward +infinity) on this route.
#[test]
fn rgb_to_gray_matches_js_vectors() {
    let vectors = load_route("rgb", "gray");
    assert_cases("rgb_to_gray", &vectors.cases, 0.0, |input| {
        let [v] = rgb::gray(rgb_input(input));
        // Single-channel gray: mirror the JS public wrapper's Math.round.
        VecValue::Nums(vec![v.round()])
    });
}

// ── rgb → apple ────────────────────────────────────────────────────────

/// API pinned for GREEN: `rgb::apple(rgb: [u8; 3]) -> [f64; 3]` returning
/// RAW (unrounded) floats `[r16 (0-65535), g16 (0-65535), b16 (0-65535)]`,
/// mirroring `convert.rgb.apple` in color-convert's conversions.js
/// (lines 941-943).
///
/// The JS algorithm linearly maps each u8 channel to Apple 16-bit range:
///   return [(r/255)*65535, (g/255)*65535, (b/255)*65535]
///
/// The JS public wrapper applies `Math.round` per channel — so the test
/// rounds here.
///
/// Tolerance: 0.0 after per-channel rounding (exact match against the
/// JS-rounded vectors in `tests/vectors/rgb_to_apple.json`, 32 cases).
/// All apple channels are non-negative, so Rust's half-away-from-zero
/// `f64::round` coincides with JS `Math.round` (half toward +infinity)
/// on this route.
#[test]
fn rgb_to_apple_matches_js_vectors() {
    let vectors = load_route("rgb", "apple");
    assert_cases("rgb_to_apple", &vectors.cases, 0.0, |input| {
        let [r, g, b] = rgb::apple(rgb_input(input));
        // Mirror the JS public wrapper's per-channel Math.round (see module doc).
        VecValue::Nums(vec![r.round(), g.round(), b.round()])
    });
}
