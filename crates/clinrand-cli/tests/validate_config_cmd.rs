//! Integration tests for `clinrand validate-config`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_path(name: &str) -> PathBuf {
    repo_root().join("examples").join(name)
}

fn write_temp_config(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write temp config");
    path
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn valid_simple_config_exits_success() {
    let config = example_path("simple.json");
    let output = clinrand()
        .args(["validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).contains("valid"),
        "stdout: {}",
        stdout(&output)
    );
}

#[test]
fn invalid_config_one_arm_exits_invalid_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write_temp_config(
        dir.path(),
        "one-arm.json",
        r#"{
  "schema_version": "1.0",
  "study_id": "TEST-INVALID-1",
  "protocol_version": "1.0",
  "arms": [{ "code": "A", "label": "Active", "ratio": 1 }],
  "method": "simple",
  "strata": [],
  "list_length_per_stratum": 24,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}"#,
    );
    let output = clinrand()
        .args(["validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config");
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("at least two arms are required"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn malformed_json_exits_io_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write_temp_config(dir.path(), "bad.json", "{ not valid json");
    let output = clinrand()
        .args(["validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config");
    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("failed to parse"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn missing_config_exits_io_error() {
    let missing = repo_root().join("examples/does-not-exist.json");
    let output = clinrand()
        .args(["validate-config", "--config"])
        .arg(&missing)
        .output()
        .expect("run validate-config");
    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("failed to read"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn json_mode_valid_config_prints_structured_ok() {
    let config = example_path("simple.json");
    let output = clinrand()
        .args(["--json", "validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config --json");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(value["ok"], true);
    assert_eq!(value["errors"], serde_json::json!([]));
    assert_eq!(value["warnings"], serde_json::json!([]));
}

#[test]
fn json_mode_invalid_config_prints_structured_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write_temp_config(
        dir.path(),
        "one-arm.json",
        r#"{
  "schema_version": "1.0",
  "study_id": "TEST-INVALID-1",
  "protocol_version": "1.0",
  "arms": [{ "code": "A", "label": "Active", "ratio": 1 }],
  "method": "simple",
  "strata": [],
  "list_length_per_stratum": 24,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}"#,
    );
    let output = clinrand()
        .args(["--json", "validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config --json");
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(value["ok"], false);
    let errors = value["errors"].as_array().expect("errors array");
    assert!(
        errors
            .iter()
            .any(|e| e.as_str() == Some("at least two arms are required")),
        "errors: {errors:?}"
    );
    assert_eq!(value["warnings"], serde_json::json!([]));
}

#[test]
fn per_stratum_range_warning_exits_success_human_mode() {
    let config = example_path("per-stratum-range.json");
    let output = clinrand()
        .args(["validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("per_stratum_range numbering discloses"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn per_stratum_range_warning_in_json_mode() {
    let config = example_path("per-stratum-range.json");
    let output = clinrand()
        .args(["--json", "validate-config", "--config"])
        .arg(&config)
        .output()
        .expect("run validate-config --json");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(value["ok"], true);
    assert_eq!(value["errors"], serde_json::json!([]));
    let warnings = value["warnings"].as_array().expect("warnings array");
    assert!(
        warnings.iter().any(|w| {
            w.as_str()
                == Some("per_stratum_range numbering discloses stratum membership in the randomization number")
        }),
        "warnings: {warnings:?}"
    );
}
