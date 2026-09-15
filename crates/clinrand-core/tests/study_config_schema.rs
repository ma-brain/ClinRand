//! Published study-config JSON Schema must accept every file in examples/.
//!
//! The schema file and examples live at the repo root. This test is allowed
//! to read them; `clinrand-core` `src/` must not.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema_path() -> PathBuf {
    repo_root().join("docs/schema/study-config-1.0.json")
}

fn examples_dir() -> PathBuf {
    repo_root().join("examples")
}

fn example_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("examples/ must exist at {}: {err}", dir.display()))
        .map(|entry| entry.expect("read examples/ entry").path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
}

/// A removed schema, an empty examples/, or an example that no longer
/// matches the published wire format should fail this test.
#[test]
fn every_example_validates_against_the_published_schema() {
    let schema_text = fs::read_to_string(schema_path())
        .unwrap_or_else(|err| panic!("schema must exist at {}: {err}", schema_path().display()));
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema must be valid JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema must compile");

    let files = example_files(&examples_dir());
    assert!(
        !files.is_empty(),
        "examples/ must contain at least one study-config JSON file"
    );

    for path in files {
        assert_eq!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("json"),
            "every file in examples/ must be JSON, got {}",
            path.display()
        );
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let instance: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|err| panic!("{} is not valid JSON: {err}", path.display()));

        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|error| format!("{error}"))
            .collect();
        assert!(
            errors.is_empty(),
            "{} failed schema validation:\n{}",
            path.display(),
            errors.join("\n")
        );

        let study_id = instance["study_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{} is missing study_id", path.display()));
        assert!(
            study_id.starts_with("DEMO-")
                || study_id.starts_with("TEST-")
                || study_id.starts_with("EXAMPLE-"),
            "{} study_id {study_id:?} must match ^(DEMO|TEST|EXAMPLE)-",
            path.display()
        );
    }
}

fn compile_schema() -> jsonschema::Validator {
    let schema_text = fs::read_to_string(schema_path())
        .unwrap_or_else(|err| panic!("schema must exist at {}: {err}", schema_path().display()));
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

fn valid_example_value() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "1.0",
        "study_id": "DEMO-201",
        "protocol_version": "2.1",
        "arms": [
            { "code": "A", "label": "Active", "ratio": 1 },
            { "code": "P", "label": "Placebo", "ratio": 1 }
        ],
        "method": "permuted_block",
        "block": { "kind": "fixed", "size": 4 },
        "strata": [],
        "list_length_per_stratum": 24,
        "numbering": { "kind": "global", "start": 10001, "width": 5 }
    })
}

fn assert_schema_rejects(instance: &serde_json::Value, because: &str) {
    let validator = compile_schema();
    assert!(
        !validator.is_valid(instance),
        "schema should reject {because}, but accepted {instance}"
    );
}

#[test]
fn schema_rejects_zero_fixed_block_size() {
    let mut instance = valid_example_value();
    instance["block"]["size"] = serde_json::json!(0);
    assert_schema_rejects(&instance, "fixed block size 0");
}

#[test]
fn schema_rejects_zero_in_variable_block_sizes() {
    let mut instance = valid_example_value();
    instance["block"] = serde_json::json!({ "kind": "variable", "sizes": [0, 2] });
    assert_schema_rejects(&instance, "variable sizes containing 0");
}

#[test]
fn schema_rejects_fewer_than_two_arms() {
    let mut instance = valid_example_value();
    instance["arms"] = serde_json::json!([{ "code": "A", "label": "Active", "ratio": 1 }]);
    assert_schema_rejects(&instance, "a single arm");
}

#[test]
fn schema_rejects_zero_ratio() {
    let mut instance = valid_example_value();
    instance["arms"][1]["ratio"] = serde_json::json!(0);
    assert_schema_rejects(&instance, "ratio 0");
}

#[test]
fn schema_rejects_variable_size_above_24() {
    let mut instance = valid_example_value();
    instance["block"] = serde_json::json!({ "kind": "variable", "sizes": [6, 27] });
    assert_schema_rejects(&instance, "variable size 27");
}

#[test]
fn schema_rejects_duplicate_variable_sizes() {
    let mut instance = valid_example_value();
    instance["block"] = serde_json::json!({ "kind": "variable", "sizes": [6, 6] });
    assert_schema_rejects(&instance, "duplicate variable sizes");
}

/// Every file in examples/ must deserialize and pass `validate_config`.
/// `per-stratum-range.json` may carry the §5.6 disclosure warning; others
/// must have an empty warning list.
#[test]
fn every_example_passes_validate_config() {
    use clinrand_core::{validate_config, ConfigWarning, StudyConfig, ValidateOptions};

    let files = example_files(&examples_dir());
    assert!(
        !files.is_empty(),
        "examples/ must contain at least one study-config JSON file"
    );

    for path in files {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let cfg: StudyConfig = serde_json::from_str(&text).unwrap_or_else(|err| {
            panic!("{} must deserialize as StudyConfig: {err}", path.display())
        });
        let warnings = validate_config(&cfg, &ValidateOptions::default())
            .unwrap_or_else(|errs| panic!("{} failed validate_config: {errs:?}", path.display()));

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "per-stratum-range.json" {
            assert!(
                warnings
                    .iter()
                    .any(|w| matches!(w, ConfigWarning::PerStratumRangeDisclosure)),
                "{} should warn about per_stratum_range disclosure, got {warnings:?}",
                path.display()
            );
        } else {
            assert!(
                warnings.is_empty(),
                "{} should have no warnings, got {warnings:?}",
                path.display()
            );
        }
    }
}
