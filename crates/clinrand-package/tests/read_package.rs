//! Package read helpers: list.csv, checksums.txt, unblinded manifest.

use std::fs;

use clinrand_core::{
    generate, Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig,
};
use clinrand_package::{
    parse_checksums_txt, parse_list_csv, parse_unblinded_manifest, render_list_csv, seed_hex,
    write_package, PackageError, PackageMeta,
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

fn stratified_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-201".into(),
        protocol_version: "2.1".into(),
        arms: vec![
            Arm {
                code: "A".into(),
                label: "Investigational product 50 mg".into(),
                ratio: 2,
            },
            Arm {
                code: "P".into(),
                label: "Placebo".into(),
                ratio: 1,
            },
        ],
        method: Method::StratifiedBlock {
            block: BlockScheme::Variable { sizes: vec![6, 9] },
        },
        strata: vec![
            StratificationFactor {
                name: "site".into(),
                levels: vec!["001".into(), "002".into()],
            },
            StratificationFactor {
                name: "agegrp".into(),
                levels: vec!["LT65".into(), "GE65".into()],
            },
        ],
        list_length_per_stratum: 6,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Demo Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn assert_display_has_no_seed(err: &PackageError, seed: &[u8; 32]) {
    let msg = err.to_string();
    let hex = seed_hex(seed);
    assert!(
        !msg.contains(&hex),
        "PackageError Display must not contain seed_hex, got: {msg}"
    );
    assert!(
        !msg.contains("01000000000000000000000000000000000000000000000000000000000000ef"),
        "PackageError Display must not contain seed bytes as hex"
    );
}

#[test]
fn parse_list_csv_round_trips_render_list_csv() {
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    let csv = render_list_csv(&cfg, &list).expect("render");
    let parsed = parse_list_csv(&cfg, &csv).expect("parse");
    assert_eq!(parsed, list.records);
}

#[test]
fn parse_list_csv_round_trips_write_package_on_disk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    let package_dir = write_package(tmp.path(), &cfg, &list, &demo_seed(), &meta()).expect("write");

    let csv = fs::read_to_string(package_dir.join("list.csv")).expect("list.csv");
    let parsed = parse_list_csv(&cfg, &csv).expect("parse");
    assert_eq!(parsed, list.records);
}

#[test]
fn parse_list_csv_respects_stratum_column_order() {
    let cfg = stratified_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    let csv = render_list_csv(&cfg, &list).expect("render");
    let parsed = parse_list_csv(&cfg, &csv).expect("parse");
    assert_eq!(parsed, list.records);
    assert!(
        parsed
            .iter()
            .all(|r| r.stratum.contains_key("site") && r.stratum.contains_key("agegrp")),
        "stratum keys must be preserved"
    );
}

#[test]
fn parse_checksums_txt_from_write_package() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate");
    let package_dir = write_package(tmp.path(), &cfg, &list, &demo_seed(), &meta()).expect("write");

    let text = fs::read_to_string(package_dir.join("checksums.txt")).expect("checksums");
    let map = parse_checksums_txt(&text).expect("parse");
    assert!(!map.contains_key("checksums.txt"));
    assert!(map.contains_key("list.csv"));
    for (name, hex) in &map {
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        let bytes = fs::read(package_dir.join(name)).expect("read file");
        let recomputed = clinrand_package::sha256_hex(&bytes);
        assert_eq!(hex, &recomputed, "digest mismatch for {name}");
    }
}

