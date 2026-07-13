//! Public `convert` API — colour-space routing graph with BFS multi-hop
//! pathfinding, mirroring `route.js` in color-convert@3.1.3.
//!
//! ## `Color`
//!
//! The 17 colour models, each holding raw (unrounded) `f64` channel values
//! (or `String`/`u16` for terminal/label encodings). `Color::round` applies
//! JavaScript `Math.round` semantics (half toward +∞) to every numeric
//! channel, matching the public wrapper behaviour.
//!
//! ## `Model`
//!
//! A lightweight `Copy` discriminant for the 17 colour models, used as the
//! key type in the BFS routing graph.
//!
//! ## Graph & `convert`
//!
//! `Graph` builds an adjacency map over all 50 native routes. `convert(from,
//! to, input)` validates that the `Color` variant matches `from`, finds a
//! shortest path via BFS, and chains the native conversion functions.
//! `convert_rounded` additionally applies per-channel rounding to produce
//! the observable JS public-wrapper output.

/// A concrete colour value in one of the 17 supported colour models.
///
/// All numeric variants store **raw `f64` channels** in their natural range
/// (e.g. `Rgb` is 0–255, `Hsl` hues are 0–360).  The public `convert` API
/// returns raw values; callers use `convert_rounded` or `Color::round` to
/// reproduce the per-channel `Math.round` behaviour of the JS public wrapper.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Rgb([f64; 3]),
    Hsl([f64; 3]),
    Hsv([f64; 3]),
    Hwb([f64; 3]),
    Cmyk([f64; 4]),
    Xyz([f64; 3]),
    Lab([f64; 3]),
    Lch([f64; 3]),
    Oklab([f64; 3]),
    Oklch([f64; 3]),
    Hcg([f64; 3]),
    Apple([f64; 3]),
    Gray([f64; 1]),
    Hex(String),
    Keyword(String),
    Ansi16(u16),
    Ansi256(u16),
}

