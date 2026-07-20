//! Multi-core parallelism for the SIMD batch routes via rayon.
//!
//! Each serial `_batch` function processes data 8 pixels at a time using
//! `wide::f32x8` SIMD lanes on a single core.  This module wraps any such
//! function in [`par_batch`], which partitions the input into order-stable
//! chunks and distributes them across all available cores via
//! [`rayon::prelude::ParallelSlice::par_chunks`].
//!
//! ## Model
//!
//! > **SIMD lanes (8-wide per core) × cores** — the SIMD path uses one core;
//! > the other N-1 cores sit idle.  `par_batch` lets every core run its own
//! > 8-wide SIMD on its chunk, turning the compute parallelism budget
//! > (transcendentals, matrix multiplies, mask-blend) into actual throughput.
//!
//! ## Order stability
//!
//! [`rayon::slice::par_chunks`] is explicitly order-stable: it processes
//! chunks in contiguous, non-overlapping input order, and
//! [`Iterator::flat_map`] concatenates the per-chunk results back in that
//! same order.  The output is therefore element-identical to the serial
//! batch for the same input, as verified by the integration test at
//! `tests/simd_parallel_routes.rs`.
//!
//! ## Chunk size
//!
//! [`PARALLEL_CHUNK`] = 65 536 pixels.  At N=50 000 000 this yields ≈ 763
//! chunks, or ≈ 27 chunks per core on a 28-core host.  The per-chunk rayon
//! overhead (work-stealing scheduling) is negligible compared to the SIMD
//! work inside each chunk, while the fine granularity keeps load balanced.
//!
//! ## Routes covered
//!
//! The generic signature `par_batch(&[I], F) -> Vec<O>` covers every SIMD
//! batch route regardless of element type (`[u8;3]→[f32;3]`,
//! `[f32;3]→[f32;3]`, `[f32;4]→[f32;3]`).  See `src/bin/bench_simd_parallel.rs`
//! for the full enumeration and per-route keep/drop decisions.

use rayon::prelude::*;

/// Default chunk size: large enough that per-chunk rayon overhead is negligible
/// vs the SIMD work, small enough for load balancing (N=50M → ~763 chunks → ~27/core on 28 cores).
pub const PARALLEL_CHUNK: usize = 65_536;

/// Run a serial `_batch` fn across cores with a configurable chunk size.
///
/// Unlike [`par_batch`] this lets callers override the default [`PARALLEL_CHUNK`]
/// for per-route tuning (e.g. memory-bandwidth-bound routes may benefit from
/// larger chunks that reduce per-chunk scheduling overhead).
/// Each chunk is processed by `f` (which itself does 8-wide f32x8 SIMD on one
/// core); rayon spreads chunks across cores.  Order is preserved.
pub fn par_batch_chunked<I, O, F>(input: &[I], chunk_size: usize, f: F) -> Vec<O>
where
    I: Sync,
    O: Send,
    F: Fn(&[I]) -> Vec<O> + Send + Sync,
{
    input.par_chunks(chunk_size).flat_map(f).collect()
}

/// Run a serial `_batch` fn across cores using the default [`PARALLEL_CHUNK`].
///
/// Each chunk is processed by `f` (which itself does 8-wide f32x8 SIMD on one
/// core); rayon spreads chunks across cores.
/// Order is preserved (par_chunks is order-stable; flat_map concatenates in chunk order).
pub fn par_batch<I, O, F>(input: &[I], f: F) -> Vec<O>
where
    I: Sync,
    O: Send,
    F: Fn(&[I]) -> Vec<O> + Send + Sync,
{
    par_batch_chunked(input, PARALLEL_CHUNK, f)
}
