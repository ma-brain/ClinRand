//! Integration tests for `clinrand generate`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clinrand_package::parse_unblinded_manifest;
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

const PACKAGE_FILES: &[&str] = &[
    "checksums.txt",
    "generation-report.html",
    "list.csv",
    "list.json",
    "manifest.blinded.json",
    "manifest.unblinded.json",
    "stream.csv",
    "unblinded-report.html",
];

#[test]
fn generate_simple_config_creates_package_without_leaking_seed() {
    let config = example_path("simple.json");
    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .arg("generate")
        .args(["--config"])
        .arg(&config)
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Test Operator"])
        .output()
        .expect("run generate");

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let stdout_text = stdout(&output);
    let package_dir = stdout_text.lines().next().expect("stdout line").trim();
    let package_path = PathBuf::from(package_dir);
    assert!(package_path.is_dir(), "package dir: {package_dir}");

    for name in PACKAGE_FILES {
        assert!(
            package_path.join(name).is_file(),
            "missing {name} in {}",
            package_path.display()
        );
    }

    let manifest_text =
        fs::read_to_string(package_path.join("manifest.unblinded.json")).expect("read manifest");
    let manifest = parse_unblinded_manifest(&manifest_text).expect("parse manifest");
    assert_eq!(manifest.seed_hex.len(), 64);

    assert!(
        !stdout_text.contains(&manifest.seed_hex),
        "stdout must not contain seed_hex"
    );
    assert!(
        !stderr(&output).contains(&manifest.seed_hex),
        "stderr must not contain seed_hex"
    );
}

#[test]
fn generate_json_mode_prints_structured_output_without_seed() {
    let config = example_path("simple.json");
    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .args(["--json", "generate", "--config"])
        .arg(&config)
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Test Operator"])
        .output()
        .expect("run generate --json");

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert!(value.get("package_dir").and_then(|v| v.as_str()).is_some());
    assert!(value.get("list_sha256").and_then(|v| v.as_str()).is_some());
    assert!(value.get("record_count").and_then(|v| v.as_u64()).is_some());
    assert!(value.get("seed_hex").is_none());
    assert!(value.get("seed").is_none());

    let package_dir = value["package_dir"].as_str().expect("package_dir");
    let manifest_text =
        fs::read_to_string(PathBuf::from(package_dir).join("manifest.unblinded.json"))
            .expect("read manifest");
    let manifest = parse_unblinded_manifest(&manifest_text).expect("parse manifest");

    let stdout_text = stdout(&output);
    assert!(
        !stdout_text.contains(&manifest.seed_hex),
        "JSON stdout must not contain seed_hex"
    );
    assert_eq!(value["list_sha256"], manifest.list_sha256);
}

#[test]
fn invalid_config_exits_invalid_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = tempfile::tempdir().expect("tempdir");
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
        .arg("generate")
        .args(["--config"])
        .arg(&config)
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Test Operator"])
        .output()
        .expect("run generate");
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
}

#[test]
fn missing_out_parent_exits_io_error() {
    let config = example_path("simple.json");
    let missing_parent = repo_root().join("nonexistent-clinrand-parent-xyz/generate-out");
    let output = clinrand()
        .arg("generate")
        .args(["--config"])
        .arg(&config)
        .args(["--out"])
        .arg(&missing_parent)
        .args(["--operator", "Test Operator"])
        .output()
        .expect("run generate");
    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("output"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn per_stratum_range_prints_disclosure_warning_to_stderr() {
    let config = example_path("per-stratum-range.json");
    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .arg("generate")
        .args(["--config"])
        .arg(&config)
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Test Operator"])
        .output()
        .expect("run generate");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("per_stratum_range numbering discloses"),
        "stderr: {}",
        stderr(&output)
    );
}
