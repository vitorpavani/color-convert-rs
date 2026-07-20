//! Serial-SIMD vs parallel-SIMD benchmark for all 16 SIMD batch routes.
//!
//! Measures each route at N=50M (1 warmup, 3 timed iters) first serially
//! (one core, `module::fn(&input)`), then in parallel (`simd_parallel::par_batch`).
//! Records are appended to `benchmarks/results.jsonl` with per-route
//! keep/revert decisions based on the serial-vs-parallel speedup.
//!
//! ## Anti-elision rule
//!
//! `black_box` a *materialized Vec* of results — never `.count()` or `.for_each(|_|)`
//! (same elision bug fixed in #19 / #58 / #97).
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release --bin bench_simd_parallel [N] [warmup] [timed-iters]
//! ```

use std::hint::black_box;
use std::io::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use color_convert_rs::{
    simd, simd_apple, simd_cmyk, simd_hcg, simd_hsl, simd_hsv, simd_hsv_rgb, simd_hwb,
    simd_lab_xyz, simd_oklab, simd_oklab_rgb, simd_parallel, simd_xyz,
};

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

// ── Timing ──────────────────────────────────────────────────────────────

/// Best-of-N timing for a batch fn over `&[[u8;3]]` input.
fn bench_u8_batch<F, T>(
    pixels: &[[u8; 3]],
    warmup: usize,
    iters: usize,
    f: F,
) -> f64
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

/// Best-of-N timing for a batch fn over `&[[f32;3]]` input (inverse routes).
fn bench_f32_batch<F, T>(
    inputs: &[[f32; 3]],
    warmup: usize,
    iters: usize,
    f: F,
) -> f64
where
    F: Fn(&[[f32; 3]]) -> T,
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

// ── JSONL record builder ────────────────────────────────────────────────
struct BenchCtx {
    commit: String,
    host: String,
    gpu_present: bool,
}

