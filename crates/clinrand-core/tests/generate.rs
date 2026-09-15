//! `generate` behaviour (plan §2.4, §5.3–5.6).
//!
//! Snapshots lock arm sequences and stream draw metadata for fixed seeds.
//! They are stability checks within this crate, not validation/reference
//! evidence (those tiers require external or hand-derived norms).

use std::collections::BTreeMap;

use clinrand_core::{
    generate, generate_with_options, Arm, BlockScheme, ConfigError, DrawPurpose, GenerationError,
    Method, NumberingScheme, StratificationFactor, StudyConfig, ValidateOptions, ALGO_VERSION,
};

fn seed_a() -> [u8; 32] {
    let mut s = [0u8; 32];
    s[0] = 0xA5;
    s[31] = 0x5A;
    s
}

fn arms_1_1() -> Vec<Arm> {
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

fn arms_2_1() -> Vec<Arm> {
    vec![
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
    ]
}

fn simple_cfg(length: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-301".into(),
        protocol_version: "1.0".into(),
        arms: arms_1_1(),
        method: Method::Simple,
        strata: vec![],
        list_length_per_stratum: length,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

fn permuted_fixed_cfg(length: u32, size: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "TEST-301".into(),
        protocol_version: "1.0".into(),
        arms: arms_1_1(),
        method: Method::PermutedBlock {
            block: BlockScheme::Fixed { size },
        },
        strata: vec![],
        list_length_per_stratum: length,
        numbering: NumberingScheme::Global { start: 1, width: 4 },
    }
}

fn stratified_variable_cfg(length: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "EXAMPLE-301".into(),
        protocol_version: "1.0".into(),
        arms: arms_2_1(),
        method: Method::StratifiedBlock {
            block: BlockScheme::Variable {
                // Unsorted on purpose: indexing must use sorted ascending deduped copy.
                sizes: vec![9, 6],
            },
        },
        strata: vec![StratificationFactor {
            name: "site".into(),
            levels: vec!["001".into(), "002".into()],
        }],
        list_length_per_stratum: length,
        numbering: NumberingScheme::PerStratumRange {
            start: 20001,
            block_size: 40,
            width: 5,
        },
    }
}

fn arm_codes(list: &clinrand_core::GeneratedList) -> Vec<&str> {
    list.records.iter().map(|r| r.arm_code.as_str()).collect()
}

fn stream_meta(list: &clinrand_core::GeneratedList) -> Vec<(DrawPurpose, u64, u64)> {
    list.stream
        .draws
        .iter()
        .map(|d| (d.purpose, d.bound, d.value))
        .collect()
}

#[test]
fn algo_version_unchanged() {
    assert_eq!(ALGO_VERSION, 1);
}

