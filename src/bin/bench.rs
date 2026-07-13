//! Rust CPU-tier benchmark harness for color-convert-rs.
//!
//! Times `rgb->lab`, `rgb->hsl`, and `rgb->hsl->rgb` over a deterministic
//! pixel buffer using `std::time::Instant` (best-of-N wall time) and appends
//! one `tier: "cpu"` record per route to the append-only
//! `benchmarks/results.jsonl` ledger.
//!
//! ## Usage
//!
//! ```bash
//! # Debug (fast compilation, slow timings — for gate validation):
//! RUSTFLAGS="-Clinker-features=-lld" nix shell nixpkgs#gcc --command cargo run --bin bench
//!
//! # Release (true performance numbers):
//! RUSTFLAGS="-Clinker-features=-lld" nix shell nixpkgs#gcc --command cargo run --release --bin bench
//!
//! # Custom input size / warmup / timed iters:
//! RUSTFLAGS="-Clinker-features=-lld" nix shell nixpkgs#gcc --command cargo run --release --bin bench 200000 5 30
//! ```
//!
//! ## GPU tier
//!
//! The GPU tier is **skipped** until the runtime probe lands (issue #21). When
//! `gpu_present` becomes true after #21, a `gpu`-tier record will be written
//! alongside the `cpu` one. Until then, only `cpu`-tier records are emitted.
//!
//! ## Schema
//!
//! Every output line conforms to `benchmarks/SCHEMA.md`. The ledger is
//! append-only — re-running appends MORE lines, never rewrites history.
//! Run `jq . benchmarks/results.jsonl` to verify.

use std::fs::OpenOptions;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

// ── Path to the append-only results ledger (compile-time constant) ──────
const RESULTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benchmarks/results.jsonl");

// ── Mulberry32 deterministic PRNG (seed 42) ────────────────────────────
// Faithful Rust port of the mulberry32 PRNG used in bench.mjs, line 48–56.
// Operations map 1:1 to JS 32-bit integer semantics (wrapping mul, logical
// shifts).  The returned f64 is in [0, 1), same as JS `>>> 0 / 2^32`.

struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f64 {
        // JS: state = (state + 0x6d2b79f5) | 0
        self.state = self.state.wrapping_add(0x6D2B79F5);

        // JS: let t = Math.imul(state ^ (state >>> 15), 1 | state);
        let mut t = (self.state ^ (self.state >> 15)).wrapping_mul(1 | self.state);

        // JS: t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        let tmp = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        t ^= tmp;

        // JS: return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
        let out = t ^ (t >> 14);
        f64::from(out) / 4_294_967_296.0
    }
}

// ── Generate N deterministic RGB pixels ────────────────────────────────
// Matches bench.mjs generatePixels(), line 59–66.  Uses seed 42.
fn generate_pixels(n: usize) -> Vec<[u8; 3]> {
    let mut rng = Mulberry32::new(42);
    let mut pixels = Vec::with_capacity(n);
    for _ in 0..n {
        // JS: (rng() * 256) | 0  →  truncates to u8 (rng() ∈ [0, 1))
        let r = (rng.next() * 256.0) as u8;
        let g = (rng.next() * 256.0) as u8;
        let b = (rng.next() * 256.0) as u8;
        pixels.push([r, g, b]);
    }
    pixels
}

