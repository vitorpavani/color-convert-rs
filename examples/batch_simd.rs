//! Batch SIMD conversion: convert 100k pixels from rgb to lab using the
//! vectorized f32x8 path, demonstrating the auto-parallel speedup.
//!
//! Run: `cargo run --release --example batch_simd`

use color_convert_rs::simd;

fn main() {
    let n: usize = 100_000;
    let pixels: Vec<[u8; 3]> = (0..n)
        .map(|i| {
            let v = (i % 256) as u8;
            [v, 255 - v, (i / 256 % 256) as u8]
        })
        .collect();

    let start = std::time::Instant::now();
    let lab = simd::rgb_to_lab_batch(&pixels);
    let elapsed = start.elapsed();

    let first = &lab[0];
    let last = &lab[n - 1];
    println!("Converted {n} pixels rgb → lab in {elapsed:?}");
    println!(
        "  first: [{:.1}, {:.1}, {:.1}]",
        first[0], first[1], first[2]
    );
    println!("  last:  [{:.1}, {:.1}, {:.1}]", last[0], last[1], last[2]);
    let throughput = (n as f64) / elapsed.as_secs_f64();
    println!("  throughput: {throughput:.0} pixels/sec");
}
