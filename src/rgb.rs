//! Conversions FROM the `rgb` color model — all 10 direct routes
//! (`hsl`, `hsv`, `hwb`, `cmyk`, `xyz`, `lab`, `oklab`, `hcg`,
//! `gray`, `apple`) ported from `convert.rgb.*` in color-convert@3.1.3
//! `conversions.js`.
//!
//! Each function returns RAW (unrounded) floats — the observable
//! per-channel rounding applied by the JS public wrapper (`Math.round`) is
//! the caller's (or test's) responsibility.  Tolerance is 0.0 after
//! per-channel rounding for every route; see the vector tests in
//! `tests/rgb_routes.rs` for per-route documentation.

/// Normalize an RGB `[u8; 3]` input to per-channel `f64` fractions in `[0.0, 1.0]`,
/// returning the three channel values along with their min, max, and delta (max-min).
#[inline]
fn normalize_rgb(rgb: [u8; 3]) -> (f64, f64, f64, f64, f64, f64) {
    let r = f64::from(rgb[0]) / 255.0;
    let g = f64::from(rgb[1]) / 255.0;
    let b = f64::from(rgb[2]) / 255.0;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;
    (r, g, b, min, max, delta)
}

/// Converts an RGB triple to raw HSL floats `[h (0-360), s (0-100), l (0-100)]`.
///
/// Faithful port of `convert.rgb.hsl` (color-convert@3.1.3 conversions.js).
/// Channel comparisons use direct `==` exactly as the JS does; the compared
/// values are exact `/255.0` divisions of the same inputs, so equality is
/// well-defined and matches the JS control flow bit-for-bit.
pub fn hsl(rgb: [u8; 3]) -> [f64; 3] {
    let (r, g, b, min, max, delta) = normalize_rgb(rgb);

    let mut h = if max == min {
        0.0
    } else if r == max {
        (g - b) / delta
    } else if g == max {
        2.0 + (b - r) / delta
    } else {
        // b == max (last arm of the JS if/else chain)
        4.0 + (r - g) / delta
    };

    h = (h * 60.0).min(360.0);
    if h < 0.0 {
        h += 360.0;
    }

    let l = (min + max) / 2.0;

    let s = if max == min {
        0.0
    } else if l <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    [h, s * 100.0, l * 100.0]
}

/// Converts an RGB triple to raw HSV floats `[h (0-360), s (0-100), v (0-100)]`.
///
/// Faithful port of `convert.rgb.hsv` (color-convert@3.1.3 conversions.js,
/// lines 128-186). The JS `switch (v)` matches channels in r, g, b order via
/// strict equality; the compared values are exact `/255.0` divisions of the
/// same inputs, so direct `==` reproduces that control flow bit-for-bit.
pub fn hsv(rgb: [u8; 3]) -> [f64; 3] {
    let (r, g, b, _min, v, diff) = normalize_rgb(rgb);

    let (h, s) = if diff == 0.0 {
        (0.0, 0.0)
    } else {
        let diffc = |c: f64| (v - c) / 6.0 / diff + 0.5;
        let rdif = diffc(r);
        let gdif = diffc(g);
        let bdif = diffc(b);

        let mut h = if v == r {
            bdif - gdif
        } else if v == g {
            1.0 / 3.0 + rdif - bdif
        } else {
            // v == b (last case of the JS switch; no default arm exists)
            2.0 / 3.0 + gdif - rdif
        };

        if h < 0.0 {
            h += 1.0;
        } else if h > 1.0 {
            h -= 1.0;
        }

        (h, diff / v)
    };

    [h * 360.0, s * 100.0, v * 100.0]
}

/// Converts an RGB triple to raw HWB floats `[h (0-360), w (0-100), b (0-100)]`.
///
/// Faithful port of `convert.rgb.hwb` (color-convert@3.1.3 conversions.js,
/// lines 188-198). The hue is derived from `hsl(rgb)[0]`, while whiteness and
/// blackness are computed from the min and max of the normalized channel
/// fractions.
pub fn hwb(rgb: [u8; 3]) -> [f64; 3] {
    let (_r, _g, _b, min, max, _delta) = normalize_rgb(rgb);
    let h = hsl(rgb)[0];
    [h, min * 100.0, (1.0 - max) * 100.0]
}

