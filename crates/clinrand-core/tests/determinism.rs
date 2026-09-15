//! Determinism checks required by plan §8.2 / AGENTS.md §8.
//!
//! Complementary unit coverage also lives in `generate.rs` (snapshots).
//! This file names the contract tests explicitly for the property tier.

use clinrand_core::{
    generate, Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig,
};

fn seed_a() -> [u8; 32] {
    let mut s = [0u8; 32];
    s[0] = 0xA5;
    s[31] = 0x5A;
    s
}

fn stratified_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-502".into(),
        protocol_version: "1.0".into(),
        arms: vec![
            Arm {
                code: "A".into(),
                label: "Active".into(),
                ratio: 2,
            },
            Arm {
                code: "P".into(),
                label: "Placebo".into(),
                ratio: 1,
            },
        ],
        method: Method::StratifiedBlock {
            block: BlockScheme::Variable { sizes: vec![9, 6] },
        },
        strata: vec![StratificationFactor {
            name: "site".into(),
            levels: vec!["001".into(), "002".into()],
        }],
        list_length_per_stratum: 12,
        numbering: NumberingScheme::PerStratumRange {
            start: 20001,
            block_size: 40,
            width: 5,
        },
    }
}

fn simple_cfg(length: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "TEST-502".into(),
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
        method: Method::Simple,
        strata: vec![],
        list_length_per_stratum: length,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

#[test]
fn generate_twice_identical_records_and_stream() {
    let cfg = stratified_cfg();
    let a = generate(&cfg, seed_a()).expect("generate a");
    let b = generate(&cfg, seed_a()).expect("generate b");
    assert_eq!(a.records, b.records);
    assert_eq!(a.stream, b.stream);
}

#[test]
fn one_bit_seed_change_changes_list() {
    let cfg = simple_cfg(16);
    let mut seed_b = seed_a();
    seed_b[0] ^= 1;
    let a = generate(&cfg, seed_a()).expect("a");
    let b = generate(&cfg, seed_b).expect("b");
    let arms_differ = a
        .records
        .iter()
        .map(|r| r.arm_code.as_str())
        .ne(b.records.iter().map(|r| r.arm_code.as_str()));
    let numbers_differ = a
        .records
        .iter()
        .map(|r| r.randomization_number.as_str())
        .ne(b.records.iter().map(|r| r.randomization_number.as_str()));
    assert!(
        arms_differ || numbers_differ,
        "one-bit seed flip must change arm sequence or randomization numbers"
    );
}
