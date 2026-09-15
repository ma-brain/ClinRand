//! Blind-safety of `generation-report.html` (plan §6.5 / AGENTS.md §8).
//!
//! For every randomization number in the list, no line of the blinded HTML
//! may contain both that number and any arm code. The seed hex must not appear.
//! Pairing uses delimiter-aware tokens so single-letter arms do not match
//! substrings inside words like `PASS` / `Active`.

use std::collections::BTreeMap;

use clinrand_core::{
    generate, AllocationRecord, Arm, BlockScheme, GeneratedList, Method, NumberingScheme,
    StratificationFactor, StudyConfig,
};

use clinrand_package::{
    render_generation_report, render_unblinded_report, seed_hex, PackageMeta, ReportFileHashes,
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

/// Stratified variable-block DEMO config with padded global numbering.
fn demo_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-505".into(),
        protocol_version: "1.0".into(),
        arms: arms_ap(),
        method: Method::StratifiedBlock {
            block: BlockScheme::Variable { sizes: vec![4, 2] },
        },
        // Config order deliberately not lexicographic on level labels:
        // canonical order is 002 then 001 (last factor would vary fastest if
        // more factors existed).
        strata: vec![StratificationFactor {
            name: "site".into(),
            levels: vec!["002".into(), "001".into()],
        }],
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

fn demo_list() -> (StudyConfig, GeneratedList) {
    let cfg = demo_cfg();
    let list = generate(&cfg, demo_seed()).expect("generate DEMO-505 list");
    assert!(
        list.records.len() >= 4,
        "fixture must produce a real generate() list"
    );
    for rec in &list.records {
        assert_eq!(
            rec.randomization_number.len(),
            5,
            "padded numbers reduce false substring matches"
        );
    }
    (cfg, list)
}

/// True when `token` appears bounded by non-alphanumeric/underscore characters.
fn contains_delimited_token(haystack: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let token_bytes = token.as_bytes();
    let mut from = 0;
    while from + token_bytes.len() <= bytes.len() {
        if let Some(rel) = haystack[from..].find(token) {
            let i = from + rel;
            let before_ok = i == 0 || !is_token_char(bytes[i - 1]);
            let after = i + token_bytes.len();
            let after_ok = after >= bytes.len() || !is_token_char(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
            from = i + 1;
        } else {
            break;
        }
    }
    false
}

fn is_token_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn line_pairs_rand_with_arm(line: &str, rand_num: &str, arm: &str) -> bool {
    contains_delimited_token(line, rand_num) && contains_delimited_token(line, arm)
}

fn assert_no_rand_arm_pairing(html: &str, list: &GeneratedList, arm_codes: &[&str]) {
    for rec in &list.records {
        let rand_num = &rec.randomization_number;
        for (line_no, line) in html.lines().enumerate() {
            for arm in arm_codes {
                assert!(
                    !line_pairs_rand_with_arm(line, rand_num, arm),
                    "blind-safety: line {} pairs randomization number {rand_num} with arm {arm}:\n{line}",
                    line_no + 1
                );
            }
        }
    }
}

fn count_rand_arm_pairings(html: &str, list: &GeneratedList, arm_codes: &[&str]) -> usize {
    let mut n = 0;
    for rec in &list.records {
        let rand_num = &rec.randomization_number;
        for line in html.lines() {
            for arm in arm_codes {
                if line_pairs_rand_with_arm(line, rand_num, arm) {
                    n += 1;
                }
            }
        }
    }
    n
}

#[test]
fn blinded_report_never_pairs_randomization_number_with_arm_code() {
    let (cfg, list) = demo_list();
    let html = render_generation_report(&cfg, &list, &meta(), &demo_hashes());
    let arm_codes: Vec<&str> = cfg.arms.iter().map(|a| a.code.as_str()).collect();
    assert_no_rand_arm_pairing(&html, &list, &arm_codes);
}

#[test]
fn unblinded_report_negative_control_detects_rand_arm_pairing() {
    let (cfg, list) = demo_list();
    let html = render_unblinded_report(&cfg, &list, &meta(), &demo_hashes());
    let arm_codes: Vec<&str> = cfg.arms.iter().map(|a| a.code.as_str()).collect();
    let violations = count_rand_arm_pairings(&html, &list, &arm_codes);
    assert!(
        violations > 0,
        "negative control: unblinded HTML must contain at least one rand#↔arm pairing line so the detector has teeth"
    );
}

#[test]
fn blinded_report_property_details_policy() {
    let (cfg, list) = demo_list();
    let blinded = render_generation_report(&cfg, &list, &meta(), &demo_hashes());
    let unblinded = render_unblinded_report(&cfg, &list, &meta(), &demo_hashes());

    assert!(
        blinded.contains("P10 is informational only and must not cause regeneration"),
        "blinded report must state P10 must not cause regeneration"
    );
    assert!(
        blinded.contains("global_max=") || blinded.contains("max_run="),
        "blinded report should include P10 max-run detail when arm-code-safe"
    );

    let blinded_detail_lines = blinded.lines().filter(|l| l.contains("detail:")).count();
    let unblinded_detail_lines = unblinded.lines().filter(|l| l.contains("detail:")).count();
    assert_eq!(
        blinded_detail_lines, 1,
        "blinded report may include only P10 detail, got {blinded_detail_lines}"
    );
    assert!(
        unblinded_detail_lines >= 10,
        "unblinded report must still include all property details"
    );
    assert!(blinded.contains("P01"));
    assert!(
        blinded.contains("PASS") || blinded.contains("FAIL") || blinded.contains("informational")
    );
}

#[test]
fn per_stratum_sections_follow_canonical_config_order_including_zeros() {
    let cfg = demo_cfg();
    // Partial list: only site=002 present → site=001 must still appear as count=0,
    // and order must be config order (002 before 001), not BTreeMap label sort.
    let list = GeneratedList {
        records: vec![AllocationRecord {
            randomization_number: "10001".into(),
            stratum: BTreeMap::from([("site".into(), "002".into())]),
            block_id: 1,
            block_size: 2,
            position_in_block: 1,
            arm_code: "A".into(),
        }],
        stream: Default::default(),
    };
    let html = render_generation_report(&cfg, &list, &meta(), &demo_hashes());

    let pos_002 = html.find("stratum=site=002 count=").expect("site=002 row");
    let pos_001 = html
        .find("stratum=site=001 count=0")
        .expect("site=001 zero-count row");
    assert!(
        pos_002 < pos_001,
        "canonical config order is 002 then 001; got 002@{pos_002} 001@{pos_001}"
    );

    let block_002 = html
        .find("stratum=site=002 blocks:")
        .expect("site=002 blocks");
    let block_001 = html
        .find("stratum=site=001 blocks: (none)")
        .expect("site=001 empty blocks");
    assert!(
        block_002 < block_001,
        "block structure must also use canonical stratum order"
    );
}

#[test]
fn blinded_report_does_not_contain_seed_hex() {
    let (cfg, list) = demo_list();
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
    let (cfg, list) = demo_list();
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