/// Converts an RGB triple to raw CMYK floats `[c (0-100), m (0-100), y (0-100), k (0-100)]`.
///
/// Faithful port of `convert.rgb.cmyk` (color-convert@3.1.3 conversions.js,
/// lines 217-228). The divide-by-zero guard when `k == 1` (pure black) mirrors
/// the JS `|| 0` fallback: the expression `(1-r-k)/(1-k) || 0` evaluates the
/// division result, and if falsy (0 or NaN) falls through to 0.
pub fn cmyk(rgb: [u8; 3]) -> [f64; 4] {
    let r = f64::from(rgb[0]) / 255.0;
    let g = f64::from(rgb[1]) / 255.0;
    let b = f64::from(rgb[2]) / 255.0;

    let k = (1.0 - r).min(1.0 - g).min(1.0 - b);
    let denom = 1.0 - k;

    let (c, m, y) = if denom == 0.0 {
        // k == 1 (pure black) — guard division by zero, mirroring JS `|| 0`
        (0.0, 0.0, 0.0)
    } else {
        (
            (1.0 - r - k) / denom,
            (1.0 - g - k) / denom,
            (1.0 - b - k) / denom,
        )
    };

    [c * 100.0, m * 100.0, y * 100.0, k * 100.0]
}

/// sRGB inverse nonlinear transform (gamma expansion).
///
/// Faithful port of the anonymous function inside `convert.rgb.xyz`
/// (color-convert@3.1.3 conversions.js, lines 273-277). Given a linearised
/// channel value `c` in [0.0, 1.0], this applies the piecewise inverse
/// transfer function: `c > 0.04045 ? ((c+0.055)/1.055)^2.4 : c/12.92`.
///
/// This is a reusable helper — it is also required by rgb→lab and rgb→oklab.
fn srgb_nonlinear_transform_inv(c: f64) -> f64 {
    if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }
}

/// Converts an RGB triple to raw XYZ floats `[x (0-100), y (0-100), z (0-100)]`.
///
/// Faithful port of `convert.rgb.xyz` (color-convert@3.1.3 conversions.js,
/// lines 270-281). Each channel is normalised to [0,1] by `/255.0`, the
/// sRGB inverse nonlinear transform is applied, and then the result is
/// multiplied by the sRGB→XYZ matrix (CIE XYZ tristimulus values, D65
/// illuminant, 2° observer). The matrix coefficients are taken verbatim
/// from the JS source.
pub fn xyz(rgb: [u8; 3]) -> [f64; 3] {
    let r = srgb_nonlinear_transform_inv(f64::from(rgb[0]) / 255.0);
    let g = srgb_nonlinear_transform_inv(f64::from(rgb[1]) / 255.0);
    let b = srgb_nonlinear_transform_inv(f64::from(rgb[2]) / 255.0);

    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

    [x * 100.0, y * 100.0, z * 100.0]
}

