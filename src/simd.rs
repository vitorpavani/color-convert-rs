//! CPU-SIMD batch conversion routes for hot matrix-heavy paths.
//!
//! Uses the [`wide`] crate for portable explicit SIMD (f64x4 lanes) to
//! process 4 pixels at once. The matrix multiply and linear combination
//! parts are SIMD-accelerated; piecewise nonlinear transforms (sRGB gamma,
//! LAB cube-root transfer) extract individual lanes, call the scalar
//! reference functions, and re-pack — matching the scalar output exactly
//! because every `f64` lane is an independent IEEE 754 computation.
//!
//! ## Routes covered
//!
//! * `rgb→xyz` — sRGB inverse gamma + sRGB→XYZ (D65) matrix
//! * `xyz→lab` — D65 white-point normalization + CIE L*a*b* transfer + linear mix
//!
//! ## Tolerance
//!
//! Each SIMD lane performs the same sequence of `f64` operations as the
//! scalar route on the same pixel, so outputs must be **bit-identical** to
//! calling the scalar function (tolerance 0.0). Documented here for
//! clarity: if a test ever observes a nonzero diff, that is a bug.
//!
//! ## Batch API
//!
//! Batch functions accept slices of pixel triples and return `Vec<[f64;3]>`,
//! processing 4 pixels at a time via `wide::f64x4` with scalar remainder
//! fallback for the final 0–3 pixels.

/// Process a batch of RGB pixels into XYZ via sRGB inverse gamma + matrix.
///
/// Processes 4 pixels at a time using `f64x4` SIMD lanes for the matrix
/// multiply; extracts lanes for the scalar piecewise gamma function and
/// re-packs. Remainder pixels (final 0–3) fall back to the scalar
/// [`crate::rgb::xyz`].
///
/// # Panics
///
/// Does not panic — every `[u8;3]` is a valid RGB triple.
pub fn rgb_to_xyz_batch(rgb: &[[u8; 3]]) -> Vec<[f64; 3]> {
    use wide::f64x4;

    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    // Process 4 pixels at a time via f64x4 lanes.
    // Each lane is an independent pixel's channel; the SIMD ops (mul, add)
    // are the same IEEE 754 f64 operations the scalar route would perform,
    // so the result is bit-identical.
    while i + 3 < n {
        let r = f64x4::new([
            f64::from(rgb[i][0]),
            f64::from(rgb[i + 1][0]),
            f64::from(rgb[i + 2][0]),
            f64::from(rgb[i + 3][0]),
        ]);
        let g = f64x4::new([
            f64::from(rgb[i][1]),
            f64::from(rgb[i + 1][1]),
            f64::from(rgb[i + 2][1]),
            f64::from(rgb[i + 3][1]),
        ]);
        let b = f64x4::new([
            f64::from(rgb[i][2]),
            f64::from(rgb[i + 1][2]),
            f64::from(rgb[i + 2][2]),
            f64::from(rgb[i + 3][2]),
        ]);

        let r_norm = r / f64x4::splat(255.0);
        let g_norm = g / f64x4::splat(255.0);
        let b_norm = b / f64x4::splat(255.0);

        // Extract lanes for the piecewise sRGB gamma (scalar-only powf).
        let r_arr = r_norm.to_array();
        let g_arr = g_norm.to_array();
        let b_arr = b_norm.to_array();
        let r_lin = f64x4::new([
            crate::rgb::srgb_nonlinear_transform_inv(r_arr[0]),
            crate::rgb::srgb_nonlinear_transform_inv(r_arr[1]),
            crate::rgb::srgb_nonlinear_transform_inv(r_arr[2]),
            crate::rgb::srgb_nonlinear_transform_inv(r_arr[3]),
        ]);
        let g_lin = f64x4::new([
            crate::rgb::srgb_nonlinear_transform_inv(g_arr[0]),
            crate::rgb::srgb_nonlinear_transform_inv(g_arr[1]),
            crate::rgb::srgb_nonlinear_transform_inv(g_arr[2]),
            crate::rgb::srgb_nonlinear_transform_inv(g_arr[3]),
        ]);
        let b_lin = f64x4::new([
            crate::rgb::srgb_nonlinear_transform_inv(b_arr[0]),
            crate::rgb::srgb_nonlinear_transform_inv(b_arr[1]),
            crate::rgb::srgb_nonlinear_transform_inv(b_arr[2]),
            crate::rgb::srgb_nonlinear_transform_inv(b_arr[3]),
        ]);

        // sRGB→XYZ (D65) matrix multiply — 9 mul + 6 add per pixel, SIMD.
        let x = r_lin * f64x4::splat(0.4124564)
            + g_lin * f64x4::splat(0.3575761)
            + b_lin * f64x4::splat(0.1804375);
        let y = r_lin * f64x4::splat(0.2126729)
            + g_lin * f64x4::splat(0.7151522)
            + b_lin * f64x4::splat(0.0721750);
        let z = r_lin * f64x4::splat(0.0193339)
            + g_lin * f64x4::splat(0.1191920)
            + b_lin * f64x4::splat(0.9503041);

        let x_arr = x.to_array();
        let y_arr = y.to_array();
        let z_arr = z.to_array();

        for j in 0..4 {
            result.push([x_arr[j] * 100.0, y_arr[j] * 100.0, z_arr[j] * 100.0]);
        }

        i += 4;
    }

    // Scalar remainder for the final 0–3 pixels.
    while i < n {
        result.push(crate::rgb::xyz(rgb[i]));
        i += 1;
    }

    result
}

