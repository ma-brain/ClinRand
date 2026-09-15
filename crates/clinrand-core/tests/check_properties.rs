//! Property checks P01–P10 (plan §7) via `check_properties`.

use std::collections::BTreeMap;

use clinrand_core::{
    check_properties, generate, AllocationRecord, Arm, BlockScheme, GeneratedList, Method,
    NumberingScheme, StratificationFactor, StreamLog, StudyConfig,
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

fn permuted_fixed_cfg(length: u32, size: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-401".into(),
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

fn stratified_cfg(length: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-402".into(),
        protocol_version: "1.0".into(),
        arms: arms_1_1(),
        method: Method::StratifiedBlock {
            block: BlockScheme::Fixed { size: 4 },
        },
        strata: vec![StratificationFactor {
            name: "site".into(),
            levels: vec!["001".into(), "002".into()],
        }],
        list_length_per_stratum: length,
        numbering: NumberingScheme::PerStratumRange {
            start: 1001,
            block_size: 20,
            width: 4,
        },
    }
}

fn simple_cfg(length: u32) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-403".into(),
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

fn rec(
    number: &str,
    stratum: BTreeMap<String, String>,
    block_id: u32,
    block_size: u32,
    position_in_block: u32,
    arm_code: &str,
) -> AllocationRecord {
    AllocationRecord {
        randomization_number: number.into(),
        stratum,
        block_id,
        block_size,
        position_in_block,
        arm_code: arm_code.into(),
    }
}

fn empty_stratum() -> BTreeMap<String, String> {
    BTreeMap::new()
}

fn site(level: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("site".into(), level.into());
    m
}

fn check_named(list: &GeneratedList, cfg: &StudyConfig, id: &str) -> (bool, String, bool) {
    let report = check_properties(list, cfg);
    let c = report
        .checks
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("missing check {id}"));
    (c.passed, c.detail.clone(), c.informational)
}

fn assert_check_fails(list: &GeneratedList, cfg: &StudyConfig, id: &str) {
    let (passed, detail, informational) = check_named(list, cfg, id);
    assert!(
        !informational,
        "{id} must be a required check, detail={detail}"
    );
    assert!(!passed, "{id} should fail, detail={detail}");
}

fn assert_check_passes(list: &GeneratedList, cfg: &StudyConfig, id: &str) {
    let (passed, detail, _) = check_named(list, cfg, id);
    assert!(passed, "{id} should pass, detail={detail}");
}

/// Valid 1:1 fixed block of size 4, length 4 — one complete block A/P balanced.
fn valid_balanced_block_list() -> (StudyConfig, GeneratedList) {
    let cfg = permuted_fixed_cfg(4, 4);
    let list = GeneratedList {
        records: vec![
            rec("0001", empty_stratum(), 1, 4, 1, "A"),
            rec("0002", empty_stratum(), 1, 4, 2, "P"),
            rec("0003", empty_stratum(), 1, 4, 3, "A"),
            rec("0004", empty_stratum(), 1, 4, 4, "P"),
        ],
        stream: StreamLog::default(),
    };
    (cfg, list)
}

#[test]
fn p01_fails_when_record_count_wrong() {
    let (cfg, mut list) = valid_balanced_block_list();
    list.records.pop();
    assert_check_fails(&list, &cfg, "P01");
}

#[test]
fn p02_fails_when_block_size_not_allowed() {
    let (cfg, mut list) = valid_balanced_block_list();
    for r in &mut list.records {
        r.block_size = 6;
    }
    assert_check_fails(&list, &cfg, "P02");
}

#[test]
fn p03_fails_when_complete_block_ratio_wrong() {
    let (cfg, mut list) = valid_balanced_block_list();
    // Complete block of size 4 with 1:1 must be 2A+2P; make it 4A.
    for r in &mut list.records {
        r.arm_code = "A".into();
    }
    assert_check_fails(&list, &cfg, "P03");
}

#[test]
fn p03_skips_truncated_final_block() {
    // Length 6, block 4 → block 1 complete (must be balanced), block 2 kept=2 (skipped for P03).
    let cfg = permuted_fixed_cfg(6, 4);
    let list = GeneratedList {
        records: vec![
            rec("0001", empty_stratum(), 1, 4, 1, "A"),
            rec("0002", empty_stratum(), 1, 4, 2, "P"),
            rec("0003", empty_stratum(), 1, 4, 3, "A"),
            rec("0004", empty_stratum(), 1, 4, 4, "P"),
            // Truncated final block: both A is fine for P03 (skipped).
            rec("0005", empty_stratum(), 2, 4, 1, "A"),
            rec("0006", empty_stratum(), 2, 4, 2, "A"),
        ],
        stream: StreamLog::default(),
    };
    assert_check_passes(&list, &cfg, "P03");
}

