//! Smoke tests for the CLI skeleton (plan §10, Phase 5 Task 1).

use std::process::Command;

use clinrand_core::{ALGO_VERSION, ENGINE_VERSION};
use serde_json::Value;

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

#[test]
fn version_human_prints_engine_and_algo() {
    let output = clinrand()
        .arg("version")
        .output()
        .expect("run clinrand version");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains(ENGINE_VERSION));
    assert!(stdout.contains(&format!("algo_version {ALGO_VERSION}")));
}

#[test]
fn version_json_prints_structured_output() {
    let output = clinrand()
        .args(["--json", "version"])
        .output()
        .expect("run clinrand --json version");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["engine_version"], ENGINE_VERSION);
    assert_eq!(value["algo_version"], ALGO_VERSION);
}

#[test]
fn list_methods_human_prints_all_methods() {
    let output = clinrand()
        .arg("list-methods")
        .output()
        .expect("run clinrand list-methods");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("simple\n"));
    assert!(stdout.contains("permuted_block\n"));
    assert!(stdout.contains("stratified_block\n"));
}

#[test]
fn list_methods_json_prints_array() {
    let output = clinrand()
        .args(["--json", "list-methods"])
        .output()
        .expect("run clinrand --json list-methods");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(
        value,
        serde_json::json!(["simple", "permuted_block", "stratified_block"])
    );
}

#[test]
fn validation_report_default_exits_success() {
    let output = clinrand()
        .arg("validation-report")
        .output()
        .expect("run clinrand validation-report");
    assert!(
        output.status.success(),
        "validation-report should exit 0, stderr: {}",
        stderr(&output)
    );
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
