//! Integration tests for `clinrand verify`.

use std::fs;
use std::path::Path;
use std::process::Command;

use clinrand_core::{generate, Arm, BlockScheme, Method, NumberingScheme, StudyConfig};
use clinrand_package::{sha256_hex, write_package, PackageMeta};

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

fn demo_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-404".into(),
        protocol_version: "1.0".into(),
        arms: vec![
            Arm {
                code: "A".into(),
                label: "Active".into(),
                ratio: 1,
            },
            Arm {
                code: "P".into(),
                label: "Placebo".into(),
                ratio: 1,
            },
        ],
        method: Method::PermutedBlock {
            block: BlockScheme::Fixed { size: 2 },
        },
        strata: vec![],
        list_length_per_stratum: 2,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Verify CLI Test Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

fn write_demo_package(out: &Path) -> std::path::PathBuf {
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    write_package(out, &cfg, &list, &demo_seed(), &meta()).expect("write package")
}

fn update_checksum_line(checksums: &str, filename: &str, new_hex: &str) -> String {
    let lines: Vec<String> = checksums
        .lines()
        .map(|line| {
            if let Some((_, path)) = line.split_once("  ") {
                if path == filename {
                    format!("{new_hex}  {filename}")
                } else {
                    line.to_owned()
                }
            } else {
                line.to_owned()
            }
        })
        .collect();
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn verify_intact_package_exits_zero() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let output = clinrand()
        .arg("verify")
        .args(["--package"])
        .arg(&package_dir)
        .output()
        .expect("run verify");

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("verify ok"));
}

#[test]
fn verify_checksum_failure_exits_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let list_path = package_dir.join("list.csv");
    let mut bytes = fs::read(&list_path).expect("read list.csv");
    let idx = bytes.len().saturating_sub(2);
    bytes[idx] ^= 0x01;
    fs::write(&list_path, &bytes).expect("corrupt list.csv");

    let output = clinrand()
        .arg("verify")
        .args(["--package"])
        .arg(&package_dir)
        .output()
        .expect("run verify");

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let err = stderr(&output);
    assert!(err.contains("list.csv") || err.contains("checksum"));
}

#[test]
fn verify_property_failure_exits_one_with_updated_checksums() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let list_path = package_dir.join("list.csv");
    let csv = fs::read_to_string(&list_path).expect("read list.csv");
    let corrupted = csv.replace(",P\n", ",A\n");
    fs::write(&list_path, &corrupted).expect("write corrupted list.csv");

    let new_hash = sha256_hex(corrupted.as_bytes());
    let checksums_path = package_dir.join("checksums.txt");
    let checksums = fs::read_to_string(&checksums_path).expect("read checksums");
    fs::write(
        &checksums_path,
        update_checksum_line(&checksums, "list.csv", &new_hash),
    )
    .expect("write checksums");

    let output = clinrand()
        .arg("verify")
        .args(["--package"])
        .arg(&package_dir)
        .output()
        .expect("run verify");

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let err = stderr(&output);
    assert!(err.contains("P03") || err.contains("property"));
}

#[test]
fn verify_missing_package_exits_three() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("no-such-package");

    let output = clinrand()
        .arg("verify")
        .args(["--package"])
        .arg(&missing)
        .output()
        .expect("run verify");

    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
}

#[test]
fn verify_never_prints_seed_on_stdout_or_stderr() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());
    let seed_hex: String = demo_seed().iter().map(|b| format!("{b:02x}")).collect();

    let output = clinrand()
        .arg("verify")
        .args(["--package"])
        .arg(&package_dir)
        .output()
        .expect("run verify");

    assert!(!stdout(&output).contains(&seed_hex));
    assert!(!stderr(&output).contains(&seed_hex));
}
