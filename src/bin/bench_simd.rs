//! SIMD benchmark binary — measures batch throughput via wide::f64x4 SIMD
//! for hot routes and records results in benchmarks/results.jsonl.
//!
//! Uses the same deterministic PRNG (mulberry32, seed=42) as bench.mjs and
//! bench.rs so comparisons are host-scoped.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release --bin bench_simd [N] [warmup] [timed-iters]
//! # env overrides: BENCH_INPUT_SIZE, BENCH_WARMUP, BENCH_ITERS
//! ```

use std::hint::black_box;
use std::io::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use color_convert_rs::hsl;
use color_convert_rs::rgb;
use color_convert_rs::simd;
use color_convert_rs::simd_hsl;

// ── Path to the append-only results ledger ─────────────────────────────
const RESULTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benchmarks/results.jsonl");

// ── Mulberry32 PRNG ────────────────────────────────────────────────────
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

// ── Helpers ────────────────────────────────────────────────────────────
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

// ── Best-of-N timing for batch functions ───────────────────────────────
fn bench_batch<F, T>(pixels: &[[u8; 3]], warmup: usize, iters: usize, f: F) -> f64
where
    F: Fn(&[[u8; 3]]) -> T,
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

fn bench_per_pixel<F>(pixels: &[[u8; 3]], warmup: usize, iters: usize, mut f: F) -> f64
where
    F: FnMut(&[u8; 3]),
{
    for _ in 0..warmup {
        for p in pixels {
            f(black_box(p));
            black_box(());
        }
    }
    let mut best_ns: u128 = u128::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        for p in pixels {
            f(black_box(p));
            black_box(());
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }
    best_ns as f64 / 1e6
}

// ── JSONL record builder ────────────────────────────────────────────────
struct BenchCtx {
    commit: String,
    host: String,
    gpu_present: bool,
}

struct RecordParams<'a> {
    route: &'a str,
    best_ms: f64,
    n: usize,
    iters: usize,
    warmup: usize,
    decision: &'a str,
    notes: &'a str,
    baseline_ref: Option<&'a str>,
}

