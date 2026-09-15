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
