//! Integration tests for `verify_package`.

use std::fs;

use clinrand_core::{generate, Arm, BlockScheme, Method, NumberingScheme, StudyConfig};
use clinrand_package::{sha256_hex, verify_package, write_package, PackageMeta};

fn arms_ap() -> Vec<Arm> {
    vec![
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
    ]
}

fn demo_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-404".into(),
        protocol_version: "1.0".into(),
        arms: arms_ap(),
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

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Verify Test Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn write_demo_package(out: &std::path::Path) -> std::path::PathBuf {
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    write_package(out, &cfg, &list, &demo_seed(), &meta()).expect("write package")
}

fn update_checksum_line(checksums: &str, filename: &str, new_hex: &str) -> String {
    let mut lines: Vec<String> = checksums
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
    lines.sort();
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[test]
fn verify_intact_package_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let report = verify_package(&package_dir).expect("verify");
    assert!(report.ok(), "report: {report:?}");
    assert!(report.checksums_ok);
    assert!(report.properties_ok);
    assert!(report.checksum_failures.is_empty());
    assert!(report.property_failures.is_empty());
}

#[test]
fn verify_detects_checksum_mismatch_when_list_csv_byte_flipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let list_path = package_dir.join("list.csv");
    let mut bytes = fs::read(&list_path).expect("read list.csv");
    let idx = bytes.len().saturating_sub(2);
    bytes[idx] ^= 0x01;
    fs::write(&list_path, &bytes).expect("corrupt list.csv");

    let report = verify_package(&package_dir).expect("verify");
    assert!(!report.ok());
    assert!(!report.checksums_ok);
    assert_eq!(report.checksum_failures.len(), 1);
    assert!(
        report.checksum_failures[0].contains("list.csv"),
        "failure: {}",
        report.checksum_failures[0]
    );
}

#[test]
fn verify_detects_property_failure_when_arms_corrupted_but_checksums_updated() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_demo_package(tmp.path());

    let list_path = package_dir.join("list.csv");
    let csv = fs::read_to_string(&list_path).expect("read list.csv");
    let corrupted = csv.replace(",P\n", ",A\n");
    assert_ne!(csv, corrupted, "fixture must change arm codes");
    fs::write(&list_path, &corrupted).expect("write corrupted list.csv");

    let new_hash = sha256_hex(corrupted.as_bytes());
    let checksums_path = package_dir.join("checksums.txt");
    let checksums = fs::read_to_string(&checksums_path).expect("read checksums");
    let updated = update_checksum_line(&checksums, "list.csv", &new_hash);
    fs::write(&checksums_path, updated).expect("write checksums");

    let report = verify_package(&package_dir).expect("verify");
    assert!(!report.ok());
    assert!(report.checksums_ok);
    assert!(!report.properties_ok);
    assert!(
        report
            .property_failures
            .iter()
            .any(|f| f.starts_with("P03:")),
        "expected P03 failure, got {:?}",
        report.property_failures
    );
}

#[test]
fn verify_module_does_not_call_generate() {
    let src = include_str!("../src/verify.rs");
    assert!(
        !src.contains("generate(") && !src.contains("generate_with_options"),
        "verify must not regenerate the list"
    );
}