impl Color {
    /// Apply JavaScript `Math.round` semantics to every numeric channel in
    /// this colour value.
    ///
    /// `Math.round` rounds half toward positive infinity:
    /// `Math.round(0.5) === 1`, `Math.round(-1.5) === -1`.  This differs
    /// from Rust's `f64::round` which rounds half away from zero
    /// (`(-1.5_f64).round() === -2.0`).  The JS-semantic rounding is
    /// implemented as `(x + 0.5).floor()`.
    ///
    /// String and `u16` variants (Hex, Keyword, Ansi16, Ansi256) are passed
    /// through unchanged — rounding does not affect them.
    #[must_use]
    pub fn round(self) -> Self {
        match self {
            Color::Rgb(v) => Color::Rgb([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hsl(v) => Color::Hsl([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hsv(v) => Color::Hsv([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hwb(v) => Color::Hwb([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Cmyk(v) => Color::Cmyk([
                js_round(v[0]),
                js_round(v[1]),
                js_round(v[2]),
                js_round(v[3]),
            ]),
            Color::Xyz(v) => Color::Xyz([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Lab(v) => Color::Lab([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Lch(v) => Color::Lch([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Oklab(v) => Color::Oklab([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Oklch(v) => Color::Oklch([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Hcg(v) => Color::Hcg([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Apple(v) => Color::Apple([js_round(v[0]), js_round(v[1]), js_round(v[2])]),
            Color::Gray(v) => Color::Gray([js_round(v[0])]),
            other => other, // Hex, Keyword, Ansi16, Ansi256 — pass through
        }
    }
}

/// JavaScript `Math.round` semantics: `(x + 0.5).floor()`.
///
/// Unlike Rust's `f64::round` (half away from zero), this rounds half
/// toward positive infinity, matching the observable behaviour of
/// `color-convert`'s public wrapper.
#[inline]
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// A lightweight `Copy` discriminant for the 17 supported colour models.
///
/// Used as the key type in the BFS routing graph and the `from`/`to`
/// parameters of the public `convert` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Rgb,
    Hsl,
    Hsv,
    Hwb,
    Cmyk,
    Xyz,
    Lab,
    Lch,
    Oklab,
    Oklch,
    Hcg,
    Apple,
    Gray,
    Hex,
    Keyword,
    Ansi16,
    Ansi256,
}

// ---- Route graph ----

use std::collections::{HashMap, VecDeque};

use crate::Error;

/// A conversion adapter: takes a `Color`, extracts the channels for the
/// expected source model, applies the native conversion, and wraps the
/// result in the target `Color` variant.
///
/// Adapters are **total** — if the input does not match the expected source
/// variant, the input is returned unchanged. The `convert` function validates
/// the variant up-front, so this is a safety net rather than a normal path.
pub type RouteFn = fn(Color) -> Color;

/// The colour-space routing graph — adjacency list over [`Model`] nodes.
///
/// Built once with all 50 native `conversions.js` edges. `find_path` runs
/// BFS to locate the shortest conversion path; `apply` chains the
/// adapters along that path.
pub struct Graph {
    adj: HashMap<Model, Vec<(Model, RouteFn)>>,
    path_cache: HashMap<(Model, Model), Vec<Model>>,
}

/// Macro: generate a route adapter `(from_model, to_model, fn)` tuple.
///
/// Pattern 1 — numeric array source (most routes):
///   `route!(Hsl, Rgb, hsl::rgb)` → adapter calls `crate::hsl::rgb(arr)`
///
/// Pattern 2 — RGB-source routes use `_f64` variants:
///   `route!(rgb_f64, Hsl, rgb::hsl_f64)`
///
/// Pattern 3 — string decoders (hex, keyword):
///   `route!(str, Hex, hex::rgb)`
///
/// Pattern 4 — u16 decoders (ansi16, ansi256):
///   `route!(u16, Ansi16, ansi16::rgb)`
macro_rules! route {
    // RGB-source via _f64 variant → String target (hex, keyword)
    // MUST precede generic ident arms — literal tokens match before idents.
    (rgb_f64_str, $to:ident, rgb :: $fn:ident) => {
        (
            Model::Rgb,
            Model::$to,
            (|c: Color| -> Color {
                if let Color::Rgb(v) = c {
                    let result = crate::rgb::$fn(v);
                    Color::$to(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // RGB-source via _f64 variant → u16 target (ansi16, ansi256)
    (rgb_f64_u16, $to:ident, rgb :: $fn:ident) => {
        (
            Model::Rgb,
            Model::$to,
            (|c: Color| -> Color {
                if let Color::Rgb(v) = c {
                    let result = crate::rgb::$fn(v);
                    Color::$to(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // RGB-source via _f64 variant (numeric target)
    (rgb_f64, $to:ident, rgb :: $fn:ident) => {
        (
            Model::Rgb,
            Model::$to,
            (|c: Color| -> Color {
                if let Color::Rgb(v) = c {
                    let result = crate::rgb::$fn(v);
                    Color::$to(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // String decoder → rgb
    (str, $from:ident, $mod:ident :: $fn:ident) => {
        (
            Model::$from,
            Model::Rgb,
            (|c: Color| -> Color {
                if let Color::$from(s) = c {
                    let result = crate::$mod::$fn(&s);
                    Color::Rgb(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // u16 decoder → rgb
    (u16_dec, $from:ident, $mod:ident :: $fn:ident) => {
        (
            Model::$from,
            Model::Rgb,
            (|c: Color| -> Color {
                if let Color::$from(n) = c {
                    let result = crate::$mod::$fn(n);
                    Color::Rgb(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // Numeric array to String (gray→hex)
    ($from:ident, str_to, $to:ident, $mod:ident :: $fn:ident) => {
        (
            Model::$from,
            Model::$to,
            (|c: Color| -> Color {
                if let Color::$from(v) = c {
                    let result = crate::$mod::$fn(v);
                    Color::$to(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
    // Numeric array → array (e.g., hsl→rgb, cmyk→rgb, lab→xyz)
    // MUST be last — catches all remaining ident patterns.
    ($from:ident, $to:ident, $mod:ident :: $fn:ident) => {
        (
            Model::$from,
            Model::$to,
            (|c: Color| -> Color {
                if let Color::$from(v) = c {
                    let result = crate::$mod::$fn(v);
                    Color::$to(result)
                } else {
                    c
                }
            }) as RouteFn,
        )
    };
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// Build the full routing graph from all 50 native colour-conversion edges
    /// defined in color-convert@3.1.3 `conversions.js`.
    pub fn new() -> Self {
        let edges: Vec<(Model, Model, RouteFn)> = vec![
            // ── rgb source (14 routes, all via _f64 adapters) ──
            route!(rgb_f64, Hsl, rgb::hsl_f64),
            route!(rgb_f64, Hsv, rgb::hsv_f64),
            route!(rgb_f64, Hwb, rgb::hwb_f64),
            route!(rgb_f64, Oklab, rgb::oklab_f64),
            route!(rgb_f64, Cmyk, rgb::cmyk_f64),
            route!(rgb_f64, Xyz, rgb::xyz_f64),
            route!(rgb_f64, Lab, rgb::lab_f64),
            route!(rgb_f64, Hcg, rgb::hcg_f64),
            route!(rgb_f64, Apple, rgb::apple_f64),
            route!(rgb_f64, Gray, rgb::gray_f64),
            route!(rgb_f64_str, Keyword, rgb::keyword_f64),
            route!(rgb_f64_u16, Ansi16, rgb::ansi16_f64),
            route!(rgb_f64_u16, Ansi256, rgb::ansi256_f64),
            route!(rgb_f64_str, Hex, rgb::hex_f64),
            // ── hsl source ──
            route!(Hsl, Rgb, hsl::rgb),
            route!(Hsl, Hsv, hsl::hsv),
            route!(Hsl, Hcg, hsl::hcg),
            // ── hsv source ──
            route!(Hsv, Rgb, hsv::rgb),
            route!(Hsv, Hsl, hsv::hsl),
            route!(Hsv, Hcg, hsv::hcg),
            route!(Hsv, Ansi16, hsv::ansi16),
            // ── hwb source ──
            route!(Hwb, Rgb, hwb::rgb),
            route!(Hwb, Hcg, hwb::hcg),
            // ── cmyk source ──
            route!(Cmyk, Rgb, cmyk::rgb),
            // ── xyz source ──
            route!(Xyz, Rgb, xyz::rgb),
            route!(Xyz, Lab, xyz::lab),
            route!(Xyz, Oklab, xyz::oklab),
            // ── lab source ──
            route!(Lab, Xyz, lab::xyz),
            route!(Lab, Lch, lab::lch),
            // ── lch source ──
            route!(Lch, Lab, lch::lab),
            // ── oklab source ──
            route!(Oklab, Oklch, oklab::oklch),
            route!(Oklab, Xyz, oklab::xyz),
            route!(Oklab, Rgb, oklab::rgb),
            // ── oklch source ──
            route!(Oklch, Oklab, oklch::oklab),
            // ── hcg source ──
            route!(Hcg, Rgb, hcg::rgb),
            route!(Hcg, Hsv, hcg::hsv),
            route!(Hcg, Hsl, hcg::hsl),
            route!(Hcg, Hwb, hcg::hwb),
            // ── apple source ──
            route!(Apple, Rgb, apple::rgb),
            // ── gray source ──
            route!(Gray, Rgb, gray::rgb),
            route!(Gray, Hsl, gray::hsl),
            route!(Gray, Hsv, gray::hsv),
            route!(Gray, Hwb, gray::hwb),
            route!(Gray, Cmyk, gray::cmyk),
            route!(Gray, Lab, gray::lab),
            route!(Gray, str_to, Hex, gray::hex),
            // ── string/u16 decoders → rgb ──
            route!(str, Keyword, keyword::rgb),
            route!(str, Hex, hex::rgb),
            route!(u16_dec, Ansi16, ansi16::rgb),
            route!(u16_dec, Ansi256, ansi256::rgb),
        ];

        let mut adj: HashMap<Model, Vec<(Model, RouteFn)>> = HashMap::new();
        for (from, to, f) in edges {
            adj.entry(from).or_default().push((to, f));
        }

        Self {
            adj,
            path_cache: HashMap::new(),
        }
    }

    /// Find the shortest conversion path from `from` to `to` using BFS.
    ///
    /// Returns `None` if no path exists. The path is cached per `(from, to)`
    /// pair for subsequent calls.
    pub fn find_path(&mut self, from: Model, to: Model) -> Option<Vec<Model>> {
        if from == to {
            return Some(vec![from]);
        }

        if let Some(cached) = self.path_cache.get(&(from, to)) {
            return Some(cached.clone());
        }

        let mut queue = VecDeque::new();
        let mut parent: HashMap<Model, Model> = HashMap::new();
        let mut visited: HashMap<Model, bool> = HashMap::new();

        queue.push_back(from);
        visited.insert(from, true);

        while let Some(current) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = vec![to];
                let mut node = to;
                while node != from {
                    node = parent[&node];
                    path.push(node);
                }
                path.reverse();
                self.path_cache.insert((from, to), path.clone());
                return Some(path);
            }

            if let Some(neighbors) = self.adj.get(&current) {
                for (next, _) in neighbors {
                    if !visited.contains_key(next) {
                        visited.insert(*next, true);
                        parent.insert(*next, current);
                        queue.push_back(*next);
                    }
                }
            }
        }

        self.path_cache.insert((from, to), vec![]);
        None
    }

    /// Apply a conversion path to an input colour, chaining the route
    /// adapters in sequence. Assumes the path is valid and the first node
    /// matches the input variant.
    fn apply(&self, path: &[Model], input: Color) -> Color {
        let mut current = input;
        for window in path.windows(2) {
            let from = window[0];
            let to = window[1];
            if let Some(neighbors) = self.adj.get(&from)
                && let Some((_, adapter)) = neighbors.iter().find(|(m, _)| *m == to)
            {
                current = adapter(current);
            }
        }
        current
    }
}

/// Convert a colour from one model to another via the shortest path found
/// by BFS over the native-route graph.
///
/// Returns raw (unrounded) floating-point channels. Use [`convert_rounded`]
/// or [`Color::round`] to reproduce the JS public wrapper's `Math.round`
/// behaviour.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if the `input` variant does not match
/// `from`, or if no conversion path exists between `from` and `to`.
pub fn convert(from: Model, to: Model, input: Color) -> Result<Color, Error> {
    let expected_variant = model_variant(&input);
    if expected_variant != from {
        return Err(Error::InvalidInput {
            message: format!("input colour is {expected_variant:?} but `from` is {from:?}"),
        });
    }

    // Static singleton — Graph::new is cheap (allocates once).
    // Safety: this is the only mutable access; the cache is per-process.
    // This uses a thread-local to avoid a global Mutex.
    thread_local! {
        static GRAPH: std::cell::RefCell<Graph> = std::cell::RefCell::new(Graph::new());
    }

    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        let path = graph
            .find_path(from, to)
            .ok_or_else(|| Error::InvalidInput {
                message: format!("no conversion path from {from:?} to {to:?}"),
            })?;
        Ok(graph.apply(&path, input))
    })
}

/// Like [`convert`], but additionally applies per-channel `Math.round`
/// semantics to the result, producing the observable output of the JS
/// public wrapper.
///
/// # Errors
///
/// Same error conditions as [`convert`].
pub fn convert_rounded(from: Model, to: Model, input: Color) -> Result<Color, Error> {
    convert(from, to, input).map(|c| c.round())
}

/// Return the [`Model`] discriminant matching this `Color` variant.
fn model_variant(c: &Color) -> Model {
    match c {
        Color::Rgb(_) => Model::Rgb,
        Color::Hsl(_) => Model::Hsl,
        Color::Hsv(_) => Model::Hsv,
        Color::Hwb(_) => Model::Hwb,
        Color::Cmyk(_) => Model::Cmyk,
        Color::Xyz(_) => Model::Xyz,
        Color::Lab(_) => Model::Lab,
        Color::Lch(_) => Model::Lch,
        Color::Oklab(_) => Model::Oklab,
        Color::Oklch(_) => Model::Oklch,
        Color::Hcg(_) => Model::Hcg,
        Color::Apple(_) => Model::Apple,
        Color::Gray(_) => Model::Gray,
        Color::Hex(_) => Model::Hex,
        Color::Keyword(_) => Model::Keyword,
        Color::Ansi16(_) => Model::Ansi16,
        Color::Ansi256(_) => Model::Ansi256,
    }
}
