//! Every `validation/regression/algo-v1/*.json` fixture's `list_sha256`
//! (and the other frozen hashes) must match (AGENTS.md §8, non-negotiable).
//!
//! **Never regenerate a fixture's expected hash to make this test pass.**
//! A failure here means either the change under test is wrong, or
//! `ALGO_VERSION` needs incrementing with a new `algo-vN/` fixture set —
//! see `validation/regression/README.md`. This test never edits fixtures.

use std::path::PathBuf;

use clinrand_core::generate;
use clinrand_package::{check_regression_case, load_regression_cases};

fn algo_v1_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../validation/regression/algo-v1")
}

#[test]
fn every_algo_v1_fixture_matches_current_engine_output() {
    let cases = load_regression_cases(&algo_v1_dir()).expect("load algo-v1 fixtures");
    assert!(
        !cases.is_empty(),
        "algo-v1 fixture directory must not be empty"
    );

    let mut failures = Vec::new();
    for case in &cases {
        let list = generate(&case.config, case.seed).expect("generate from frozen fixture");
        let outcome = check_regression_case(case, &list);
        if !outcome.passed {
            failures.push(format!(
                "{}: {}",
                outcome.case_id,
                outcome.failures.join("; ")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "regression fixture mismatch(es):\n{}",
        failures.join("\n")
    );
}

#[test]
fn algo_v1_fixture_case_ids_are_unique() {
    let cases = load_regression_cases(&algo_v1_dir()).expect("load algo-v1 fixtures");
    let mut ids: Vec<&str> = cases.iter().map(|c| c.case_id.as_str()).collect();
    ids.sort_unstable();
    let mut deduped = ids.clone();
    deduped.dedup();
    assert_eq!(ids, deduped, "duplicate case_id in algo-v1 fixtures");
}
