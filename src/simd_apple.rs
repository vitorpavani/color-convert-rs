use wide::f32x8;

/// Process a batch of RGB pixels into Apple RGB (16-bit channels) via SIMD.
///
/// Processes 8 pixels at a time using `f32x8` SIMD lanes. The apple
/// conversion is `channel * (65535.0 / 255.0) = channel * 257.0` —
/// a trivial per-channel linear scale. Remainder pixels (final 0–7)
/// fall back to the scalar [`crate::rgb::apple`], converting its f64
/// output to f32.
pub fn rgb_to_apple_batch(rgb: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let n = rgb.len();
    let mut result = Vec::with_capacity(n);
    let mut i = 0;

    let factor = f32x8::splat(257.0);

    while i + 7 < n {
        // Load 8 pixels' channels into separate SIMD lanes (AoS → SoA gather)
        let r = f32x8::new([
            rgb[i][0] as f32,
            rgb[i + 1][0] as f32,
            rgb[i + 2][0] as f32,
            rgb[i + 3][0] as f32,
            rgb[i + 4][0] as f32,
            rgb[i + 5][0] as f32,
            rgb[i + 6][0] as f32,
            rgb[i + 7][0] as f32,
        ]);
        let g = f32x8::new([
            rgb[i][1] as f32,
            rgb[i + 1][1] as f32,
            rgb[i + 2][1] as f32,
            rgb[i + 3][1] as f32,
            rgb[i + 4][1] as f32,
            rgb[i + 5][1] as f32,
            rgb[i + 6][1] as f32,
            rgb[i + 7][1] as f32,
        ]);
        let b = f32x8::new([
            rgb[i][2] as f32,
            rgb[i + 1][2] as f32,
            rgb[i + 2][2] as f32,
            rgb[i + 3][2] as f32,
            rgb[i + 4][2] as f32,
            rgb[i + 5][2] as f32,
            rgb[i + 6][2] as f32,
            rgb[i + 7][2] as f32,
        ]);

        // Apply linear scale: channel * 257.0
        let r_out = r * factor;
        let g_out = g * factor;
        let b_out = b * factor;

        // Scatter back to AoS
        let r_arr = r_out.to_array();
        let g_arr = g_out.to_array();
        let b_arr = b_out.to_array();

        for j in 0..8 {
            result.push([r_arr[j], g_arr[j], b_arr[j]]);
        }

        i += 8;
    }

    // Scalar remainder — delegate to f64 scalar, convert to f32
    while i < n {
        let scalar = crate::rgb::apple(rgb[i]);
        result.push([scalar[0] as f32, scalar[1] as f32, scalar[2] as f32]);
        i += 1;
    }

    result
}