// ── Best-of-N timing harness ───────────────────────────────────────────
// Warmup iterations prime the CPU cache; then N timed iterations are run
// and the MINIMUM wall time is taken (best-of-N).  black_box() prevents
// the compiler from optimising away the conversion calls.
fn benchmark<F>(pixels: &[[u8; 3]], warmup: u32, iters: u32, mut f: F) -> f64
where
    F: FnMut(&[u8; 3]),
{
    // Warmup (cache / branch predictor)
    for _ in 0..warmup {
        for pixel in pixels {
            f(black_box(pixel));
            black_box(());
        }
    }

    // Timed iterations — best-of-N (min wall time)
    let mut best_ns = u128::MAX;
    for _ in 0..iters {
        let start = Instant::now();
        for pixel in pixels {
            f(black_box(pixel));
            black_box(());
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best_ns {
            best_ns = elapsed;
        }
    }

    best_ns as f64 / 1_000_000.0
}

// ── Route benchmarks ───────────────────────────────────────────────────
// Each function calls the native Rust conversion for a single pixel.
// Returns () to keep the measurement overhead minimal — black_box
// prevents the call from being optimised away.

fn bench_rgb_to_hsl(pixel: &[u8; 3]) {
    let _ = color_convert_rs::rgb::hsl(*pixel);
}

fn bench_rgb_to_lab(pixel: &[u8; 3]) {
    let _ = color_convert_rs::rgb::lab(*pixel);
}

fn bench_rgb_hsl_rgb(pixel: &[u8; 3]) {
    let hsl = color_convert_rs::rgb::hsl(*pixel);
    let _ = color_convert_rs::hsl::rgb(hsl);
}

// ── Helpers for JSONL record fields ────────────────────────────────────

/// Git short SHA (or "unknown" if git is unavailable).
fn git_short_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from("unknown"))
}

/// Current UTC timestamp in ISO-8601 with milliseconds (e.g. `2026-07-14T12:30:00.123Z`).
fn utc_now_iso() -> String {
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%S.%3NZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from("1970-01-01T00:00:00.000Z"))
}

/// Hostname for host-scoped comparisons.
fn hostname() -> String {
    // `hostname` returns the system hostname via gethostname(2).
    // Fall back to env vars or "unknown".
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

// ── JSONL record builder ───────────────────────────────────────────────
/// Builds a single, schema-conformant JSON record string for a CPU-tier
/// benchmark run and appends it to `benchmarks/results.jsonl`.
///
/// All required fields per `benchmarks/SCHEMA.md` are present.
fn append_record(route: &str, best_ms: f64, n: usize, iters: u32, warmup: u32) {
    let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);

    // Build JSON manually to avoid a serde_json dependency (Rule 9).
    // Escape the route string defensively — route names are simple ASCII
    // identifiers, but we escape backslashes and double-quotes.
    let escaped_route = route.replace('\\', "\\\\").replace('"', "\\\"");
    let ts = utc_now_iso();
    let commit = git_short_sha();
    let host = hostname();

    let record = format!(
        concat!(
            r#"{{"ts":"{ts}","commit":"{commit}","issue":19,"#,
            r#""route":"{route}","tier":"cpu","input_size":{n},"#,
            r#""metric":"throughput_mpx_s","value":{mps:.2},"ms":{ms:.3},"#,
            r#""iters":{iters},"warmup":{warmup},"host":"{host}","#,
            r#""gpu_present":false,"decision":"baseline","#,
            r#""notes":"Rust CPU (std::time best-of-N) baseline, N={n_human}"}}"#,
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
        n_human = format_number(n),
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(RESULTS_PATH)
        .expect("must be able to open results.jsonl for append");

    writeln!(file, "{record}").expect("must be able to write record");
}

/// Format a number with commas as thousands separators (e.g. 100000 → "100,000").
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

// ── Route table ────────────────────────────────────────────────────────
// Listed in bench.mjs order for cross-tier comparability.
type BenchFn = fn(&[u8; 3]);

const ROUTES: &[(&str, BenchFn)] = &[
    ("rgb->hsl", bench_rgb_to_hsl),
    ("rgb->lab", bench_rgb_to_lab),
    ("rgb->hsl->rgb", bench_rgb_hsl_rgb),
];

// ── Main ───────────────────────────────────────────────────────────────
fn main() {
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

    for &(route, bench_fn) in ROUTES {
        let best_ms = benchmark(&pixels, warmup, iters, bench_fn);
        let throughput_mps = (n as f64 / 1_000_000.0) / (best_ms / 1000.0);

        append_record(route, best_ms, n, iters, warmup);

        // Human-readable summary — matches bench.mjs output style
        println!(
            "{route:<18}  N={n:>8}  best={ms:>9.3} ms  {mps:>10.1} MP/s",
            route = route,
            n = n,
            ms = best_ms,
            mps = throughput_mps,
        );
    }

    println!("\nAppended {} records to {}", ROUTES.len(), RESULTS_PATH,);
}