fn append_record(ctx: &BenchCtx, p: RecordParams<'_>) {
    let issue: u32 = std::env::var("BENCH_ISSUE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(23);
    let throughput_mps = (p.n as f64 / 1_000_000.0) / (p.best_ms / 1000.0);
    let escaped_route = p.route.replace('\\', "\\\\").replace('"', "\\\"");
    let ts = utc_now_iso();
    let escaped_notes = p.notes.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_decision = p.decision.replace('\\', "\\\\").replace('"', "\\\"");

    let baseline_ref_json = match p.baseline_ref {
        Some(r) => {
            let escaped = r.replace('\\', "\\\\").replace('"', "\\\"");
            format!(r#","baseline_ref":"{escaped}""#)
        }
        None => String::new(),
    };

    let record = format!(
        concat!(
            r#"{{"ts":"{ts}","commit":"{commit}","issue":{issue},"#,
            r#""route":"{route}","tier":"cpu","input_size":{n},"#,
            r#""metric":"throughput_mpx_s","value":{mps:.2},"ms":{ms:.3},"#,
            r#""iters":{iters},"warmup":{warmup},"host":"{host}","#,
            r#""gpu_present":{gp},"decision":"{decision}""#,
            r#"{baseline_ref},"notes":"{notes}"}}"#,
        ),
        ts = ts,
        commit = ctx.commit,
        issue = issue,
        route = escaped_route,
        n = p.n,
        mps = throughput_mps,
        ms = p.best_ms,
        iters = p.iters,
        warmup = p.warmup,
        host = ctx.host,
        gp = ctx.gpu_present,
        decision = escaped_decision,
        baseline_ref = baseline_ref_json,
        notes = escaped_notes,
    );

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(RESULTS_PATH)
        .expect("must be able to open results.jsonl for append");
    writeln!(file, "{record}").expect("must be able to write record");
}

// ── Route definitions ──────────────────────────────────────────────────
fn rgb_to_xyz_simd(pixels: &[[u8; 3]]) -> Vec<[f32; 3]> {
    simd::rgb_to_xyz_batch(pixels)
}

fn rgb_to_lab_simd(pixels: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let xyz_batch = simd::rgb_to_xyz_batch(pixels);
    simd::xyz_to_lab_batch(&xyz_batch)
}

fn rgb_to_lab_fused(pixels: &[[u8; 3]]) -> Vec<[f32; 3]> {
    simd::rgb_to_lab_batch(pixels)
}

fn rgb_to_hsl_scalar_batch(pixels: &[[u8; 3]]) -> Vec<[f64; 3]> {
    pixels.iter().map(|&p| rgb::hsl(p)).collect()
}

fn rgb_to_hsl_simd(pixels: &[[u8; 3]]) -> Vec<[f32; 3]> {
    simd_hsl::rgb_to_hsl_batch(pixels)
}

fn rgb_hsl_rgb_scalar(pixel: &[u8; 3]) {
    let h = rgb::hsl(*pixel);
    let _ = hsl::rgb(h);
}

fn rgb_hsl_rgb_scalar_batch(pixels: &[[u8; 3]]) -> Vec<[f64; 3]> {
    pixels
        .iter()
        .map(|&p| {
            let h = rgb::hsl(p);
            hsl::rgb(h)
        })
        .collect()
}

fn rgb_hsl_rgb_simd(pixels: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let hsl_batch = simd_hsl::rgb_to_hsl_batch(pixels);
    simd_hsl::hsl_to_rgb_batch(&hsl_batch)
}

// ── Main ────────────────────────────────────────────────────────────────
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // N: positional arg 1, else env, else default 100k
    let n: usize = std::env::var("BENCH_INPUT_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(1).and_then(|s| s.parse().ok()))
        .unwrap_or(100_000);

    let warmup_iters: usize = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(2).and_then(|s| s.parse().ok()))
        .unwrap_or(3);

    let timed_iters: usize = std::env::var("BENCH_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(3).and_then(|s| s.parse().ok()))
        .unwrap_or(20);

    if n == 0 || warmup_iters == 0 || timed_iters == 0 {
        eprintln!("ERROR: N, warmup, and timed-iters must be > 0");
        std::process::exit(1);
    }

    let pixels = generate_rgb_pixels(n);
    let host = hostname();
    let commit = git_short_sha();
    let gpu_present = color_convert_rs::gpu_present();
    let ctx = BenchCtx {
        commit: commit.clone(),
        host: host.clone(),
        gpu_present,
    };

    println!(
        "Generated {} deterministic pixels (seed=42)",
        format_number(pixels.len())
    );
    println!("Warmup: {warmup_iters}   Timed iters: {timed_iters}\n");

    // ── SIMD routes ────────────────────────────────────────────────────

    // rgb→xyz (SIMD batch)
    let xyz_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_to_xyz_simd);
    let xyz_mps = (n as f64 / 1e6) / (xyz_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->xyz",
            best_ms: xyz_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "baseline",
            notes: &format!("wide::f64x4 SIMD batch, N={}", format_number(n)),
            baseline_ref: None,
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [SIMD]",
        "rgb->xyz", n, xyz_ms, xyz_mps
    );

    // rgb→lab (SIMD: xyz batch → lab batch)
    let lab_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_to_lab_simd);
    let lab_mps = (n as f64 / 1e6) / (lab_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->lab",
            best_ms: lab_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "baseline",
            notes: &format!(
                "wide::f32x8 SIMD two-step chain (xyz→lab), #61 baseline, N={}",
                format_number(n)
            ),
            baseline_ref: None,
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [SIMD two-step]",
        "rgb->lab", n, lab_ms, lab_mps
    );

    // rgb→lab (SIMD: fused single-pass rgb→xyz→lab, no intermediate XYZ buffer)
    let lab_fused_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_to_lab_fused);
    let lab_fused_mps = (n as f64 / 1e6) / (lab_fused_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->lab",
            best_ms: lab_fused_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "kept",
            notes: &format!(
                "wide::f32x8 SIMD fused single-pass (rgb→lab), N={}",
                format_number(n)
            ),
            baseline_ref: Some(&ctx.commit),
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [SIMD fused]",
        "rgb->lab (fused)", n, lab_fused_ms, lab_fused_mps
    );

    // ── Scalar routes (for reference) + SIMD HSL ──────────────────────

    // rgb→hsl (scalar batch baseline — prevents compiler elimination)
    let hsl_scalar_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_to_hsl_scalar_batch);
    let hsl_scalar_mps = (n as f64 / 1e6) / (hsl_scalar_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->hsl",
            best_ms: hsl_scalar_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "baseline",
            notes: &format!(
                "Rust scalar batch baseline (pre-SIMD), N={}",
                format_number(n)
            ),
            baseline_ref: None,
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [scalar]",
        "rgb->hsl (scalar)", n, hsl_scalar_ms, hsl_scalar_mps
    );

    // rgb→hsl (SIMD batch via mask-blend)
    let hsl_simd_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_to_hsl_simd);
    let hsl_simd_mps = (n as f64 / 1e6) / (hsl_simd_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->hsl",
            best_ms: hsl_simd_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "kept",
            notes: &format!(
                "wide::f32x8 SIMD batch (mask-blend hue), N={}",
                format_number(n)
            ),
            baseline_ref: None,
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [SIMD]",
        "rgb->hsl (SIMD)", n, hsl_simd_ms, hsl_simd_mps
    );

    // rgb→hsl→rgb (scalar batch baseline)
    let hslrgb_scalar_ms =
        bench_batch(&pixels, warmup_iters, timed_iters, rgb_hsl_rgb_scalar_batch);
    let hslrgb_scalar_mps = (n as f64 / 1e6) / (hslrgb_scalar_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->hsl->rgb",
            best_ms: hslrgb_scalar_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "baseline",
            notes: &format!(
                "Rust scalar batch baseline (pre-SIMD roundtrip), N={}",
                format_number(n)
            ),
            baseline_ref: None,
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [scalar]",
        "rgb->hsl->rgb (sc)", n, hslrgb_scalar_ms, hslrgb_scalar_mps
    );

    // rgb→hsl→rgb (SIMD batch round-trip)
    let hslrgb_simd_ms = bench_batch(&pixels, warmup_iters, timed_iters, rgb_hsl_rgb_simd);
    let hslrgb_simd_mps = (n as f64 / 1e6) / (hslrgb_simd_ms / 1000.0);
    append_record(
        &ctx,
        RecordParams {
            route: "rgb->hsl->rgb",
            best_ms: hslrgb_simd_ms,
            n,
            iters: timed_iters,
            warmup: warmup_iters,
            decision: "kept",
            notes: &format!(
                "wide::f32x8 SIMD batch round-trip (rgb→hsl→rgb), N={}",
                format_number(n)
            ),
            baseline_ref: Some(&ctx.commit),
        },
    );
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [SIMD]",
        "rgb->hsl->rgb (SIMD)", n, hslrgb_simd_ms, hslrgb_simd_mps
    );

    // rgb→hsl→rgb (scalar per-pixel, for reference)
    let hslrgb_pp_ms = bench_per_pixel(&pixels, warmup_iters, timed_iters, rgb_hsl_rgb_scalar);
    let hslrgb_pp_mps = (n as f64 / 1e6) / (hslrgb_pp_ms / 1000.0);
    println!(
        "{:<18}  N={:>8}  best={:>9.3} ms  {:>10.1} MP/s  [scalar pp]",
        "rgb->hsl->rgb (pp)", n, hslrgb_pp_ms, hslrgb_pp_mps
    );

    println!("\nAppended 7 records to {}", RESULTS_PATH);
}
