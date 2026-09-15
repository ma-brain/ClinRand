//! `write_package` filesystem layout and `checksums.txt`.
//!
//! Fixtures use synthetic DEMO study IDs only. The seed appears on disk only
//! inside `manifest.unblinded.json`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clinrand_core::{
    AllocationRecord, Arm, BlockScheme, DrawPurpose, GeneratedList, Method, NumberingScheme,
    StreamDraw, StreamLog, StudyConfig,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use clinrand_package::{
    build_manifests, compact_generated_at, render_list_csv, sha256_hex, write_package, PackageMeta,
};

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

fn demo_list() -> GeneratedList {
    GeneratedList {
        records: vec![
            AllocationRecord {
                randomization_number: "10001".into(),
                stratum: BTreeMap::new(),
                block_id: 1,
                block_size: 2,
                position_in_block: 1,
                arm_code: "A".into(),
            },
            AllocationRecord {
                randomization_number: "10002".into(),
                stratum: BTreeMap::new(),
                block_id: 1,
                block_size: 2,
                position_in_block: 2,
                arm_code: "P".into(),
            },
        ],
        stream: StreamLog {
            draws: vec![StreamDraw {
                index: 0,
                bound: 2,
                value: 1,
                purpose: DrawPurpose::Permutation,
            }],
        },
    }
}

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Demo Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

/// Fixed synthetic seed for package write tests. Not a production secret.
fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn expected_filenames() -> [&'static str; 8] {
    [
        "checksums.txt",
        "generation-report.html",
        "list.csv",
        "list.json",
        "manifest.blinded.json",
        "manifest.unblinded.json",
        "stream.csv",
        "unblinded-report.html",
    ]
}

#[test]
fn write_package_creates_expected_filenames_and_dir_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let list = demo_list();
    let seed = demo_seed();

    let package_dir = write_package(tmp.path(), &cfg, &list, &seed, &meta()).expect("write");

    let list_csv = render_list_csv(&cfg, &list).expect("list.csv");
    let list_prefix = &sha256_hex(list_csv.as_bytes())[..8];
    let expected_name = format!(
        "{}_{}_{}",
        cfg.study_id,
        compact_generated_at(&meta().generated_at),
        list_prefix
    );
    assert_eq!(
        package_dir.file_name().and_then(|s| s.to_str()),
        Some(expected_name.as_str())
    );
    assert_eq!(package_dir.parent(), Some(tmp.path()));

    let mut names: Vec<String> = fs::read_dir(&package_dir)
        .expect("read_dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, expected_filenames());

    // qc.R is Phase 6; HTML reports are present (Task 5).
    assert!(!package_dir.join("qc.R").exists());
    assert!(package_dir.join("generation-report.html").is_file());
    assert!(package_dir.join("unblinded-report.html").is_file());
}

#[test]
fn checksums_txt_matches_recomputed_file_hashes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir =
        write_package(tmp.path(), &demo_cfg(), &demo_list(), &demo_seed(), &meta()).expect("write");

    let checksums = fs::read_to_string(package_dir.join("checksums.txt")).expect("checksums");
    assert!(checksums.ends_with('\n'));

    let mut seen = Vec::new();
    for line in checksums.lines() {
        let (hex, name) = line
            .split_once("  ")
            .expect("GNU sha256sum text mode: `hex  filename`");
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "digest must be lowercase hex"
        );
        assert_ne!(name, "checksums.txt");
        seen.push(name.to_owned());

        let bytes = fs::read(package_dir.join(name)).expect("read file");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let recomputed: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hex, recomputed, "mismatch for {name}");
    }

    let mut expected: Vec<String> = expected_filenames()
        .into_iter()
        .filter(|n| *n != "checksums.txt")
        .map(str::to_owned)
        .collect();
    expected.sort();
    assert_eq!(seen, expected, "checksum lines must be sorted by filename");
}

#[test]
fn blinded_manifest_on_disk_has_no_seed_hex() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let list = demo_list();
    let seed = demo_seed();
    let package_dir = write_package(tmp.path(), &cfg, &list, &seed, &meta()).expect("write");

    let blinded = fs::read_to_string(package_dir.join("manifest.blinded.json")).expect("blinded");
    let unblinded =
        fs::read_to_string(package_dir.join("manifest.unblinded.json")).expect("unblinded");

    let manifests = build_manifests(&cfg, &list, &seed, &meta()).expect("manifests");
    // Write path must use exact ManifestPair bytes — no second serializer.
    assert_eq!(blinded, manifests.blinded);
    assert_eq!(unblinded, manifests.unblinded);

    let blinded_v: Value = serde_json::from_str(blinded.trim_end_matches('\n')).expect("json");
    assert!(
        blinded_v.get("seed_hex").is_none(),
        "blinded manifest on disk must omit seed_hex"
    );
    assert!(
        !blinded.contains("seed_hex"),
        "blinded file text must not mention seed_hex"
    );

    let unblinded_v: Value = serde_json::from_str(unblinded.trim_end_matches('\n')).expect("json");
    assert!(unblinded_v.get("seed_hex").is_some());
}

#[test]
fn list_csv_on_disk_matches_renderer_bytes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let list = demo_list();
    let package_dir = write_package(tmp.path(), &cfg, &list, &demo_seed(), &meta()).expect("write");

    let on_disk = fs::read(package_dir.join("list.csv")).expect("list.csv");
    let expected = render_list_csv(&cfg, &list).expect("render");
    assert_eq!(on_disk, expected.as_bytes());
}

#[test]
fn package_error_io_display_has_no_seed() {
    // Force an I/O failure: out_dir is a file, not a directory.
    let tmp = tempfile::tempdir().expect("tempdir");
    let blocker = tmp.path().join("not-a-dir");
    fs::write(&blocker, b"x").expect("blocker file");

    let err = write_package(
        Path::new(&blocker),
        &demo_cfg(),
        &demo_list(),
        &demo_seed(),
        &meta(),
    )
    .expect_err("must fail I/O");

    let msg = err.to_string();
    let seed = demo_seed();
    let seed_hex: String = seed.iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        !msg.contains(&seed_hex),
        "PackageError Display must not contain the seed"
    );
}