/// Piecewise CIELAB transfer function (the `f(t)` in the CIE 1976 L*a*b*
/// specification).
///
/// Faithful port of the anonymous function inside `convert.rgb.lab`
/// (color-convert@3.1.3 conversions.js, lines 288–290). For values above
/// the threshold LAB_FT = (6/29)³, the cube root is used; below, a linear
/// segment (7.787 * t + 16/116) provides a smooth join. In the JS
/// reference, 16/116 = 4/29, but the expression `16 / 116` is evaluated
/// verbatim at each call site — the same is done here.
///
/// This helper is reusable by future lab↔xyz conversions (issue #12).
#[inline]
fn lab_transfer(t: f64) -> f64 {
    let ft = (6.0_f64 / 29.0).powi(3); // LAB_FT = (6/29)³
    if t > ft {
        t.cbrt()
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

/// Converts an RGB triple to raw Oklab floats `[l (0-100), a, b]`.
///
/// Faithful port of `convert.rgb.oklab` (color-convert@3.1.3 conversions.js,
/// lines 200–215). The algorithm:
///
/// 1. Apply [`srgb_nonlinear_transform_inv`] to each channel / 255 → linear sRGB
/// 2. Linear sRGB → LMS cone response, then ∛ each channel
/// 3. LMS' → L'a'b' via the Oklab matrix
/// 4. Scale: `[l * 100, a * 100, b * 100]`
///
/// The `a` and `b` channels may be negative (e.g. `[0, 0, 128]` →
/// `[27, -2, -19]`). The caller (or test) is responsible for per-channel
/// rounding to reproduce the JS public wrapper's `Math.round` behaviour.
pub fn oklab(rgb: [u8; 3]) -> [f64; 3] {
    let r = srgb_nonlinear_transform_inv(f64::from(rgb[0]) / 255.0);
    let g = srgb_nonlinear_transform_inv(f64::from(rgb[1]) / 255.0);
    let b = srgb_nonlinear_transform_inv(f64::from(rgb[2]) / 255.0);

    // LMS cone response — linear sRGB → LMS, then cube root
    let lp = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let mp = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let sp = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();

    // Oklab matrix — LMS' → L'a'b'
    let l = 0.2104542553 * lp + 0.7936177850 * mp - 0.0040720468 * sp;
    let aa = 1.9779984951 * lp - 2.4285922050 * mp + 0.4505937099 * sp;
    let bb = 0.0259040371 * lp + 0.7827717662 * mp - 0.8086757660 * sp;

    [l * 100.0, aa * 100.0, bb * 100.0]
}

/// Converts an RGB triple to raw CIELAB floats `[l (0-100), a, b]`.
///
/// Faithful port of `convert.rgb.lab` (color-convert@3.1.3 conversions.js,
/// lines 283–302). The conversion chains through [`xyz`] and then applies
/// the standard CIE 1976 L*a*b* formulas with D65 reference white point
/// (Xn = 95.047, Yn = 100, Zn = 108.883).
///
/// The `a` and `b` channels may be negative (e.g. green and blue primaries
/// produce strong negative a and b, respectively). The caller (or test) is
/// responsible for per-channel rounding to reproduce the JS public
/// wrapper's `Math.round` behaviour.
pub fn lab(rgb: [u8; 3]) -> [f64; 3] {
    let xyz_vals = xyz(rgb);
    let mut x = xyz_vals[0] / 95.047;
    let mut y = xyz_vals[1] / 100.0;
    let mut z = xyz_vals[2] / 108.883;

    x = lab_transfer(x);
    y = lab_transfer(y);
    z = lab_transfer(z);

    let l = 116.0 * y - 16.0;
    let a = 500.0 * (x - y);
    let b = 200.0 * (y - z);

    [l, a, b]
}

/// Converts an RGB triple to raw HCG floats `[h (0-360), c (0-100), g (0-100)]`.
///
/// Faithful port of `convert.rgb.hcg` (color-convert@3.1.3 conversions.js,
/// lines 779–803). The JS `%` operator is the IEEE 754 remainder — the same
/// semantics as Rust's `%` on `f64` — so `((g - b) / chroma) % 6.0` and
/// `hue %= 1.0` are used directly; they reproduce the JS behaviour
/// bit-for-bit, including the sign of negative remainders.
pub fn hcg(rgb: [u8; 3]) -> [f64; 3] {
    let (r, g, b, min, max, chroma) = normalize_rgb(rgb);

    let grayscale = if chroma < 1.0 {
        min / (1.0 - chroma)
    } else {
        0.0
    };

    let mut hue = if chroma <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / chroma) % 6.0
    } else if max == g {
        2.0 + (b - r) / chroma
    } else {
        4.0 + (r - g) / chroma
    };

    hue /= 6.0;
    hue %= 1.0;

    [hue * 360.0, chroma * 100.0, grayscale * 100.0]
}

/// Converts an RGB triple to a single raw gray value `[0-100]`.
///
/// Faithful port of `convert.rgb.gray` (color-convert@3.1.3 conversions.js,
/// lines 977-980). The raw `[u8; 3]` channels are averaged and then scaled:
///
/// ```text
/// value = (r + g + b) / 3
/// return [value / 255 * 100]
/// ```
///
/// Single-channel output `[f64; 1]` (unrounded). The JS public wrapper
/// applies `Math.round` to the single value, so the caller (or test) is
/// responsible for rounding to reproduce the observable JS output.
pub fn gray(rgb: [u8; 3]) -> [f64; 1] {
    let value = (f64::from(rgb[0]) + f64::from(rgb[1]) + f64::from(rgb[2])) / 3.0;
    [value / 255.0 * 100.0]
}

