//! Integration tests for `clinrand reproduce`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clinrand_core::{generate, ALGO_VERSION};
use clinrand_package::{parse_unblinded_manifest, seed_hex, write_package, PackageMeta};

fn clinrand() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clinrand"))
}

fn demo_cfg() -> clinrand_core::StudyConfig {
    serde_json::from_str(include_str!("../../../examples/simple.json")).expect("parse example")
}

fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Reproduce Test Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

fn write_known_package(out: &Path) -> (PathBuf, String) {
    let cfg = demo_cfg();
    let seed = demo_seed();
    let list = generate(&cfg, seed).expect("generate");
    let package_dir = write_package(out, &cfg, &list, &seed, &meta()).expect("write package");
    let manifest_path = package_dir.join("manifest.unblinded.json");
    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest = parse_unblinded_manifest(&manifest_text).expect("parse manifest");
    (manifest_path, manifest.list_sha256)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn package_subdirs(out: &Path) -> Vec<PathBuf> {
    fs::read_dir(out)
        .expect("read out dir")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().ok()?.is_dir().then_some(entry.path())
        })
        .collect()
}

#[test]
fn reproduce_from_manifest_writes_matching_package_without_leaking_seed() {
    let source = tempfile::tempdir().expect("tempdir");
    let (manifest_path, expected_list_sha256) = write_known_package(source.path());

    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .arg("reproduce")
        .args(["--manifest"])
        .arg(&manifest_path)
        .args(["--out"])
        .arg(out.path())
        .output()
        .expect("run reproduce");

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let stdout_text = stdout(&output);
    let package_dir = stdout_text.lines().next().expect("stdout line").trim();
    let printed_hash = stdout_text.lines().nth(1).expect("list_sha256 line").trim();
    assert_eq!(printed_hash, expected_list_sha256);

    let reproduced_manifest =
        fs::read_to_string(PathBuf::from(package_dir).join("manifest.unblinded.json"))
            .expect("read reproduced manifest");
    let reproduced = parse_unblinded_manifest(&reproduced_manifest).expect("parse reproduced");
    assert_eq!(reproduced.list_sha256, expected_list_sha256);

    let seed_hex = seed_hex(&demo_seed());
    assert!(
        !stdout_text.contains(&seed_hex),
        "stdout must not contain seed_hex"
    );
    assert!(
        !stderr(&output).contains(&seed_hex),
        "stderr must not contain seed_hex"
    );
}

#[test]
fn reproduce_algo_version_mismatch_exits_four_without_writing_package() {
    let source = tempfile::tempdir().expect("tempdir");
    let (manifest_path, _) = write_known_package(source.path());

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text).expect("json");
    value["algo_version"] = serde_json::json!(999);
    let mutated_path = source.path().join("manifest.bad-algo.json");
    fs::write(
        &mutated_path,
        serde_json::to_string(&value).expect("serialize"),
    )
    .expect("write");

    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .arg("reproduce")
        .args(["--manifest"])
        .arg(&mutated_path)
        .args(["--out"])
        .arg(out.path())
        .output()
        .expect("run reproduce");

    assert_eq!(output.status.code(), Some(4), "stderr: {}", stderr(&output));

    let err = stderr(&output);
    assert!(
        err.contains("999"),
        "stderr should name manifest algo_version: {err}"
    );
    assert!(
        err.contains(&ALGO_VERSION.to_string()),
        "stderr should name binary algo_version: {err}"
    );
    assert!(
        package_subdirs(out.path()).is_empty(),
        "no package directory should be created under out"
    );

    let seed_hex = seed_hex(&demo_seed());
    assert!(!stdout(&output).contains(&seed_hex));
    assert!(!err.contains(&seed_hex));
}

#[test]
fn reproduce_list_sha256_mismatch_exits_one_without_writing_package() {
    let source = tempfile::tempdir().expect("tempdir");
    let (manifest_path, _) = write_known_package(source.path());

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text).expect("json");
    value["seed_hex"] =
        serde_json::json!("0000000000000000000000000000000000000000000000000000000000000001");
    let mutated_path = source.path().join("manifest.bad-seed.json");
    fs::write(
        &mutated_path,
        serde_json::to_string(&value).expect("serialize"),
    )
    .expect("write");

    let out = tempfile::tempdir().expect("tempdir");
    let output = clinrand()
        .arg("reproduce")
        .args(["--manifest"])
        .arg(&mutated_path)
        .args(["--out"])
        .arg(out.path())
        .output()
        .expect("run reproduce");

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        package_subdirs(out.path()).is_empty(),
        "no package directory should be created under out"
    );

    let seed_hex = seed_hex(&demo_seed());
    let alt_seed_hex = "0000000000000000000000000000000000000000000000000000000000000001";
    assert!(!stdout(&output).contains(&seed_hex));
    assert!(!stdout(&output).contains(alt_seed_hex));
    assert!(!stderr(&output).contains(&seed_hex));
    assert!(!stderr(&output).contains(alt_seed_hex));
}
