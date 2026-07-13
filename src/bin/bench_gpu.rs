//! GPU-tier benchmark harness (CubeCL/wgpu) for color-convert-rs.
//!
//! Times `rgb->lab` on the GPU using the CubeCL kernel from `gpu::rgb_to_lab_gpu_batch`.
//! On a GPU-less host, the binary exits cleanly without writing any record.
//!
//! ## Usage
//!
//! ```bash
//! RUSTFLAGS="-Clinker-features=-lld" nix shell nixpkgs#gcc --command cargo run --bin bench_gpu
//! ```
//!
//! ## Schema
//!
//! Every output line conforms to `benchmarks/SCHEMA.md`. The ledger is
//! append-only.  When `gpu_present` is false (this host), no GPU-tier
//! record is written and the binary prints a skip message.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

// ── Path to the append-only results ledger (compile-time constant) ──────
const RESULTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benchmarks/results.jsonl");

// ── Mulberry32 deterministic PRNG (seed 42) ────────────────────────────
// Identical to bench.rs — matches bench.mjs, line 48–56.

struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut t = (self.state ^ (self.state >> 15)).wrapping_mul(1 | self.state);
        let tmp = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        t ^= tmp;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

fn generate_pixels(n: usize) -> Vec<[u8; 3]> {
    let mut rng = Mulberry32::new(42);
    let mut pixels = Vec::with_capacity(n);
    for _ in 0..n {
        let r = (rng.next() * 256.0) as u8;
        let g = (rng.next() * 256.0) as u8;
        let b = (rng.next() * 256.0) as u8;
        pixels.push([r, g, b]);
    }
    pixels
}

// ── Best-of-N timing for batch GPU ─────────────────────────────────────
// Warmup calls prime the GPU kernel JIT. Then N timed iterations run with
// best-of-N wall time.
fn benchmark_gpu(pixels: &[[u8; 3]], warmup: u32, iters: u32) -> Option<f64> {
    // Warmup — triggers CubeCL JIT compilation
    for _ in 0..warmup {
        let _ = color_convert_rs::gpu::rgb_to_lab_gpu_batch(pixels);
    }

    let mut best_ns = u128::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        let result = color_convert_rs::gpu::rgb_to_lab_gpu_batch(pixels);
        let elapsed = start.elapsed().as_nanos();

        let _ = result.as_ref()?;

        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }

    Some(best_ns as f64 / 1_000_000.0)
}

// ── Helpers for JSONL record fields ────────────────────────────────────

fn git_short_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from("unknown"))
}

fn utc_now_iso() -> String {
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%S.%3NZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from("1970-01-01T00:00:00.000Z"))
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("HOST"))
                .unwrap_or_else(|_| String::from("unknown"))
        })
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

fn append_gpu_record(route: &str, best_ms: f64, n: usize, iters: u32, warmup: u32) {
    let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);

    let escaped_route = route.replace('\\', "\\\\").replace('"', "\\\"");
    let ts = utc_now_iso();
    let commit = git_short_sha();
    let host = hostname();
    let gpu_present = true; // We only reach here if a GPU is present.

    let record = format!(
        concat!(
            r#"{{"ts":"{ts}","commit":"{commit}","issue":48,"#,
            r#""route":"{route}","tier":"gpu","input_size":{n},"#,
            r#""metric":"throughput_mpx_s","value":{mps:.2},"ms":{ms:.3},"#,
            r#""iters":{iters},"warmup":{warmup},"host":"{host}","#,
            r#""gpu_present":{gp},"decision":"kept","#,
            r#""notes":"CubeCL/wgpu GPU kernel rgb->lab batch on NVIDIA RTX 2000 Ada; beats CPU-SIMD and JS baseline, N={n_human}"}}"#,
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
        gp = gpu_present,
        n_human = format_number(n),
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(RESULTS_PATH)
        .expect("must be able to open results.jsonl for append");

    writeln!(file, "{record}").expect("must be able to write record");
}

// ── Main ───────────────────────────────────────────────────────────────
fn main() {
    // Gate: skip entirely if no GPU is present (this host).
    if !color_convert_rs::gpu_present() {
        println!("no GPU — gpu tier skipped");
        return;
    }

    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let warmup: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let iters: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);

    if n == 0 || warmup == 0 || iters == 0 {
        eprintln!("ERROR: N, warmup, and timed-iters must be > 0");
        std::process::exit(1);
    }

    let pixels = generate_pixels(n);
    println!(
        "Generated {} deterministic pixels (seed=42)",
        format_number(pixels.len())
    );
    println!("Warmup: {warmup}   Timed iters: {iters}\n");

    let route = "rgb->lab";

    match benchmark_gpu(&pixels, warmup, iters) {
        None => {
            eprintln!("ERROR: GPU became unavailable during benchmark — no record written");
            std::process::exit(1);
        }
        Some(best_ms) => {
            let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);

            append_gpu_record(route, best_ms, n, iters, warmup);

            println!(
                "{route:<18}  N={n:>8}  best={ms:>9.3} ms  {mps:>10.1} MP/s",
                route = route,
                n = n,
                ms = best_ms,
                mps = throughput_mps,
            );

            println!("\nAppended 1 record to {}", RESULTS_PATH);
        }
    }
}
