//! SIMD benchmark binary — measures scalar vs SIMD batch throughput for
//! rgb→xyz and records the result in benchmarks/results.jsonl.
//!
//! Uses the same deterministic PRNG (mulberry32, seed=42) and input size
//! (N=100000) as `benchmarks/js/bench.mjs` so comparisons are host-scoped.

use std::hint::black_box;
use std::io::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use color_convert_rs::rgb;
use color_convert_rs::simd;

fn mulberry32(state: &mut u32) -> f64 {
    *state = state.wrapping_add(0x6d2b79f5);
    let mut t = *state;
    t = (t ^ (t >> 15)).wrapping_mul(1 | t);
    t = (t ^ (t >> 7)).wrapping_mul(61 | t);
    t ^= t >> 14;
    (t as f64) / 4_294_967_296.0
}

fn generate_rgb_pixels(n: usize) -> Vec<[u8; 3]> {
    let mut state: u32 = 42;
    let mut pixels = Vec::with_capacity(n);
    for _ in 0..n {
        let r = (mulberry32(&mut state) * 256.0) as u8;
        let g = (mulberry32(&mut state) * 256.0) as u8;
        let b = (mulberry32(&mut state) * 256.0) as u8;
        pixels.push([r, g, b]);
    }
    pixels
}

fn main() {
    let n: usize = std::env::var("BENCH_INPUT_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);
    let warmup_iters: usize = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let timed_iters: usize = std::env::var("BENCH_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let host = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days = secs / 86400;
            let time = secs % 86400;
            let h = time / 3600;
            let m = (time % 3600) / 60;
            let s = time % 60;
            let ms = d.subsec_millis();
            let total_days = days as i64;
            let (y, mo, d) = civil_from_days(total_days).unwrap_or((1970, 1, 1));
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                y, mo, d, h, m, s, ms
            )
        })
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_string());

    let pixels = generate_rgb_pixels(n);

    // Scalar rgb→xyz
    let mut best_ns: u128 = u128::MAX;
    for _ in 0..warmup_iters {
        for p in &pixels {
            black_box(rgb::xyz(black_box(*p)));
        }
    }
    for _ in 0..timed_iters {
        let start = Instant::now();
        for p in &pixels {
            black_box(rgb::xyz(black_box(*p)));
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }
    let scalar_ms = best_ns as f64 / 1e6;
    let scalar_throughput = (n as f64 / 1e6) / (scalar_ms / 1000.0);

    // SIMD rgb→xyz
    let mut best_ns_simd: u128 = u128::MAX;
    for _ in 0..warmup_iters {
        black_box(simd::rgb_to_xyz_batch(&pixels));
    }
    for _ in 0..timed_iters {
        let start = Instant::now();
        black_box(simd::rgb_to_xyz_batch(&pixels));
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns_simd {
            best_ns_simd = elapsed;
        }
    }
    let simd_ms = best_ns_simd as f64 / 1e6;
    let simd_throughput = (n as f64 / 1e6) / (simd_ms / 1000.0);

    let speedup = simd_throughput / scalar_throughput;

    eprintln!("╔════════════════════════════════════════════════╗");
    eprintln!("║  rgb→xyz  N={:<8}                             ║", n);
    eprintln!("╠════════════════════════════════════════════════╣");
    eprintln!(
        "║  scalar:  {:>8.2} MP/s  ({:>8.3} ms)        ║",
        scalar_throughput, scalar_ms
    );
    eprintln!(
        "║  SIMD:    {:>8.2} MP/s  ({:>8.3} ms)        ║",
        simd_throughput, simd_ms
    );
    eprintln!("║  speedup: {:>7.1}x                           ║", speedup);
    eprintln!("╚════════════════════════════════════════════════╝");

    // Build schema-conformant JSONL records by hand (no serde dep needed).
    let scalar_json = format!(
        r#"{{"ts":"{}","commit":"{}","issue":20,"route":"rgb->xyz","tier":"cpu","input_size":{},"metric":"throughput_mpx_s","value":{},"ms":{},"iters":{},"warmup":{},"host":"{}","gpu_present":false,"decision":"baseline","notes":"scalar rgb::xyz per-pixel baseline"}}"#,
        ts, commit, n,
        format!("{:.2}", scalar_throughput),
        format!("{:.3}", scalar_ms),
        timed_iters, warmup_iters, host
    );

    let simd_json = format!(
        r#"{{"ts":"{}","commit":"{}","issue":20,"route":"rgb->xyz","tier":"cpu","input_size":{},"metric":"throughput_mpx_s","value":{},"ms":{},"iters":{},"warmup":{},"host":"{}","gpu_present":false,"baseline_ref":"scalar {}","decision":"{}","notes":"wide::f64x4 SIMD batch, {:.1}x vs scalar"}}"#,
        ts, commit, n,
        format!("{:.2}", simd_throughput),
        format!("{:.3}", simd_ms),
        timed_iters, warmup_iters, host,
        commit,
        if simd_throughput > scalar_throughput { "kept" } else { "reverted" },
        speedup
    );

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let results_path = std::path::Path::new(manifest_dir).join("benchmarks/results.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&results_path)
        .expect("failed to open results.jsonl");
    writeln!(file, "{}", scalar_json).expect("failed to write scalar record");
    writeln!(file, "{}", simd_json).expect("failed to write simd record");

    eprintln!("\nAppended 2 records to {}", results_path.display());
}

fn civil_from_days(days: i64) -> Option<(i64, u32, u32)> {
    if days < 0 {
        return None;
    }
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    Some((y, m, d))
}
