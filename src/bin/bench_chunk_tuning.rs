//! Chunk-size sweep for memory-bandwidth-bound parallel routes (issue #122).
//!
//! Benchmarks serial-vs-parallel for each candidate chunk size ∈ {65K, 262K, 524K, 1M}
//! on the 3 routes that REVERTED parallel in Wave 9 (#110): rgb→cmyk, rgb→apple, lab→xyz.
//! N=10M to keep runs fast; the sweep is the experiment, not the absolute numbers.
//!
//! Usage: `cargo run --release --bin bench_chunk_tuning`

use std::hint::black_box;
use std::io::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use color_convert_rs::{simd, simd_apple, simd_cmyk, simd_lab_xyz, simd_parallel};

const RESULTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benchmarks/results.jsonl");
const CHUNK_SIZES: [usize; 4] = [65_536, 262_144, 524_288, 1_048_576];

// ── PRNG + input generation ─────────────────────────────────────────────

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

// ── Helpers ─────────────────────────────────────────────────────────────

fn git_short_sha() -> String {
    std::process::Command::new("git")
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
        .unwrap_or_else(|| "unknown".to_string())
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn utc_now_iso() -> String {
    SystemTime::now()
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
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_string())
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut result = String::with_capacity(len + (len.saturating_sub(1)) / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
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

// ── Timing ──────────────────────────────────────────────────────────────

fn bench_u8<T, F>(pixels: &[[u8; 3]], warmup: usize, iters: usize, f: F) -> f64
where
    F: Fn(&[[u8; 3]]) -> Vec<T>,
{
    for _ in 0..warmup {
        black_box(f(pixels));
    }
    let mut best_ns: u128 = u128::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        black_box(f(pixels));
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }
    best_ns as f64 / 1e6
}

fn bench_f32<T, F>(inputs: &[[f32; 3]], warmup: usize, iters: usize, f: F) -> f64
where
    F: Fn(&[[f32; 3]]) -> Vec<T>,
{
    for _ in 0..warmup {
        black_box(f(inputs));
    }
    let mut best_ns: u128 = u128::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        black_box(f(inputs));
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }
    best_ns as f64 / 1e6
}

// ── JSONL record ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn append(
    commit: &str,
    host: &str,
    route: &str,
    best_ms: f64,
    n: usize,
    iters: usize,
    warmup: usize,
    decision: &str,
    notes: &str,
) {
    let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);
    let ts = utc_now_iso();
    let escaped_route = route.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_notes = notes.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_decision = decision.replace('\\', "\\\\").replace('"', "\\\"");

    let record = format!(
        concat!(
            r#"{{"ts":"{ts}","commit":"{commit}","issue":122,"#,
            r#""route":"{route}","tier":"cpu","input_size":{n},"#,
            r#""metric":"throughput_mpx_s","value":{mps:.2},"ms":{ms:.3},"#,
            r#""iters":{iters},"warmup":{warmup},"host":"{host}","#,
            r#""gpu_present":false,"decision":"{decision}","#,
            r#""notes":"{notes}"}}"#,
        ),
        ts = ts,
        commit = commit,
        route = escaped_route,
        n = n,
        mps = throughput_mps,
        ms = best_ms,
        iters = iters,
        warmup = warmup,
        host = host,
        decision = escaped_decision,
        notes = escaped_notes,
    );

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(RESULTS_PATH)
        .expect("must be able to open results.jsonl for append");
    writeln!(file, "{record}").expect("must be able to write record");
}

// ── Route benchmark (serial + parallel at given chunk size) ─────────────

fn bench_cmyk(
    pixels: &[[u8; 3]],
    n: usize,
    warmup: usize,
    iters: usize,
    chunk: usize,
    commit: &str,
    host: &str,
) -> f64 {
    let route = "rgb->cmyk";
    // Serial
    let ser_ms = bench_u8(pixels, warmup, iters, simd_cmyk::rgb_to_cmyk_batch);
    let ser_mps = (n as f64 / 1e6) / (ser_ms / 1000.0);
    append(
        commit,
        host,
        route,
        ser_ms,
        n,
        iters,
        warmup,
        "baseline",
        &format!(
            "serial SIMD (1 core), N={}, chunk={}",
            format_number(n),
            chunk
        ),
    );
    println!(
        "  {route:<16} serial:   {:>9.3} ms  {:>10.1} MP/s",
        ser_ms, ser_mps
    );
    // Parallel
    let par_ms = bench_u8(pixels, warmup, iters, |pix| {
        simd_parallel::par_batch_chunked(pix, chunk, simd_cmyk::rgb_to_cmyk_batch)
    });
    let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
    let speedup = par_mps / ser_mps;
    let decision = if par_mps > ser_mps {
        "kept"
    } else {
        "reverted"
    };
    append(
        commit,
        host,
        route,
        par_ms,
        n,
        iters,
        warmup,
        decision,
        &format!(
            "parallel SIMD (rayon {} cores), N={}, chunk={}, speedup={:.2}x",
            rayon::current_num_threads(),
            format_number(n),
            chunk,
            speedup
        ),
    );
    println!(
        "  {route:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
        par_ms, par_mps, speedup, decision
    );
    speedup
}