#[test]
fn generate_rejects_invalid_config() {
    let mut cfg = simple_cfg(8);
    cfg.arms = vec![Arm {
        code: "A".into(),
        label: "Only".into(),
        ratio: 1,
    }];
    let err = generate(&cfg, seed_a()).expect_err("too few arms must fail");
    match err {
        GenerationError::InvalidConfig(errors) => {
            assert!(
                errors.contains(&ConfigError::TooFewArms),
                "expected TooFewArms among {errors:?}"
            );
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn same_seed_twice_identical_records_and_stream() {
    let cfg = stratified_variable_cfg(12);
    let a = generate(&cfg, seed_a()).expect("generate a");
    let b = generate(&cfg, seed_a()).expect("generate b");
    assert_eq!(a.records, b.records);
    assert_eq!(a.stream, b.stream);
}

#[test]
fn simple_snapshot_arms_and_stream() {
    let cfg = simple_cfg(8);
    let list = generate(&cfg, seed_a()).expect("simple generate");

    assert_eq!(list.records.len(), 8);
    for (i, rec) in list.records.iter().enumerate() {
        assert_eq!(rec.block_id, u32::try_from(i + 1).unwrap());
        assert_eq!(rec.block_size, 1);
        assert_eq!(rec.position_in_block, 1);
        assert!(rec.stratum.is_empty());
        assert_eq!(
            rec.randomization_number,
            format!("{:05}", 10001 + u32::try_from(i).unwrap())
        );
    }

    assert_eq!(
        arm_codes(&list),
        vec!["A", "A", "A", "A", "P", "P", "A", "A"]
    );
    assert_eq!(
        stream_meta(&list),
        vec![
            (DrawPurpose::SimpleAllocation, 2, 0),
            (DrawPurpose::SimpleAllocation, 2, 0),
            (DrawPurpose::SimpleAllocation, 2, 0),
            (DrawPurpose::SimpleAllocation, 2, 0),
            (DrawPurpose::SimpleAllocation, 2, 1),
            (DrawPurpose::SimpleAllocation, 2, 1),
            (DrawPurpose::SimpleAllocation, 2, 0),
            (DrawPurpose::SimpleAllocation, 2, 0),
        ]
    );
}

#[test]
fn permuted_fixed_snapshot_arms_and_stream() {
    let cfg = permuted_fixed_cfg(8, 4);
    let list = generate(&cfg, seed_a()).expect("permuted generate");

    assert_eq!(list.records.len(), 8);
    assert_eq!(
        arm_codes(&list),
        vec!["P", "P", "A", "A", "P", "P", "A", "A"]
    );
    // Two blocks of size 4: each permute draws bounds 4,3,2 (no BlockSize draw).
    assert_eq!(
        stream_meta(&list),
        vec![
            (DrawPurpose::Permutation, 4, 0),
            (DrawPurpose::Permutation, 3, 1),
            (DrawPurpose::Permutation, 2, 0),
            (DrawPurpose::Permutation, 4, 0),
            (DrawPurpose::Permutation, 3, 1),
            (DrawPurpose::Permutation, 2, 1),
        ]
    );
    assert_eq!(list.records[0].block_id, 1);
    assert_eq!(list.records[0].block_size, 4);
    assert_eq!(list.records[0].position_in_block, 1);
    assert_eq!(list.records[3].position_in_block, 4);
    assert_eq!(list.records[4].block_id, 2);
    assert_eq!(list.records[4].position_in_block, 1);
    assert_eq!(list.records[0].randomization_number, "0001");
    assert_eq!(list.records[7].randomization_number, "0008");
}

#[test]
fn stratified_variable_snapshot_uses_sorted_sizes() {
    let cfg = stratified_variable_cfg(6);
    let list = generate(&cfg, seed_a()).expect("stratified generate");

    // Two strata × 6 allocations.
    assert_eq!(list.records.len(), 12);
    assert_eq!(
        list.records[0].stratum,
        BTreeMap::from([("site".into(), "001".into())])
    );
    assert_eq!(
        list.records[6].stratum,
        BTreeMap::from([("site".into(), "002".into())])
    );
    // Per-stratum numbering: stratum 0 → 20001.., stratum 1 → 20041..
    assert_eq!(list.records[0].randomization_number, "20001");
    assert_eq!(list.records[5].randomization_number, "20006");
    assert_eq!(list.records[6].randomization_number, "20041");
    assert_eq!(list.records[11].randomization_number, "20046");

    // First draw per stratum is BlockSize with bound 2 (sorted [6,9]).
    assert_eq!(list.stream.draws[0].purpose, DrawPurpose::BlockSize);
    assert_eq!(list.stream.draws[0].bound, 2);

    assert_eq!(
        arm_codes(&list),
        vec![
            "A", "A", "A", "A", "P", "P", // site 001 (BlockSize→6)
            "A", "P", "A", "A", "A", "P", // site 002 (BlockSize→6)
        ]
    );
    assert_eq!(
        stream_meta(&list),
        vec![
            (DrawPurpose::BlockSize, 2, 0),
            (DrawPurpose::Permutation, 6, 4),
            (DrawPurpose::Permutation, 5, 4),
            (DrawPurpose::Permutation, 4, 0),
            (DrawPurpose::Permutation, 3, 1),
            (DrawPurpose::Permutation, 2, 1),
            (DrawPurpose::BlockSize, 2, 0),
            (DrawPurpose::Permutation, 6, 4),
            (DrawPurpose::Permutation, 5, 1),
            (DrawPurpose::Permutation, 4, 2),
            (DrawPurpose::Permutation, 3, 0),
            (DrawPurpose::Permutation, 2, 1),
        ]
    );
}

#[test]
fn truncation_consumes_full_block_stream_then_keeps_prefix() {
    let truncated = permuted_fixed_cfg(5, 4);
    let full = permuted_fixed_cfg(8, 4);
    let t = generate(&truncated, seed_a()).expect("truncated");
    let f = generate(&full, seed_a()).expect("full");

    assert_eq!(t.records.len(), 5);
    assert_eq!(f.records.len(), 8);
    // Truncated run still permutes two full blocks (same stream draws as length 8).
    assert_eq!(t.stream.draws.len(), f.stream.draws.len());
    assert_eq!(stream_meta(&t), stream_meta(&f));
    // Kept prefix matches the untruncated sibling.
    assert_eq!(arm_codes(&t), arm_codes(&f)[..5]);
    assert_eq!(t.records[4].block_id, 2);
    assert_eq!(t.records[4].block_size, 4);
    assert_eq!(t.records[4].position_in_block, 1);
}

#[test]
fn numbering_emits_full_decimal_when_wider_than_width() {
    let mut cfg = simple_cfg(2);
    cfg.numbering = NumberingScheme::Global {
        start: 9998,
        width: 3,
    };
    let list = generate(&cfg, seed_a()).expect("generate");
    assert_eq!(list.records[0].randomization_number, "9998");
    assert_eq!(list.records[1].randomization_number, "9999");
}

#[test]
fn numbering_zero_pads_when_within_width() {
    let mut cfg = simple_cfg(1);
    cfg.numbering = NumberingScheme::Global {
        start: 42,
        width: 5,
    };
    let list = generate(&cfg, seed_a()).expect("generate");
    assert_eq!(list.records[0].randomization_number, "00042");
}

fn level_names(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("L{i:03}")).collect()
}

fn large_strata_cfg(level_count: usize) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "TEST-201".into(),
        protocol_version: "2.1".into(),
        arms: arms_2_1(),
        method: Method::StratifiedBlock {
            block: BlockScheme::Variable { sizes: vec![6, 9] },
        },
        strata: vec![StratificationFactor {
            name: "site".into(),
            levels: level_names(level_count),
        }],
        list_length_per_stratum: 6,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

#[test]
fn generate_rejects_more_than_200_stratum_combinations() {
    let cfg = large_strata_cfg(201);
    let err = generate(&cfg, seed_a()).expect_err("default generate must reject >200 strata");
    match err {
        GenerationError::InvalidConfig(errors) => {
            assert!(
                errors
                    .iter()
                    .any(|e| matches!(e, ConfigError::TooManyStrata { count: 201 })),
                "expected TooManyStrata among {errors:?}"
            );
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn generate_with_options_accepts_large_strata_when_allowed() {
    let cfg = large_strata_cfg(201);
    let list = generate_with_options(
        &cfg,
        seed_a(),
        &ValidateOptions {
            allow_large_strata: true,
        },
    )
    .expect("allow_large_strata should accept 201 combinations");
    assert_eq!(list.records.len(), 201 * 6);
}

#[test]
fn generate_matches_generate_with_options_default() {
    let cfg = stratified_variable_cfg(12);
    let default_list = generate(&cfg, seed_a()).expect("generate");
    let options_list = generate_with_options(
        &cfg,
        seed_a(),
        &ValidateOptions {
            allow_large_strata: false,
        },
    )
    .expect("generate_with_options with defaults");
    assert_eq!(default_list, options_list);
}

#[test]
fn one_bit_seed_change_changes_list() {
    let cfg = simple_cfg(16);
    let mut seed_b = seed_a();
    seed_b[0] ^= 1;
    let a = generate(&cfg, seed_a()).expect("a");
    let b = generate(&cfg, seed_b).expect("b");
    assert_ne!(arm_codes(&a), arm_codes(&b));
}
