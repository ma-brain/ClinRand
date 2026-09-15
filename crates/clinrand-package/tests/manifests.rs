//! Manifest builders: blinded / unblinded JSON and content hashes.
//!
//! Fixtures use synthetic DEMO study IDs only. Seed bytes appear only where
//! the unblinded manifest contract requires `seed_hex`; error Display must
//! never include the seed.

use std::collections::BTreeMap;

use clinrand_core::{
    AllocationRecord, Arm, BlockScheme, DrawPurpose, GeneratedList, Method, NumberingScheme,
    StreamDraw, StreamLog, StudyConfig, ALGO_VERSION,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use clinrand_package::{
    build_manifests, config_sha256, render_list_csv, render_stream_csv, seed_hex, seed_sha256,
    sha256_hex, ManifestPair, PackageError, PackageMeta, MANIFEST_SCHEMA_VERSION,
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
        study_id: "DEMO-403".into(),
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

/// Fixed synthetic seed for hash checks. Not a production secret.
fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x01;
    seed[31] = 0xef;
    seed
}

fn parse_manifest(text: &str) -> Value {
    assert!(
        text.ends_with('\n') && !text.ends_with("\n\n"),
        "manifest file bytes must end with exactly one trailing LF"
    );
    serde_json::from_str(text.trim_end_matches('\n')).expect("manifest JSON")
}

#[test]
fn blinded_omits_seed_hex_unblinded_includes_it() {
    let cfg = demo_cfg();
    let list = demo_list();
    let seed = demo_seed();
    let ManifestPair { unblinded, blinded } =
        build_manifests(&cfg, &list, &seed, &meta()).expect("manifests");

    let unblinded_v = parse_manifest(&unblinded);
    let blinded_v = parse_manifest(&blinded);

    assert!(
        unblinded_v.get("seed_hex").is_some(),
        "unblinded must include seed_hex"
    );
    assert!(
        blinded_v.get("seed_hex").is_none(),
        "blinded must omit seed_hex key entirely"
    );
    assert!(!blinded.contains("seed_hex"), "key name must not appear");

    let expected_hex = seed_hex(&seed);
    assert_eq!(
        unblinded_v["seed_hex"].as_str().expect("seed_hex string"),
        expected_hex
    );
    assert_eq!(expected_hex.len(), 64);
    assert!(expected_hex
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn seed_sha256_is_digest_of_raw_seed_bytes() {
    let seed = demo_seed();
    let expected = {
        let digest = Sha256::digest(seed);
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(seed_sha256(&seed), expected);

    let ManifestPair { unblinded, blinded } =
        build_manifests(&demo_cfg(), &demo_list(), &seed, &meta()).expect("manifests");
    let u = parse_manifest(&unblinded);
    let b = parse_manifest(&blinded);
    assert_eq!(u["seed_sha256"], expected);
    assert_eq!(b["seed_sha256"], expected);
    // Must not be SHA-256 of the hex string.
    let of_hex = sha256_hex(seed_hex(&seed).as_bytes());
    assert_ne!(expected, of_hex);
}

#[test]
fn list_and_stream_sha256_match_render_bytes() {
    let cfg = demo_cfg();
    let list = demo_list();
    let list_csv = render_list_csv(&cfg, &list).expect("list.csv");
    let stream_csv = render_stream_csv(&list.stream).expect("stream.csv");
    let expected_list = sha256_hex(list_csv.as_bytes());
    let expected_stream = sha256_hex(stream_csv.as_bytes());

    let ManifestPair { unblinded, blinded } =
        build_manifests(&cfg, &list, &demo_seed(), &meta()).expect("manifests");
    for text in [&unblinded, &blinded] {
        let v = parse_manifest(text);
        assert_eq!(v["list_sha256"], expected_list);
        assert_eq!(v["stream_sha256"], expected_stream);
    }
}

#[test]
fn manifest_fields_match_contract() {
    let cfg = demo_cfg();
    let list = demo_list();
    let ManifestPair { unblinded, blinded } =
        build_manifests(&cfg, &list, &demo_seed(), &meta()).expect("manifests");

    for text in [&unblinded, &blinded] {
        let v = parse_manifest(text);
        assert_eq!(v["schema_version"], MANIFEST_SCHEMA_VERSION);
        assert_eq!(v["study_id"], "DEMO-403");
        assert_eq!(v["protocol_version"], "1.0");
        assert_eq!(v["generated_at"], "2026-09-15T14:42:10Z");
        assert_eq!(v["operator"], "Demo Operator");
        assert_eq!(v["algo_version"], ALGO_VERSION);
        assert_eq!(v["engine_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(v["record_count"], 2);
        assert_eq!(
            v["config_sha256"].as_str().expect("config_sha256"),
            config_sha256(&cfg).expect("config hash")
        );
        assert_eq!(v["rng"]["algorithm"], "ChaCha20");
        assert_eq!(v["rng"]["crate"], "rand_chacha");
        assert_eq!(v["rng"]["crate_version"], "0.3.1");
        assert!(v.get("config").is_some());
    }

    // Same inputs → byte-identical manifests.
    let again = build_manifests(&cfg, &list, &demo_seed(), &meta()).expect("repeat");
    assert_eq!(unblinded.as_bytes(), again.unblinded.as_bytes());
    assert_eq!(blinded.as_bytes(), again.blinded.as_bytes());
}

#[test]
fn package_error_display_never_contains_seed() {
    let err = PackageError::MissingStratumFactor {
        factor: "site".into(),
        randomization_number: "10001".into(),
    };
    let display = err.to_string();
    assert!(!display.contains("seed"));
    // Synthetic seed hex must not appear even if someone confused fields.
    assert!(!display.contains(&seed_hex(&demo_seed())));
}