#[test]
fn p04_fails_on_duplicate_randomization_numbers() {
    let (cfg, mut list) = valid_balanced_block_list();
    list.records[3].randomization_number = "0001".into();
    assert_check_fails(&list, &cfg, "P04");
}

#[test]
fn p05_fails_when_numbers_not_contiguous_ascending() {
    let (cfg, mut list) = valid_balanced_block_list();
    list.records[2].randomization_number = "0009".into();
    assert_check_fails(&list, &cfg, "P05");
}

#[test]
fn p06_fails_when_stratum_combination_missing() {
    let cfg = stratified_cfg(4);
    // Only site 001 present; 002 missing.
    let list = GeneratedList {
        records: vec![
            rec("1001", site("001"), 1, 4, 1, "A"),
            rec("1002", site("001"), 1, 4, 2, "P"),
            rec("1003", site("001"), 1, 4, 3, "A"),
            rec("1004", site("001"), 1, 4, 4, "P"),
        ],
        stream: StreamLog::default(),
    };
    assert_check_fails(&list, &cfg, "P06");
}

#[test]
fn p07_fails_on_cross_stratum_contamination() {
    let cfg = stratified_cfg(4);
    let list = GeneratedList {
        records: vec![
            rec("1001", site("001"), 1, 4, 1, "A"),
            rec("1002", site("001"), 1, 4, 2, "P"),
            rec("1003", site("001"), 1, 4, 3, "A"),
            rec("1004", site("001"), 1, 4, 4, "P"),
            // site 002 range, but one record claims site 001 (contamination).
            rec("1021", site("002"), 1, 4, 1, "A"),
            rec("1022", site("001"), 1, 4, 2, "P"),
            rec("1023", site("002"), 1, 4, 3, "A"),
            rec("1024", site("002"), 1, 4, 4, "P"),
        ],
        stream: StreamLog::default(),
    };
    assert_check_fails(&list, &cfg, "P07");
}

#[test]
fn p08_fails_when_position_in_block_has_gaps() {
    let (cfg, mut list) = valid_balanced_block_list();
    list.records[2].position_in_block = 4; // positions 1,2,4,4 — gap at 3, duplicate 4
    list.records[3].position_in_block = 4;
    assert_check_fails(&list, &cfg, "P08");
}

#[test]
fn p09_fails_when_overall_ratio_outside_one_block_tolerance() {
    // Length 12, block 4, 1:1 → ideal 6A+6P; tolerance one block (4).
    // 12A+0P exceeds tolerance.
    let cfg = permuted_fixed_cfg(12, 4);
    let list = GeneratedList {
        records: (1..=12)
            .map(|i| {
                let block_id = (i - 1) / 4 + 1;
                let pos = (i - 1) % 4 + 1;
                rec(&format!("{i:04}"), empty_stratum(), block_id, 4, pos, "A")
            })
            .collect(),
        stream: StreamLog::default(),
    };
    assert_check_fails(&list, &cfg, "P09");
}

#[test]
fn p10_is_informational_and_never_fails() {
    let (cfg, mut list) = valid_balanced_block_list();
    // Long run of identical arms — still must not fail P10.
    for r in &mut list.records {
        r.arm_code = "A".into();
    }
    let (passed, detail, informational) = check_named(&list, &cfg, "P10");
    assert!(informational, "P10 must be informational");
    assert!(passed, "P10 must never fail, detail={detail}");
    assert!(
        detail.to_lowercase().contains("informational")
            || detail.to_lowercase().contains("not a failure"),
        "P10 detail must state informational nature: {detail}"
    );
}

#[test]
fn generated_list_passes_p01_through_p09() {
    let cfg = stratified_cfg(6);
    let list = generate(&cfg, seed_a()).expect("generate");
    let report = check_properties(&list, &cfg);
    assert!(
        report.all_required_passed(),
        "expected P01–P09 pass: {:?}",
        report
            .checks
            .iter()
            .filter(|c| !c.informational && !c.passed)
            .map(|c| format!("{}: {}", c.id, c.detail))
            .collect::<Vec<_>>()
    );
    let p10 = report
        .checks
        .iter()
        .find(|c| c.id == "P10")
        .expect("P10 present");
    assert!(p10.informational);
    assert!(p10.passed);

    // Simple also passes required checks (P03/P09 treat size-1 / no block balance specially).
    let simple = simple_cfg(8);
    let simple_list = generate(&simple, seed_a()).expect("generate simple");
    let simple_report = check_properties(&simple_list, &simple);
    assert!(
        simple_report.all_required_passed(),
        "simple must pass P01–P09: {:?}",
        simple_report
            .checks
            .iter()
            .filter(|c| !c.informational && !c.passed)
            .map(|c| format!("{}: {}", c.id, c.detail))
            .collect::<Vec<_>>()
    );
}
