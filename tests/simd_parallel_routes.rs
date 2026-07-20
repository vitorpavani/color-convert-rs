//! Direct correctness tests for the multi-core parallel batch dispatch.
//!
//! `simd_parallel::par_batch(input, f)` wraps any serial `_batch` fn in
//! `rayon::par_chunks(CHUNK).flat_map(f).collect()`. Because `par_chunks` is
//! order-stable and `flat_map` concatenates in chunk order, the parallel output
//! must be element-identical to the serial batch for the same input — regardless
//! of route, input type, or where N falls relative to the chunk size.
//!
//! These tests pin that contract across representative routes (different I/O
//! element types) and chunk-boundary N values (N < CHUNK, N == CHUNK, N just
//! over CHUNK, and N with a partial-SIMD-tile remainder).

use color_convert_rs::{simd, simd_apple, simd_hsl, simd_oklab, simd_oklab_rgb, simd_xyz};

/// Deterministic RGB pixel generator (mulberry32, seed=42) — matches the harness.
fn generate_rgb_pixels(n: usize) -> Vec<[u8; 3]> {
    let mut s: u32 = 42;
    (0..n)
        .map(|_| {
            s = s.wrapping_add(0x6d2b79f5);
            let z = s;
            let mut x = z;
            x ^= x >> 16;
            x = x.wrapping_mul(0x21f0aaad);
            x ^= x >> 15;
            x = x.wrapping_mul(0x735a2d97);
            x ^= x >> 15;
            let r = (x & 0xff) as u8;
            let g = ((x >> 8) & 0xff) as u8;
            let b = ((x >> 16) & 0xff) as u8;
            [r, g, b]
        })
        .collect()
}

/// N values straddling the parallel-chunk boundary (PARALLEL_CHUNK = 65_536)
/// AND the per-core SIMD tile boundary (8). Each must round-trip identically.
const CHUNK_BOUNDARY_NS: [usize; 6] = [
    1,           // tiny: single scalar-remainder pixel
    8,           // exactly one SIMD tile, no remainder
    65_536,      // exactly one parallel chunk
    65_537,      // one chunk + 1 pixel (forces chunk split + remainder)
    100_000,     // multiple chunks, non-multiple of 8 and of CHUNK
    200_003,     // larger non-multiple, exercises load balancing
];

#[test]
fn par_batch_matches_serial_rgb_to_hsl() {
    // [u8;3] -> [f32;3], branchy hue route.
    for &n in &CHUNK_BOUNDARY_NS {
        let pixels = generate_rgb_pixels(n);
        let serial = simd_hsl::rgb_to_hsl_batch(&pixels);
        let parallel = simd_parallel::par_batch(&pixels, simd_hsl::rgb_to_hsl_batch);
        assert_eq!(serial.len(), parallel.len(), "len mismatch at n={n}");
        for (i, (s, p)) in serial.iter().zip(parallel.iter()).enumerate() {
            assert_eq!(s, p, "pixel {i} differs at n={n} (rgb->hsl)");
        }
    }
}

#[test]
fn par_batch_matches_serial_rgb_to_lab() {
    // [u8;3] -> [f32;3], fused matrix+transcendental route.
    for &n in &CHUNK_BOUNDARY_NS {
        let pixels = generate_rgb_pixels(n);
        let serial = simd::rgb_to_lab_batch(&pixels);
        let parallel = simd_parallel::par_batch(&pixels, simd::rgb_to_lab_batch);
        assert_eq!(serial, parallel, "differs at n={n} (rgb->lab)");
    }
}

#[test]
fn par_batch_matches_serial_xyz_to_rgb() {
    // [f32;3] -> [f32;3], INVERSE route — proves par_batch is element-type-generic.
    for &n in &CHUNK_BOUNDARY_NS {
        let pixels = generate_rgb_pixels(n);
        // Generate f32 XYZ inputs via the existing SIMD forward path (in-gamut).
        let xyz_inputs: Vec<[f32; 3]> = simd::rgb_to_xyz_batch(&pixels);
        let serial = simd_xyz::xyz_to_rgb_batch(&xyz_inputs);
        let parallel = simd_parallel::par_batch(&xyz_inputs, simd_xyz::xyz_to_rgb_batch);
        assert_eq!(serial, parallel, "differs at n={n} (xyz->rgb, f32 input)");
    }
}

#[test]
fn par_batch_matches_serial_rgb_to_apple_and_oklab_rgb() {
    // Two more element-type shapes: trivial linear scale ([u8;3]->[f32;3]) and
    // an inverse route with f32 input (oklab->rgb). Belt-and-braces on genericity.
    for &n in &CHUNK_BOUNDARY_NS {
        let pixels = generate_rgb_pixels(n);
        // apple ([u8;3] -> [f32;3])
        let s_apple = simd_apple::rgb_to_apple_batch(&pixels);
        let p_apple = simd_parallel::par_batch(&pixels, simd_apple::rgb_to_apple_batch);
        assert_eq!(s_apple, p_apple, "differs at n={n} (rgb->apple)");
        // oklab->rgb ([f32;3] -> [f32;3], inverse, via rgb->oklab pre-conversion)
        let oklab_inputs: Vec<[f32; 3]> = simd_oklab::rgb_to_oklab_batch(&pixels);
        let s_oklabb = simd_oklab_rgb::oklab_to_rgb_batch(&oklab_inputs);
        let p_oklabb = simd_parallel::par_batch(&oklab_inputs, simd_oklab_rgb::oklab_to_rgb_batch);
        assert_eq!(s_oklabb, p_oklabb, "differs at n={n} (oklab->rgb)");
    }
}
