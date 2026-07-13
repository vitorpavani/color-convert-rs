//! RED test for issue #3 — forces the vector-loading test harness into existence.
//!
//! The harness (a shared `tests/harness/` module, GREEN's to implement) must:
//!   1. load `tests/vectors/<route>.json` by route name into typed serde structs
//!      (`Vectors { from, to, source, cases }`, `Case { input, expected }`,
//!      `VecValue::{Nums(Vec<f64>) | Text(String) | Num(f64)}`), and
//!   2. provide a parametric assertion that runs a conversion closure over every
//!      case within a per-route numeric tolerance and reports the first mismatch
//!      (route, input, expected, actual, tolerance).
//!
//! Expectations below come verbatim from the committed JS-generated vectors
//! (`tests/vectors/rgb_to_hsl.json`, `tests/vectors/rgb_to_hex.json`,
//! source: color-convert@3.1.3) — AGENTS.md Rule 8. Tolerance is 0.0 here
//! because the conversion under test is the identity (exact echo), which needs
//! no float slack; real routes will document per-route tolerances.

mod harness;

use harness::{Case, VecValue, assert_cases, load_route, load_vectors};
use rstest::rstest;

#[rstest]
fn harness_loads_vectors_and_checks_cases_parametrically() {
    // 1. Loader: typed parse of a numeric-route vector file, via the
    //    metadata-guarded route entry point.
    let vectors = load_route("rgb", "hsl");
    assert_eq!(vectors.from, "rgb");
    assert_eq!(vectors.to, "hsl");
    assert_eq!(vectors.source, "color-convert@3.1.3");
    assert!(
        !vectors.cases.is_empty(),
        "rgb_to_hsl.json must contain at least one case"
    );

    // 2. Parametric assertion + tolerance path, end-to-end, without any src/
    //    conversion code: feed each case's `expected` through an identity
    //    closure — it must equal itself within tolerance 0.0.
    let identity_cases: Vec<Case> = vectors
        .cases
        .iter()
        .map(|case| Case {
            input: case.expected.clone(),
            expected: case.expected.clone(),
        })
        .collect();
    assert_cases(
        "rgb_to_hsl (identity over expected)",
        &identity_cases,
        0.0,
        |value: &VecValue| value.clone(),
    );

    // 3. Loader handles string-valued expectations (hex route).
    let hex_vectors = load_vectors("rgb_to_hex");
    let first = hex_vectors
        .cases
        .first()
        .expect("rgb_to_hex.json must contain at least one case");
    assert_eq!(first.expected, VecValue::Text("000000".to_string()));

    // 4. Untagged variant ordering: a bare JSON number must parse as `Num`,
    //    never `Nums`/`Text` (rgb_to_ansi16.json case 0: [0,0,0] -> 30).
    let ansi_vectors = load_vectors("rgb_to_ansi16");
    let first = ansi_vectors
        .cases
        .first()
        .expect("rgb_to_ansi16.json must contain at least one case");
    assert_eq!(first.input, VecValue::Nums(vec![0.0, 0.0, 0.0]));
    assert_eq!(first.expected, VecValue::Num(30.0));
}
