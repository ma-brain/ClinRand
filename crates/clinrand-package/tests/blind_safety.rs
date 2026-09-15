//! Blind-safety of `generation-report.html` (plan §6.5 / AGENTS.md §8).
//!
//! For every randomization number in the list, no line of the blinded HTML
//! may contain both that number and any arm code. The seed hex must not appear.

use std::collections::BTreeMap;

use clinrand_core::{
    AllocationRecord, Arm, BlockScheme, DrawPurpose, GeneratedList, Method, NumberingScheme,
    StreamDraw, StreamLog, StudyConfig,
};

use clinrand_package::{render_generation_report, seed_hex, PackageMeta, ReportFileHashes};

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
        study_id: "DEMO-505".into(),
        protocol_version: "1.0".into(),
        arms: arms_ap(),
        method: Method::PermutedBlock {
            block: BlockScheme::Fixed { size: 2 },
        },
        strata: vec![],
        list_length_per_stratum: 4,
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
            AllocationRecord {
                randomization_number: "10003".into(),
                stratum: BTreeMap::new(),
                block_id: 2,
                block_size: 2,
                position_in_block: 1,
                arm_code: "P".into(),
            },
            AllocationRecord {
                randomization_number: "10004".into(),
                stratum: BTreeMap::new(),
                block_id: 2,
                block_size: 2,
                position_in_block: 2,
                arm_code: "A".into(),
            },
        ],
        stream: StreamLog {
            draws: vec![
                StreamDraw {
                    index: 0,
                    bound: 2,
                    value: 1,
                    purpose: DrawPurpose::Permutation,
                },
                StreamDraw {
                    index: 1,
                    bound: 2,
                    value: 0,
                    purpose: DrawPurpose::Permutation,
                },
            ],
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
    seed[0] = 0xab;
    seed[31] = 0xcd;
    seed
}

fn demo_hashes() -> ReportFileHashes {
    ReportFileHashes {
        list_sha256: "11".repeat(32),
        list_json_sha256: "22".repeat(32),
        stream_sha256: "33".repeat(32),
        manifest_blinded_sha256: "44".repeat(32),
        manifest_unblinded_sha256: "55".repeat(32),
        config_sha256: "66".repeat(32),
        seed_sha256: "77".repeat(32),
    }
}

#[test]
fn blinded_report_never_pairs_randomization_number_with_arm_code() {
    let cfg = demo_cfg();
    let list = demo_list();
    let html = render_generation_report(&cfg, &list, &meta(), &demo_hashes());

    let arm_codes: Vec<&str> = cfg.arms.iter().map(|a| a.code.as_str()).collect();

    for rec in &list.records {
        let rand_num = &rec.randomization_number;
        for (line_no, line) in html.lines().enumerate() {
            if !line.contains(rand_num.as_str()) {
                continue;
            }
            for arm in &arm_codes {
                assert!(
                    !line.contains(arm),
                    "blind-safety: line {} contains both randomization number {rand_num} and arm code {arm}:\n{line}",
                    line_no + 1
                );
            }
        }
    }
}

#[test]
fn blinded_report_does_not_contain_seed_hex() {
    let cfg = demo_cfg();
    let list = demo_list();
    let seed = demo_seed();
    let seed_hex_str = seed_hex(&seed);
    let html = render_generation_report(&cfg, &list, &meta(), &demo_hashes());

    assert!(
        !html.contains(&seed_hex_str),
        "blinded HTML must not contain the seed hex string"
    );
    assert!(
        !html.contains("seed_hex"),
        "blinded HTML must not mention seed_hex"
    );
}

#[test]
fn blinded_report_includes_required_sections() {
    let cfg = demo_cfg();
    let list = demo_list();
    let hashes = demo_hashes();
    let html = render_generation_report(&cfg, &list, &meta(), &hashes);

    assert!(html.contains("DEMO-505"));
    assert!(html.contains("Demo Operator"));
    assert!(html.contains("2026-09-15T14:42:10Z"));
    assert!(html.contains("arm_code=A"));
    assert!(html.contains("arm_code=P"));
    assert!(html.contains(&hashes.seed_sha256));
    assert!(html.contains(&hashes.list_sha256));
    assert!(html.contains("algo_version: 1"));
    assert!(html.contains("P01"));
    assert!(html.contains("P10"));
    // Blinded: no allocation table pairing.
    assert!(!html.contains("Allocations (unblinded)"));
}
