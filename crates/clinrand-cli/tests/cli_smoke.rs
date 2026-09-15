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
fn stub_commands_exit_check_failure() {
    for (args, name) in [
        (
            vec!["validate-config", "--config", "missing.json"],
            "validate-config",
        ),
        (
            vec![
                "generate",
                "--config",
                "missing.json",
                "--out",
                "/tmp/out",
                "--operator",
                "Test User",
            ],
            "generate",
        ),
        (
            vec![
                "reproduce",
                "--manifest",
                "missing.json",
                "--out",
                "/tmp/out",
            ],
            "reproduce",
        ),
        (vec!["verify", "--package", "/tmp/pkg"], "verify"),
        (vec!["validation-report"], "validation-report"),
    ] {
        let output = clinrand()
            .args(&args)
            .output()
            .unwrap_or_else(|e| panic!("run clinrand {name}: {e}"));
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name} should exit 1, stderr: {}",
            stderr(&output)
        );
        let stderr = stderr(&output);
        assert!(
            stderr.contains("not yet implemented"),
            "{name} stderr: {stderr}"
        );
    }
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