/// Converts an RGB triple to an ANSI-16 terminal color code (30–37, 40–47,
/// 90–97, 100–107).
///
/// Faithful port of `convert.rgb.ansi16` (color-convert@3.1.3 conversions.js,
/// lines 643–666). The algorithm:
///
/// 1. Convert RGB → HSV via [`hsv`]; extract the V (value) channel.
/// 2. Bucket the value into 0, 1, or 2 via `round(v / 50)`.
/// 3. If value == 0 → return 30 (foreground black).
/// 4. Pack the raw R/G/B channels into a 3-bit colour index
///    (`round(c/255)` for each channel c; equivalent to `c >= 128` for u8).
/// 5. `ansi = 30 + ((bbit << 2) | (gbit << 1) | rbit)`.
/// 6. If value == 2 → `ansi += 60`.
///
/// The returned `u16` is an exact integer code; no rounding tolerance applies.
pub fn ansi16(rgb: [u8; 3]) -> u16 {
    let hsv_vals = hsv(rgb);
    let value = (hsv_vals[2] / 50.0).round() as i32;

    if value == 0 {
        return 30;
    }

    // Bits: round(c/255) for u8 c is 1 iff c >= 128, 0 otherwise.
    // Provably equivalent: c>=128 ⇔ round(c/255)==1 for all u8 inputs.
    let rbit = if rgb[0] >= 128 { 1u16 } else { 0u16 };
    let gbit = if rgb[1] >= 128 { 1u16 } else { 0u16 };
    let bbit = if rgb[2] >= 128 { 1u16 } else { 0u16 };

    let mut ansi: u16 = 30 + ((bbit << 2) | (gbit << 1) | rbit);

    if value == 2 {
        ansi += 60;
    }

    ansi
}

/// Converts an RGB triple to raw Apple 16-bit RGB floats
/// `[r16 (0-65535), g16 (0-65535), b16 (0-65535)]`.
///
/// Faithful port of `convert.rgb.apple` (color-convert@3.1.3 conversions.js,
/// lines 941-943). Each 0-255 channel is linearly mapped to the 0-65535 range
/// of Apple's 16‑bit RGB colour-picker representation:
///
/// ```text
/// return [(r/255)*65535, (g/255)*65535, (b/255)*65535]
/// ```
///
/// The JS public wrapper applies `Math.round` per channel, so the caller
/// (or test) is responsible for rounding to reproduce the observable JS output.
pub fn apple(rgb: [u8; 3]) -> [f64; 3] {
    [
        (f64::from(rgb[0]) / 255.0) * 65535.0,
        (f64::from(rgb[1]) / 255.0) * 65535.0,
        (f64::from(rgb[2]) / 255.0) * 65535.0,
    ]
}

/// Converts an RGB triple to a 6-digit UPPERCASE hex string (e.g. `"8CC864"`).
///
/// Faithful port of `convert.rgb.hex` (color-convert@3.1.3 conversions.js,
/// lines 746–755). Input channels are already `u8` (0–255), so the JS rounding
/// and `& 0xFF` masking step is a no-op. The packed `u32` channel word is
/// formatted with zero-padded uppercase hex digits via the standard library's
/// `{:06X}` format specifier.
pub fn hex(rgb: [u8; 3]) -> String {
    let int_val: u32 = (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]);
    format!("{int_val:06X}")
}

/// Finds the nearest CSS color keyword for an RGB triple.
///
/// Faithful port of `convert.rgb.keyword` (color-convert@3.1.3 conversions.js,
/// lines 241–264). The algorithm is:
///
/// 1. **Exact match** — scan all entries in `color_name::CSS_COLORS` in insertion
///    order. If multiple entries share the same RGB, the *last* one wins (mirrors
///    the JS `reverseKeywords` object-assignment behaviour where a later key
///    overwrites an earlier one, e.g. `"grey"` overwrites `"gray"` for
///    `[128,128,128]`).
/// 2. **Nearest neighbour** (no exact match) — iterate in insertion order,
///    compute squared Euclidean distance with `i32` arithmetic, and track the
///    minimum with **strict `<`** so the *first* entry at the minimum distance
///    wins (ties broken by insertion order).
pub fn keyword(rgb: [u8; 3]) -> String {
    // Exact-match pass: last matching entry wins (JS reverseKeywords semantics).
    let mut exact: Option<&str> = None;
    for (name, entry_rgb) in &crate::color_name::CSS_COLORS {
        if *entry_rgb == rgb {
            exact = Some(name);
        }
    }
    if let Some(name) = exact {
        return name.to_string();
    }

    // Nearest-neighbour fallback: first entry at minimum squared distance wins
    // (strict `<`).
    let r = i32::from(rgb[0]);
    let g = i32::from(rgb[1]);
    let b = i32::from(rgb[2]);

    let mut best_name: &str = "";
    let mut best_dist: i32 = i32::MAX;

    for (name, entry_rgb) in &crate::color_name::CSS_COLORS {
        let dr = r - i32::from(entry_rgb[0]);
        let dg = g - i32::from(entry_rgb[1]);
        let db = b - i32::from(entry_rgb[2]);
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_name = name;
        }
    }

    best_name.to_string()
}
