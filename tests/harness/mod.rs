//! Shared vector-test harness (issue #3).
//!
//! Loads JS-generated reference vectors from `tests/vectors/<route>.json`
//! (source of truth per AGENTS.md Rule 8) into typed serde structs and runs a
//! parametric assertion over every case with a per-route numeric tolerance.
//! Panicking `expect`/`panic!` here is the assertion mechanism — this is
//! test-support code, not library code.

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// A single vector value: `[0,0,0]`, `"000000"`, or `30`.
///
/// Untagged variant order matters: a JSON array parses as `Nums`, a JSON
/// string as `Text`, and a bare JSON number as `Num`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum VecValue {
    Nums(Vec<f64>),
    Text(String),
    Num(f64),
}

/// One input/expected pair from a vector file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Case {
    pub input: VecValue,
    pub expected: VecValue,
}

/// A whole `tests/vectors/<route>.json` file.
#[derive(Debug, Clone, Deserialize)]
pub struct Vectors {
    pub from: String,
    pub to: String,
    pub source: String,
    pub cases: Vec<Case>,
}

/// Loads and parses `tests/vectors/<route>.json` relative to the crate root.
pub fn load_vectors(route: &str) -> Vectors {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let path = dir.join(format!("{route}.json"));
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "failed to read vector file {}: {err}\navailable routes: {}",
            path.display(),
            available_routes(&dir)
        )
    });
    serde_json::from_str(&json)
        .unwrap_or_else(|err| panic!("failed to parse vector file {}: {err}", path.display()))
}

/// Sorted, comma-separated route names found in the vectors directory, so a
/// mistyped route name in one of the ~50 route suites fails with the fix in
/// the message.
fn available_routes(dir: &Path) -> String {
    let Ok(entries) = fs::read_dir(dir) else {
        return format!("<cannot read {}>", dir.display());
    };
    let mut routes: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                Some(path.file_stem()?.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    routes.sort();
    routes.join(", ")
}

/// Runs `convert` over every case and panics on the first mismatch beyond
/// `tolerance` (absolute difference for numeric values, exact for text).
pub fn assert_cases(
    route: &str,
    cases: &[Case],
    tolerance: f64,
    convert: impl Fn(&VecValue) -> VecValue,
) {
    for (index, case) in cases.iter().enumerate() {
        let actual = convert(&case.input);
        if !matches_within(&case.expected, &actual, tolerance) {
            panic!(
                "vector mismatch: route={route} case={index}\n  \
                 input:     {:?}\n  \
                 expected:  {:?}\n  \
                 actual:    {actual:?}\n  \
                 tolerance: {tolerance}",
                case.input, case.expected
            );
        }
    }
}

fn matches_within(expected: &VecValue, actual: &VecValue, tolerance: f64) -> bool {
    match (expected, actual) {
        (VecValue::Nums(exp), VecValue::Nums(act)) => {
            exp.len() == act.len() && exp.iter().zip(act).all(|(e, a)| (e - a).abs() <= tolerance)
        }
        (VecValue::Text(exp), VecValue::Text(act)) => exp == act,
        (VecValue::Num(exp), VecValue::Num(act)) => (exp - act).abs() <= tolerance,
        _ => false,
    }
}
