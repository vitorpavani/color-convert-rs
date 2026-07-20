/// Quick benchmark for lab→cmyk scalar vs convert_batch.
/// Run with: cargo test bench_lab_to_cmyk_scalar_vs_batch -- --nocapture --ignored
use std::hint::black_box;
use std::time::Instant;

use color_convert_rs::{Color, Model, convert};

/// Generate N deterministic Lab inputs (seeded for reproducibility).
fn generate_lab_inputs(n: usize) -> Vec<Color> {
    // Simple deterministic sequence: vary L from 0-100, a from -128 to 127, b from -128 to 127
    (0..n)
        .map(|i| {
            let l = ((i as f64 * 97.0 / n as f64 + 1.0) % 100.0).max(0.0);
            let a = ((i as f64 * 31.0) % 256.0) - 128.0;
            let b = ((i as f64 * 37.0) % 256.0) - 128.0;
            Color::Lab([l, a, b])
        })
        .collect()
}

/// Benchmark scalar per-pixel lab→cmyk using `convert`.
fn bench_scalar_lab_to_cmyk(input: &[Color], warmup: usize, iters: usize) -> f64 {
    // Warmup
    for _ in 0..warmup {
        for c in input {
            black_box(convert(Model::Lab, Model::Cmyk, c.clone()).unwrap());
        }
    }
    let mut best_ms = f64::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        for c in input {
            black_box(convert(Model::Lab, Model::Cmyk, c.clone()).unwrap());
        }
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if elapsed < best_ms {
            best_ms = elapsed;
        }
    }
    best_ms
}

/// Benchmark convert_batch lab→cmyk.
fn bench_batch_lab_to_cmyk(input: &[Color], warmup: usize, iters: usize) -> f64 {
    // Warmup
    for _ in 0..warmup {
        black_box(
            color_convert_rs::convert::convert_batch(Model::Lab, Model::Cmyk, input).unwrap(),
        );
    }
    let mut best_ms = f64::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        black_box(
            color_convert_rs::convert::convert_batch(Model::Lab, Model::Cmyk, input).unwrap(),
        );
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if elapsed < best_ms {
            best_ms = elapsed;
        }
    }
    best_ms
}

#[test]
#[ignore]
fn bench_lab_to_cmyk_scalar_vs_batch() {
    let n: usize = std::env::var("BENCH_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    let warmup: usize = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let iters: usize = std::env::var("BENCH_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let input = generate_lab_inputs(n);
    println!("N={n}  warmup={warmup}  iters={iters}");

    let scalar_ms = bench_scalar_lab_to_cmyk(&input, warmup, iters);
    let scalar_mps = (n as f64 / 1_000_000.0) / (scalar_ms / 1000.0);
    println!("scalar per-pixel:  best={scalar_ms:.3} ms  {scalar_mps:.1} MP/s");

    let batch_ms = bench_batch_lab_to_cmyk(&input, warmup, iters);
    let batch_mps = (n as f64 / 1_000_000.0) / (batch_ms / 1000.0);
    println!("convert_batch:      best={batch_ms:.3} ms  {batch_mps:.1} MP/s");

    let speedup = scalar_ms / batch_ms;
    let decision = if batch_ms < scalar_ms { "KEEP" } else { "DROP" };
    println!("speedup: {speedup:.2}x  decision: {decision}");
}