/// Process a batch of XYZ pixels into CIE L*a*b* via D65 normalization + transfer.
///
/// Processes 4 pixels at a time using `f64x4` SIMD lanes for the linear
/// combination (L, a, b formulas); extracts lanes for the scalar piecewise
/// LAB transfer function (cbrt / linear) and re-packs. Remainder pixels
/// fall back to the scalar [`crate::xyz::lab`].
///
/// # Panics
///
/// Does not panic — every `[f64;3]` is a valid XYZ triple.
pub fn xyz_to_lab_batch(xyz: &[[f64; 3]]) -> Vec<[f64; 3]> {
    use wide::f64x4;

    let n = xyz.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    // D65 reference white-point divisors (CIE 1931 2° observer)
    let xn = f64x4::splat(95.047);
    let yn = f64x4::splat(100.0);
    let zn = f64x4::splat(108.883);

    while i + 3 < n {
        let x = f64x4::new([xyz[i][0], xyz[i + 1][0], xyz[i + 2][0], xyz[i + 3][0]]);
        let y = f64x4::new([xyz[i][1], xyz[i + 1][1], xyz[i + 2][1], xyz[i + 3][1]]);
        let z = f64x4::new([xyz[i][2], xyz[i + 1][2], xyz[i + 2][2], xyz[i + 3][2]]);

        let x_norm = x / xn;
        let y_norm = y / yn;
        let z_norm = z / zn;

        // Extract lanes for the piecewise LAB transfer (scalar cbrt).
        let x_arr = x_norm.to_array();
        let y_arr = y_norm.to_array();
        let z_arr = z_norm.to_array();
        let fx = f64x4::new([
            crate::xyz::lab_transfer(x_arr[0]),
            crate::xyz::lab_transfer(x_arr[1]),
            crate::xyz::lab_transfer(x_arr[2]),
            crate::xyz::lab_transfer(x_arr[3]),
        ]);
        let fy = f64x4::new([
            crate::xyz::lab_transfer(y_arr[0]),
            crate::xyz::lab_transfer(y_arr[1]),
            crate::xyz::lab_transfer(y_arr[2]),
            crate::xyz::lab_transfer(y_arr[3]),
        ]);
        let fz = f64x4::new([
            crate::xyz::lab_transfer(z_arr[0]),
            crate::xyz::lab_transfer(z_arr[1]),
            crate::xyz::lab_transfer(z_arr[2]),
            crate::xyz::lab_transfer(z_arr[3]),
        ]);

        // CIE L*a*b* linear combination — SIMD
        let l = fy * f64x4::splat(116.0) - f64x4::splat(16.0);
        let a = (fx - fy) * f64x4::splat(500.0);
        let b = (fy - fz) * f64x4::splat(200.0);

        let l_arr = l.to_array();
        let a_arr = a.to_array();
        let b_arr = b.to_array();

        for j in 0..4 {
            result.push([l_arr[j], a_arr[j], b_arr[j]]);
        }

        i += 4;
    }

    // Scalar remainder for the final 0–3 pixels.
    while i < n {
        result.push(crate::xyz::lab(xyz[i]));
        i += 1;
    }

    result
}