fn append_record(
    ctx: &BenchCtx,
    route: &str,
    best_ms: f64,
    n: usize,
    iters: usize,
    warmup: usize,
    decision: &str,
    notes: &str,
    baseline_ref: Option<&str>,
) {
    let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);
    let escaped_route = route.replace('\\', "\\\\").replace('"', "\\\"");
    let ts = utc_now_iso();
    let escaped_notes = notes.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_decision = decision.replace('\\', "\\\\").replace('"', "\\\"");

    let baseline_ref_json = match baseline_ref {
        Some(r) => {
            let escaped = r.replace('\\', "\\\\").replace('"', "\\\"");
            format!(r#","baseline_ref":"{escaped}""#)
        }
        None => String::new(),
    };

    let record = format!(
        concat!(
            r#"{{"ts":"{ts}","commit":"{commit}","issue":110,"#,
            r#""route":"{route}","tier":"cpu","input_size":{n},"#,
            r#""metric":"throughput_mpx_s","value":{mps:.2},"ms":{ms:.3},"#,
            r#""iters":{iters},"warmup":{warmup},"host":"{host}","#,
            r#""gpu_present":{gp},"decision":"{decision}""#,
            r#"{baseline_ref},"notes":"{notes}"}}"#,
        ),
        ts = ts,
        commit = ctx.commit,
        route = escaped_route,
        n = n,
        mps = throughput_mps,
        ms = best_ms,
        iters = iters,
        warmup = warmup,
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

/// Measure a forward route (input: `&[[u8;3]]`) — serial then parallel.
fn bench_forward_route(
    ctx: &BenchCtx,
    pixels: &[[u8; 3]],
    n: usize,
    warmup: usize,
    iters: usize,
    route_name: &str,
    serial_fn: fn(&[[u8; 3]]) -> Vec<[f32; 3]>,
) {
    // Serial baseline
    let serial_ms = bench_u8_batch(pixels, warmup, iters, serial_fn);
    let serial_mps = (n as f64 / 1e6) / (serial_ms / 1000.0);
    append_record(
        ctx,
        route_name,
        serial_ms,
        n,
        iters,
        warmup,
        "baseline",
        &format!("serial SIMD (1 core), N={}", format_number(n)),
        None,
    );
    println!(
        "  {:<16} serial:   {:>9.3} ms  {:>10.1} MP/s",
        route_name, serial_ms, serial_mps
    );

    // Parallel
    let par_ms = bench_u8_batch(pixels, warmup, iters, |pix| {
        simd_parallel::par_batch(pix, serial_fn)
    });
    let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
    let speedup = par_mps / serial_mps;
    let decision = if par_mps > serial_mps { "kept" } else { "reverted" };
    append_record(
        ctx,
        route_name,
        par_ms,
        n,
        iters,
        warmup,
        decision,
        &format!(
            "parallel SIMD (rayon {} cores), N={}, speedup={:.2}x",
            rayon::current_num_threads(),
            format_number(n),
            speedup
        ),
        Some(&ctx.commit),
    );
    println!(
        "  {:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
        route_name, par_ms, par_mps, speedup, decision
    );
}

/// Measure an inverse route (input: `&[[f32;3]]`) — serial then parallel.
fn bench_f32_route(
    ctx: &BenchCtx,
    inputs: &[[f32; 3]],
    n: usize,
    warmup: usize,
    iters: usize,
    route_name: &str,
    serial_fn: fn(&[[f32; 3]]) -> Vec<[f32; 3]>,
) {
    // Serial baseline
    let serial_ms = bench_f32_batch(inputs, warmup, iters, serial_fn);
    let serial_mps = (n as f64 / 1e6) / (serial_ms / 1000.0);
    append_record(
        ctx,
        route_name,
        serial_ms,
        n,
        iters,
        warmup,
        "baseline",
        &format!("serial SIMD (1 core), N={}", format_number(n)),
        None,
    );
    println!(
        "  {:<16} serial:   {:>9.3} ms  {:>10.1} MP/s",
        route_name, serial_ms, serial_mps
    );

    // Parallel
    let par_ms = bench_f32_batch(inputs, warmup, iters, |inp| {
        simd_parallel::par_batch(inp, serial_fn)
    });
    let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
    let speedup = par_mps / serial_mps;
    let decision = if par_mps > serial_mps { "kept" } else { "reverted" };
    append_record(
        ctx,
        route_name,
        par_ms,
        n,
        iters,
        warmup,
        decision,
        &format!(
            "parallel SIMD (rayon {} cores), N={}, speedup={:.2}x",
            rayon::current_num_threads(),
            format_number(n),
            speedup
        ),
        Some(&ctx.commit),
    );
    println!(
        "  {:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
        route_name, par_ms, par_mps, speedup, decision
    );
}

// ── Main ────────────────────────────────────────────────────────────────
fn main() {
    let args: Vec<String> = std::env::args().collect();

    let n: usize = std::env::var("BENCH_INPUT_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| args.get(1).and_then(|s| s.parse().ok()))
        .unwrap_or(50_000_000);

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

    if n == 0 || warmup == 0 || iters == 0 {
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

    let num_threads = rayon::current_num_threads();
    println!(
        "Generated {} deterministic pixels (seed=42)",
        format_number(pixels.len())
    );
    println!(
        "Host: {host}  Cores: {num_threads}  GPU: {gpu_present}  Commit: {commit}"
    );
    println!("Warmup: {warmup}   Timed iters: {iters}\n");

    // ═══════════════════════════════════════════════════════════════════════
    // FORWARD routes (input: &[[u8;3]])
    // ═══════════════════════════════════════════════════════════════════════

    println!("─── Forward routes (rgb→X) ───");
    bench_forward_route(&ctx, &pixels, n, warmup, iters, "rgb->xyz", simd::rgb_to_xyz_batch);
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->lab",
        simd::rgb_to_lab_batch,
    );
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->hsl",
        simd_hsl::rgb_to_hsl_batch,
    );
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->hsv",
        simd_hsv::rgb_to_hsv_batch,
    );

    // rgb->cmyk (returns Vec<[f32;4]>, handle separately)
    {
        let route = "rgb->cmyk";
        // Serial
        let ser_ms = bench_u8_batch(&pixels, warmup, iters, simd_cmyk::rgb_to_cmyk_batch);
        let ser_mps = (n as f64 / 1e6) / (ser_ms / 1000.0);
        append_record(
            &ctx, route, ser_ms, n, iters, warmup, "baseline",
            &format!("serial SIMD (1 core), N={}", format_number(n)), None,
        );
        println!("  {:<16} serial:   {:>9.3} ms  {:>10.1} MP/s", route, ser_ms, ser_mps);
        // Parallel
        let par_ms = bench_u8_batch(&pixels, warmup, iters, |pix| {
            simd_parallel::par_batch(pix, simd_cmyk::rgb_to_cmyk_batch)
        });
        let par_mps = (n as f64 / 1e6) / (par_ms / 1000.0);
        let speedup = par_mps / ser_mps;
        let decision = if par_mps > ser_mps { "kept" } else { "reverted" };
        append_record(
            &ctx, route, par_ms, n, iters, warmup, decision,
            &format!(
                "parallel SIMD (rayon {} cores), N={}, speedup={:.2}x",
                rayon::current_num_threads(),
                format_number(n),
                speedup
            ),
            Some(&ctx.commit),
        );
        println!(
            "  {:<16} parallel: {:>9.3} ms  {:>10.1} MP/s  speedup={:.2}x  decision={}",
            route, par_ms, par_mps, speedup, decision
        );
    }
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->hwb",
        simd_hwb::rgb_to_hwb_batch,
    );
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->oklab",
        simd_oklab::rgb_to_oklab_batch,
    );
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->hcg",
        simd_hcg::rgb_to_hcg_batch,
    );
    bench_forward_route(
        &ctx, &pixels, n, warmup, iters, "rgb->apple",
        simd_apple::rgb_to_apple_batch,
    );

    // ── xyz→lab (intermediate, f32 input) ──
    println!("\n─── Intermediate route (xyz→lab) ───");
    // Pre-convert rgb→xyz (NOT timed)
    let xyz_inputs: Vec<[f32; 3]> = simd::rgb_to_xyz_batch(&pixels);
    bench_f32_route(
        &ctx, &xyz_inputs, n, warmup, iters, "xyz->lab",
        simd::xyz_to_lab_batch,
    );

    // ═══════════════════════════════════════════════════════════════════════
    // INVERSE routes (input: &[[f32;3]], pre-converted from rgb)
    // ═══════════════════════════════════════════════════════════════════════

    println!("\n─── Inverse routes (X→rgb) ───");

    // hsl→rgb
    let hsl_inputs: Vec<[f32; 3]> = simd_hsl::rgb_to_hsl_batch(&pixels);
    bench_f32_route(
        &ctx, &hsl_inputs, n, warmup, iters, "hsl->rgb",
        simd_hsl::hsl_to_rgb_batch,
    );

    // hsv→rgb
    let hsv_inputs: Vec<[f32; 3]> = simd_hsv::rgb_to_hsv_batch(&pixels);
    bench_f32_route(
        &ctx, &hsv_inputs, n, warmup, iters, "hsv->rgb",
        simd_hsv_rgb::hsv_to_rgb_batch,
    );

    // oklab→rgb
    let oklab_inputs: Vec<[f32; 3]> = simd_oklab::rgb_to_oklab_batch(&pixels);
    bench_f32_route(
        &ctx, &oklab_inputs, n, warmup, iters, "oklab->rgb",
        simd_oklab_rgb::oklab_to_rgb_batch,
    );

    // hcg→rgb
    let hcg_inputs: Vec<[f32; 3]> = simd_hcg::rgb_to_hcg_batch(&pixels);
    bench_f32_route(
        &ctx, &hcg_inputs, n, warmup, iters, "hcg->rgb",
        simd_hcg::hcg_to_rgb_batch,
    );

    // xyz→rgb
    // (already have xyz_inputs from above)
    bench_f32_route(
        &ctx, &xyz_inputs, n, warmup, iters, "xyz->rgb",
        simd_xyz::xyz_to_rgb_batch,
    );

    // lab→xyz (pre: rgb→xyz→lab, using two-step chain for authenticity)
    let lab_inputs: Vec<[f32; 3]> = {
        let xyz_tmp = simd::rgb_to_xyz_batch(&pixels);
        simd::xyz_to_lab_batch(&xyz_tmp)
    };
    bench_f32_route(
        &ctx, &lab_inputs, n, warmup, iters, "lab->xyz",
        simd_lab_xyz::lab_to_xyz_batch,
    );

    println!("\nAppended records to {RESULTS_PATH}");
}
