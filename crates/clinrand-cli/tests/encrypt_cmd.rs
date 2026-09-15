//! Integration tests for `clinrand generate --encrypt` and `clinrand decrypt`.
//!
//! The passphrase prompt reads from `/dev/tty` only when stdin is an
//! interactive terminal; a piped stdin (as in these subprocess tests) takes
//! the plain-stdin fallback path (see `src/passphrase.rs`), so tests drive
//! it exactly the way a script would.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_path(name: &str) -> PathBuf {
    repo_root().join("examples").join(name)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Run `cmd`, feeding `stdin_text` on stdin (the non-interactive passphrase
/// path) and capturing stdout/stderr.
fn run_with_stdin(mut cmd: Command, stdin_text: &str) -> Output {
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn clinrand");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin_text.as_bytes())
        .expect("write passphrase to stdin");
    child.wait_with_output().expect("wait for clinrand")
}

const RESTRICTED_FILES: &[&str] = &[
    "list.csv",
    "list.json",
    "manifest.unblinded.json",
    "stream.csv",
    "unblinded-report.html",
];

fn generate_encrypted(out_dir: &std::path::Path, passphrase: &str) -> Output {
    let mut cmd = clinrand();
    cmd.arg("generate")
        .args(["--config"])
        .arg(example_path("simple.json"))
        .args(["--out"])
        .arg(out_dir)
        .args(["--operator", "Encrypt CLI Test"])
        .arg("--encrypt");
    // First prompt (passphrase) then confirmation, one per line.
    run_with_stdin(cmd, &format!("{passphrase}\n{passphrase}\n"))
}

#[test]
fn generate_encrypt_writes_restricted_age_and_no_plaintext_restricted_files() {
    let out = tempfile::tempdir().expect("tempdir");
    let output = generate_encrypted(out.path(), "cli test passphrase");
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let package_dir = stdout(&output)
        .lines()
        .next()
        .expect("stdout line")
        .trim()
        .to_owned();
    let package_path = PathBuf::from(&package_dir);
    assert!(package_path.join("restricted.age").is_file());
    assert!(package_path.join("manifest.blinded.json").is_file());
    assert!(package_path.join("generation-report.html").is_file());
    assert!(package_path.join("qc.R").is_file());

    for name in RESTRICTED_FILES {
        assert!(
            !package_path.join(name).exists(),
            "{name} must not exist as plaintext in an encrypted package"
        );
    }
}

#[test]
fn generate_encrypt_mismatched_confirmation_exits_five_and_writes_nothing() {
    let out = tempfile::tempdir().expect("tempdir");
    let mut cmd = clinrand();
    cmd.arg("generate")
        .args(["--config"])
        .arg(example_path("simple.json"))
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Encrypt CLI Test"])
        .arg("--encrypt");
    let output = run_with_stdin(cmd, "first passphrase\nsecond different passphrase\n");

    assert_eq!(output.status.code(), Some(5), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("did not match"));
    assert_eq!(
        std::fs::read_dir(out.path()).expect("read out dir").count(),
        0,
        "a rejected confirmation must not create any package directory"
    );
}

#[test]
fn generate_encrypt_empty_passphrase_exits_five() {
    let out = tempfile::tempdir().expect("tempdir");
    let mut cmd = clinrand();
    cmd.arg("generate")
        .args(["--config"])
        .arg(example_path("simple.json"))
        .args(["--out"])
        .arg(out.path())
        .args(["--operator", "Encrypt CLI Test"])
        .arg("--encrypt");
    let output = run_with_stdin(cmd, "\n\n");

    assert_eq!(output.status.code(), Some(5), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("empty"));
}

#[test]
fn decrypt_round_trip_then_verify_checks_properties() {
    let out = tempfile::tempdir().expect("tempdir");
    let passphrase = "round trip passphrase";
    let generated = generate_encrypted(out.path(), passphrase);
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&generated)
    );
    let package_dir = stdout(&generated)
        .lines()
        .next()
        .expect("stdout line")
        .trim()
        .to_owned();

    let mut decrypt_cmd = clinrand();
    decrypt_cmd.arg("decrypt").args(["--package", &package_dir]);
    let decrypted = run_with_stdin(decrypt_cmd, &format!("{passphrase}\n"));
    assert_eq!(
        decrypted.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&decrypted)
    );

    for name in RESTRICTED_FILES {
        assert!(
            PathBuf::from(&package_dir).join(name).is_file(),
            "missing {name} after decrypt"
        );
    }

    let verify_output = clinrand()
        .arg("verify")
        .args(["--package", &package_dir])
        .output()
        .expect("run verify");
    assert_eq!(
        verify_output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&verify_output)
    );
    assert_eq!(stdout(&verify_output), "verify ok\n");
}

#[test]
fn decrypt_wrong_passphrase_exits_five_and_writes_nothing() {
    let out = tempfile::tempdir().expect("tempdir");
    let generated = generate_encrypted(out.path(), "the real passphrase");
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&generated)
    );
    let package_dir = stdout(&generated)
        .lines()
        .next()
        .expect("stdout line")
        .trim()
        .to_owned();

    let mut decrypt_cmd = clinrand();
    decrypt_cmd.arg("decrypt").args(["--package", &package_dir]);
    let output = run_with_stdin(decrypt_cmd, "definitely wrong\n");

    assert_eq!(output.status.code(), Some(5), "stderr: {}", stderr(&output));
    for name in RESTRICTED_FILES {
        assert!(
            !PathBuf::from(&package_dir).join(name).exists(),
            "a failed decrypt must not leave {name} on disk"
        );
    }
}
