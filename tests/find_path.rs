//! Direct unit tests for the BFS routing core `Graph::find_path`.
//!
//! `find_path` is the shortest-path engine behind the public `convert` API: it
//! runs breadth-first search over the 50-edge colour-conversion `Graph` and
//! memoises each `(from, to)` result. Prior to this file it was exercised only
//! *indirectly* through the multi-hop `convert` route tests; these tests pin its
//! observable contract directly:
//!
//! - identity (`from == to`) returns a single-node path,
//! - a direct edge returns a two-node path,
//! - BFS returns the **shortest** path (fewest hops) for multi-hop routes,
//! - repeated queries return an identical path (the per-pair cache is correct),
//! - the graph is fully strongly connected (every ordered Model pair is
//!   reachable) — so `find_path` never returns `None` for a valid pair.
//!
//! Expected paths are derived from the committed edge list in `Graph::new`
//! (AGENTS.md Rule 8 — the graph definition is the source of truth), not
//! hand-fudged.

use color_convert_rs::convert::{Graph, Model};

/// All 17 colour models, for exhaustive reachability checks.
const ALL_MODELS: [Model; 17] = [
    Model::Rgb,
    Model::Hsl,
    Model::Hsv,
    Model::Hwb,
    Model::Cmyk,
    Model::Xyz,
    Model::Lab,
    Model::Lch,
    Model::Oklab,
    Model::Oklch,
    Model::Hcg,
    Model::Apple,
    Model::Gray,
    Model::Hex,
    Model::Keyword,
    Model::Ansi16,
    Model::Ansi256,
];

#[test]
fn find_path_identity_returns_single_node() {
    let mut g = Graph::new();
    for m in ALL_MODELS {
        assert_eq!(
            g.find_path(m, m),
            Some(vec![m]),
            "identity path for {m:?} must be a single-node path"
        );
    }
}

#[test]
fn find_path_direct_edge_returns_two_nodes() {
    let mut g = Graph::new();
    // rgb->hsl is a native single edge in conversions.js.
    assert_eq!(
        g.find_path(Model::Rgb, Model::Hsl),
        Some(vec![Model::Rgb, Model::Hsl]),
        "a direct edge must yield a two-node path"
    );
    // hsl->rgb is the reverse native edge.
    assert_eq!(
        g.find_path(Model::Hsl, Model::Rgb),
        Some(vec![Model::Hsl, Model::Rgb])
    );
}

#[test]
fn find_path_returns_shortest_multi_hop_path() {
    let mut g = Graph::new();

    // rgb->lch: rgb->lab is a direct edge, lab->lch is a direct edge, so the
    // shortest path is exactly rgb -> lab -> lch (2 hops). BFS must NOT take a
    // longer route (e.g. via xyz).
    assert_eq!(
        g.find_path(Model::Rgb, Model::Lch),
        Some(vec![Model::Rgb, Model::Lab, Model::Lch]),
        "rgb->lch must be the 2-hop rgb->lab->lch"
    );

    // cmyk only decodes to rgb, so cmyk->lch = cmyk -> rgb -> lab -> lch (3 hops).
    assert_eq!(
        g.find_path(Model::Cmyk, Model::Lch),
        Some(vec![Model::Cmyk, Model::Rgb, Model::Lab, Model::Lch]),
        "cmyk->lch must route through rgb then lab"
    );

    // lch is a near-terminal source (lch->lab only), so lch->hsl must climb back
    // out: lch -> lab -> xyz -> rgb -> hsl (4 hops).
    assert_eq!(
        g.find_path(Model::Lch, Model::Hsl),
        Some(vec![
            Model::Lch,
            Model::Lab,
            Model::Xyz,
            Model::Rgb,
            Model::Hsl
        ]),
        "lch->hsl must be the 4-hop climb back through rgb"
    );
}

#[test]
fn find_path_result_is_stable_across_calls_cache() {
    let mut g = Graph::new();
    // The first call populates the per-pair cache; the second must return the
    // identical path (and must NOT leak the internal empty-vec negative-cache
    // sentinel as a Some).
    let first = g.find_path(Model::Oklch, Model::Cmyk);
    let second = g.find_path(Model::Oklch, Model::Cmyk);
    assert!(first.is_some(), "oklch->cmyk must be reachable");
    assert_eq!(
        first, second,
        "cached result must equal the first computation"
    );
    // A cached path is never empty and always starts at `from`, ends at `to`.
    let path = first.expect("reachable");
    assert_eq!(path.first(), Some(&Model::Oklch));
    assert_eq!(path.last(), Some(&Model::Cmyk));
    assert!(
        path.len() >= 2,
        "a non-identity path has at least two nodes"
    );
}

#[test]
fn find_path_graph_is_fully_strongly_connected() {
    // Every ordered (from, to) pair over the 17 models must be reachable — the
    // colour graph has no dead ends. This both documents the property and guards
    // against an edge being dropped in a future refactor (which would silently
    // make some `convert` route return an error).
    let mut g = Graph::new();
    for from in ALL_MODELS {
        for to in ALL_MODELS {
            let path = g.find_path(from, to);
            assert!(
                path.is_some(),
                "no conversion path from {from:?} to {to:?} — graph regressed"
            );
            let path = path.expect("reachable");
            assert_eq!(*path.first().expect("non-empty"), from);
            assert_eq!(*path.last().expect("non-empty"), to);
        }
    }
}
