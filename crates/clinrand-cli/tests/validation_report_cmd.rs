//! Integration tests for `clinrand validation-report`.

use std::process::Command;

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

#[test]
fn reference_tier_md_report_passes() {
    let output = clinrand()
        .args(["validation-report", "--tier", "reference", "--format", "md"])
        .output()
        .expect("run clinrand validation-report");
    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        stderr(&output)
    );
    let stdout = stdout(&output);
    assert!(stdout.contains("chacha20"), "stdout: {stdout}");
    assert!(stdout.contains("uniform-below"), "stdout: {stdout}");
    assert!(stdout.contains("fisher-yates"), "stdout: {stdout}");
    assert!(stdout.contains("PASS"), "stdout: {stdout}");
}

#[test]
fn regression_tier_reports_pass_against_algo_v1_fixtures() {
    let output = clinrand()
        .args([
            "validation-report",
            "--tier",
            "regression",
            "--format",
            "md",
        ])
        .output()
        .expect("run clinrand validation-report");
    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        stderr(&output)
    );
    let stdout = stdout(&output);
    assert!(stdout.contains("algo-v1"), "stdout: {stdout}");
    assert!(
        stdout.contains("algo-v1-simple-global: PASS"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.to_ascii_uppercase().contains("SKIP"),
        "regression tier must not skip once algo-v1 fixtures exist, stdout: {stdout}"
    );
}

#[test]
fn html_format_contains_html_document() {
    let output = clinrand()
        .args([
            "validation-report",
            "--tier",
            "reference",
            "--format",
            "html",
        ])
        .output()
        .expect("run clinrand validation-report");
    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        stderr(&output)
    );
    let stdout = stdout(&output);
    assert!(
        stdout.to_ascii_lowercase().contains("<html"),
        "stdout: {stdout}"
    );
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
