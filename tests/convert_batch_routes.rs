//! Tests for the fused multi-hop `convert_batch` API (issue #118).
//!
//! Validates that `convert_batch` returns results matching the scalar
//! `convert` function per-pixel within a documented tolerance that accounts
//! for the f32/f64 precision gap across multiple chained conversion hops.
//!
//! ## Tolerance
//!
//! The SIMD batch path uses f32 throughout (via `wide::f32x8`) while the
//! scalar reference uses f64. Across the lab→xyz→rgb→cmyk chain:
//! - lab→xyz: ≤ 1e-3 per channel (f32 SIMD vs f64 scalar)
//! - xyz→rgb: ≤ 0.1 per channel (f32 SIMD vs f64 scalar)
//! - rgb→cmyk: ≤ 1e-3 per channel (f32 SIMD vs f64 scalar)
//! - Plus the f32→u8→f32 quantization step adds up to 0.5/255×100 ≈ 0.2
//!
//! A tolerance of 2.0 on the CMYK [0,100] output captures all of these
//! accumulated errors with generous margin while staying narrow enough to
//! detect real algorithmic divergence.

use color_convert_rs::{Color, Model, convert};

/// Behaviour: `convert_batch(Model::Lab, Model::Cmyk, &[Lab colors])` must
/// return CMYK results that match the scalar `convert` per-pixel within
/// tolerance 2.0 on all four CMYK channels.
///
/// This test WILL FAIL TO COMPILE because `convert_batch` does not exist yet
/// — that is the expected RED failure.
#[test]
fn convert_batch_lab_to_cmyk_matches_scalar() {
    // Representative Lab colours — hand-picked to span the gamut.
    // Values are real Lab coordinates that produce well-defined CMYK outputs.
    let lab_inputs: Vec<Color> = vec![
        Color::Lab([53.23288, 80.10933, 67.22007]),  // red-ish
        Color::Lab([87.73703, -86.18464, 83.18117]), // green-ish
        Color::Lab([32.30258, 79.1966, -107.8636]),  // blue-ish
        Color::Lab([0.0, 0.0, 0.0]),                 // black
        Color::Lab([100.0, 0.0, 0.0]),               // white
        Color::Lab([50.0, 0.0, 0.0]),                // mid-gray
        Color::Lab([60.3199, 98.2542, -60.8429]),    // purple-ish
        Color::Lab([97.1382, -21.5559, 94.4825]),    // yellow-ish
    ];

    // This call MUST FAIL TO COMPILE — `convert_batch` does not exist.
    let results = color_convert_rs::convert::convert_batch(Model::Lab, Model::Cmyk, &lab_inputs)
        .expect("convert_batch should succeed");

    assert_eq!(results.len(), lab_inputs.len());

    for (i, (input, result)) in lab_inputs.iter().zip(results.iter()).enumerate() {
        let scalar = convert(Model::Lab, Model::Cmyk, input.clone())
            .unwrap_or_else(|e| panic!("scalar convert failed for pixel {i}: {e}"));

        let batch_cmyk = match result {
            Color::Cmyk(v) => v,
            other => panic!("pixel {i}: expected Cmyk, got {other:?}"),
        };
        let scalar_cmyk = match &scalar {
            Color::Cmyk(v) => v,
            other => panic!("pixel {i}: expected Cmyk, got {other:?}"),
        };

        for (c, (&b, &s)) in batch_cmyk.iter().zip(scalar_cmyk.iter()).enumerate() {
            let channel_name = ["c", "m", "y", "k"][c];
            let diff = (b - s).abs();
            assert!(
                diff <= 2.0,
                "pixel {i} ({input:?}), channel {channel_name}: batch={b}, scalar={s}, diff={diff}",
            );
        }
    }
}