fn bench_apple(
    pixels: &[[u8; 3]],
    n: usize,
    warmup: usize,
    iters: usize,
    chunk: usize,
    commit: &str,
    host: &str,
) -> f64 {
    let route = "rgb->apple";
    let ser_ms = bench_u8(pixels, warmup, iters, simd_apple::rgb_to_apple_batch);
    let ser_mps = (n as f64 / 1e6) / (ser_ms / 1000.0);
    append(
        commit,
        host,
        route,
        ser_ms,
        n,
        iters,
        warmup,
        "baseline",
        &format!(
            "serial SIMD (1 core), N={}, chunk={}",
            format_number(n),
            chunk
        ),
    );
    println!(
        "  {route:<16} serial:   {:>9.3} ms  {:>10.1} MP/s",
        ser_ms, ser_mps
    );
    let par_ms = bench_u8(pixels, warmup, iters, |pix| {
        simd_parallel::par_batch_chunked(pix, chunk, simd_apple::rgb_to_apple_batch)
    });
    let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
    let speedup = par_mps / ser_mps;
    let decision = if par_mps > ser_mps {
        "kept"
    } else {
        "reverted"
    };
    append(
        commit,
        host,
        route,
        par_ms,
        n,
        iters,
        warmup,
        decision,
        &format!(
            "parallel SIMD (rayon {} cores), N={}, chunk={}, speedup={:.2}x",
            rayon::current_num_threads(),
            format_number(n),
            chunk,
            speedup
        ),
    );
    println!(
        "  {route:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
        par_ms, par_mps, speedup, decision
    );
    speedup
}

fn bench_lab_xyz(
    inputs: &[[f32; 3]],
    n: usize,
    warmup: usize,
    iters: usize,
    chunk: usize,
    commit: &str,
    host: &str,
) -> f64 {
    let route = "lab->xyz";
    let ser_ms = bench_f32(inputs, warmup, iters, simd_lab_xyz::lab_to_xyz_batch);
    let ser_mps = (n as f64 / 1e6) / (ser_ms / 1000.0);
    append(
        commit,
        host,
        route,
        ser_ms,
        n,
        iters,
        warmup,
        "baseline",
        &format!(
            "serial SIMD (1 core), N={}, chunk={}",
            format_number(n),
            chunk
        ),
    );
    println!(
        "  {route:<16} serial:   {:>9.3} ms  {:>10.1} MP/s",
        ser_ms, ser_mps
    );
    let par_ms = bench_f32(inputs, warmup, iters, |inp| {
        simd_parallel::par_batch_chunked(inp, chunk, simd_lab_xyz::lab_to_xyz_batch)
    });
    let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
    let speedup = par_mps / ser_mps;
    let decision = if par_mps > ser_mps {
        "kept"
    } else {
        "reverted"
    };
    append(
        commit,
        host,
        route,
        par_ms,
        n,
        iters,
        warmup,
        decision,
        &format!(
            "parallel SIMD (rayon {} cores), N={}, chunk={}, speedup={:.2}x",
            rayon::current_num_threads(),
            format_number(n),
            chunk,
            speedup
        ),
    );
    println!(
        "  {route:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
        par_ms, par_mps, speedup, decision
    );
    speedup
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = std::env::var("BENCH_INPUT_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(1).and_then(|s| s.parse().ok()))
        .unwrap_or(10_000_000);
    let warmup: usize = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(2).and_then(|s| s.parse().ok()))
        .unwrap_or(1);
    let iters: usize = std::env::var("BENCH_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(3).and_then(|s| s.parse().ok()))
        .unwrap_or(3);

    let pixels = generate_rgb_pixels(n);
    let lab_inputs: Vec<[f32; 3]> = {
        let xyz_tmp = simd::rgb_to_xyz_batch(&pixels);
        simd::xyz_to_lab_batch(&xyz_tmp)
    };

    let host = hostname();
    let commit = git_short_sha();
    let num_threads = rayon::current_num_threads();

    println!("=== Chunk-size Tuning Sweep for Reverted Parallel Routes (#122) ===");
    println!(
        "N={}  Warmup={}  Iters={}  cores={}",
        format_number(n),
        warmup,
        iters,
        num_threads
    );
    println!("Host: {host}  Commit: {commit}");
    println!("Chunk sizes: {CHUNK_SIZES:?}\n");

    for &chunk in &CHUNK_SIZES {
        println!("─────────────────────────────────────────────────────────────────");
        println!("  CHUNK = {chunk}\n");
        bench_cmyk(&pixels, n, warmup, iters, chunk, &commit, &host);
        bench_apple(&pixels, n, warmup, iters, chunk, &commit, &host);
        bench_lab_xyz(&lab_inputs, n, warmup, iters, chunk, &commit, &host);
        println!();
    }

    println!("Done. Appended records to {RESULTS_PATH}");
}