#[test]
fn parse_unblinded_manifest_extracts_reproduce_fields() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = demo_cfg();
    let seed = demo_seed();
    let list = generate(&cfg, seed).expect("generate");
    let package_dir = write_package(tmp.path(), &cfg, &list, &seed, &meta()).expect("write");

    let json = fs::read_to_string(package_dir.join("manifest.unblinded.json")).expect("manifest");
    let manifest = parse_unblinded_manifest(&json).expect("parse");

    assert_eq!(manifest.schema_version, "1.0");
    assert_eq!(manifest.study_id, cfg.study_id);
    assert_eq!(manifest.protocol_version, cfg.protocol_version);
    assert_eq!(manifest.generated_at, meta().generated_at);
    assert_eq!(manifest.operator, meta().operator);
    assert_eq!(manifest.config, cfg);
    assert_eq!(manifest.seed_hex, seed_hex(&seed));
    assert_eq!(manifest.algo_version, clinrand_core::ALGO_VERSION);
    assert_eq!(manifest.record_count, list.records.len());
    assert!(!manifest.list_sha256.is_empty());
    assert!(!manifest.stream_sha256.is_empty());
    assert!(!manifest.config_sha256.is_empty());
    assert!(!manifest.seed_sha256.is_empty());
    assert!(!manifest.engine_version.is_empty());
}

#[test]
fn parse_list_csv_rejects_unknown_column() {
    let cfg = demo_cfg();
    let csv = "randomization_number,extra,block_id,block_size,position_in_block,arm_code\n\
               10001,x,1,2,1,A\n";
    let err = parse_list_csv(&cfg, csv).expect_err("unknown column");
    assert_display_has_no_seed(&err, &demo_seed());
}

#[test]
fn parse_list_csv_rejects_bad_integer() {
    let cfg = demo_cfg();
    let csv = "randomization_number,block_id,block_size,position_in_block,arm_code\n\
               10001,not-a-u32,2,1,A\n";
    let err = parse_list_csv(&cfg, csv).expect_err("bad integer");
    assert_display_has_no_seed(&err, &demo_seed());
}

#[test]
fn parse_checksums_txt_rejects_bad_line_format() {
    let err = parse_checksums_txt("deadbeef  list.csv\n").expect_err("bad format");
    assert_display_has_no_seed(&err, &demo_seed());
}

#[test]
fn parse_unblinded_manifest_rejects_bad_seed_hex_length() {
    let cfg = demo_cfg();
    let config_json = serde_json::to_value(&cfg).expect("config json");
    let json = serde_json::json!({
        "schema_version": "1.0",
        "study_id": "DEMO-404",
        "protocol_version": "1.0",
        "generated_at": "2026-09-15T14:42:10Z",
        "operator": "Demo Operator",
        "config": config_json,
        "config_sha256": "aa".repeat(32),
        "seed_hex": "abcd",
        "seed_sha256": "bb".repeat(32),
        "algo_version": 1,
        "engine_version": "0.1.0",
        "record_count": 0,
        "list_sha256": "cc".repeat(32),
        "stream_sha256": "dd".repeat(32),
    });
    let json = serde_json::to_string(&json).expect("manifest json");
    let err = parse_unblinded_manifest(&json).expect_err("bad seed_hex");
    assert_display_has_no_seed(&err, &demo_seed());
    let msg = err.to_string();
    assert!(
        msg.contains("seed_hex") || msg.contains("64"),
        "error should mention seed_hex length without printing value, got: {msg}"
    );
    assert!(!msg.contains("abcd"), "must not print bad seed_hex value");
}

#[test]
fn parse_errors_display_never_contains_known_seed_fixture() {
    let seed = demo_seed();
    let known_hex = seed_hex(&seed);

    // Trigger manifest parse with malformed JSON containing the known seed — error must not echo it.
    let json = format!(r#"{{"seed_hex":"{known_hex}","schema_version":}}"#,);
    let err = parse_unblinded_manifest(&json).expect_err("malformed json");
    assert!(
        !err.to_string().contains(&known_hex),
        "parse error Display must not leak seed_hex from input"
    );

    // list.csv row count mismatch
    let cfg = demo_cfg();
    let csv = "randomization_number,block_id,block_size,position_in_block,arm_code\n";
    let err = parse_list_csv(&cfg, csv).expect_err("header only");
    assert_display_has_no_seed(&err, &seed);

    // checksums with uppercase hex
    let err = parse_checksums_txt(
        "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789  list.csv\n",
    )
    .expect_err("uppercase hex");
    assert_display_has_no_seed(&err, &seed);
}
