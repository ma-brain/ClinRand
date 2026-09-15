//! StudyConfig JSON wire format (plan §5.1).
//!
//! Expected values are the §5.1 example with `study_id` changed to
//! `DEMO-201` (public-repo rule). They are not this engine's own output.

use clinrand_core::{Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig};

/// Plan §5.1 example, `study_id` set to `DEMO-201`.
const SECTION_5_1_EXAMPLE: &str = r#"{
  "schema_version": "1.0",
  "study_id": "DEMO-201",
  "protocol_version": "2.1",
  "arms": [
    { "code": "A", "label": "Investigational product 50 mg", "ratio": 2 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "stratified_block",
  "block": { "kind": "variable", "sizes": [6, 9] },
  "strata": [
    { "name": "site", "levels": ["001", "002", "003"] },
    { "name": "agegrp", "levels": ["LT65", "GE65"] }
  ],
  "list_length_per_stratum": 36,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}"#;

#[test]
fn section_5_1_example_deserializes_and_round_trips() {
    let cfg: StudyConfig =
        serde_json::from_str(SECTION_5_1_EXAMPLE).expect("§5.1 example should deserialize");

    assert_eq!(cfg.schema_version, "1.0");
    assert_eq!(cfg.study_id, "DEMO-201");
    assert_eq!(cfg.protocol_version, "2.1");
    assert_eq!(
        cfg.arms,
        vec![
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
        ]
    );
    match &cfg.method {
        Method::StratifiedBlock {
            block: BlockScheme::Variable { sizes },
        } => assert_eq!(sizes, &vec![6, 9]),
        other => panic!("expected stratified_block with variable sizes, got {other:?}"),
    }
    assert_eq!(
        cfg.strata,
        vec![
            StratificationFactor {
                name: "site".into(),
                levels: vec!["001".into(), "002".into(), "003".into()],
            },
            StratificationFactor {
                name: "agegrp".into(),
                levels: vec!["LT65".into(), "GE65".into()],
            },
        ]
    );
    assert_eq!(cfg.list_length_per_stratum, 36);
    match cfg.numbering {
        NumberingScheme::Global { start, width } => {
            assert_eq!(start, 10001);
            assert_eq!(width, 5);
        }
        other => panic!("expected global numbering, got {other:?}"),
    }

    let serialized = serde_json::to_value(&cfg).expect("StudyConfig should serialize");
    assert_eq!(serialized["method"], "stratified_block");
    assert_eq!(serialized["block"]["kind"], "variable");
    assert!(
        serialized.get("block").is_some(),
        "block must be a sibling of method, not nested inside it"
    );

    let round_tripped: StudyConfig =
        serde_json::from_value(serialized).expect("serialized config should deserialize");
    assert_eq!(cfg, round_tripped);
}

fn base_object() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "1.0",
        "study_id": "DEMO-201",
        "protocol_version": "2.1",
        "arms": [{ "code": "A", "label": "Active", "ratio": 1 }, { "code": "P", "label": "Placebo", "ratio": 1 }],
        "strata": [],
        "list_length_per_stratum": 24,
        "numbering": { "kind": "global", "start": 10001, "width": 5 }
    })
}

#[test]
fn simple_has_no_block_on_the_wire() {
    let mut value = base_object();
    value["method"] = serde_json::json!("simple");
    let cfg: StudyConfig = serde_json::from_value(value).expect("simple should deserialize");
    assert!(matches!(cfg.method, Method::Simple));

    let serialized = serde_json::to_value(&cfg).expect("simple should serialize");
    assert_eq!(serialized["method"], "simple");
    assert!(
        serialized.get("block").is_none(),
        "simple must not emit a block field, got {serialized}"
    );

    let round_tripped: StudyConfig =
        serde_json::from_value(serialized).expect("simple round-trip should deserialize");
    assert_eq!(cfg, round_tripped);
}

#[test]
fn simple_rejects_a_sibling_block() {
    let mut value = base_object();
    value["method"] = serde_json::json!("simple");
    value["block"] = serde_json::json!({ "kind": "fixed", "size": 4 });
    let err = serde_json::from_value::<StudyConfig>(value)
        .expect_err("simple must not accept a block field");
    let msg = err.to_string();
    assert!(
        msg.contains("simple") || msg.contains("block"),
        "error should mention simple/block, got {msg}"
    );
}

#[test]
fn permuted_block_requires_block() {
    let mut value = base_object();
    value["method"] = serde_json::json!("permuted_block");
    let err = serde_json::from_value::<StudyConfig>(value)
        .expect_err("permuted_block without block must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("block") || msg.contains("permuted_block"),
        "error should mention block, got {msg}"
    );
}

#[test]
fn stratified_block_requires_block() {
    let mut value = base_object();
    value["method"] = serde_json::json!("stratified_block");
    let err = serde_json::from_value::<StudyConfig>(value)
        .expect_err("stratified_block without block must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("block") || msg.contains("stratified_block"),
        "error should mention block, got {msg}"
    );
}

#[test]
fn extra_top_level_key_fails_deserialize() {
    let mut value = base_object();
    value["method"] = serde_json::json!("simple");
    value["operator_notes"] = serde_json::json!("must not be dropped");
    let err = serde_json::from_value::<StudyConfig>(value)
        .expect_err("unknown top-level keys must fail deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown") || msg.contains("operator_notes"),
        "error should name the unknown field, got {msg}"
    );
}

#[test]
fn block_typo_blok_fails_deserialize() {
    let mut value = base_object();
    value["method"] = serde_json::json!("permuted_block");
    value["block"] = serde_json::json!({ "kind": "fixed", "size": 4 });
    value["blok"] = serde_json::json!({ "kind": "fixed", "size": 4 });
    let err = serde_json::from_value::<StudyConfig>(value)
        .expect_err("typo \"blok\" must fail deserialize, not be dropped");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown") || msg.contains("blok"),
        "error should mention the unknown field, got {msg}"
    );
}

#[test]
fn permuted_block_round_trips() {
    let mut value = base_object();
    value["method"] = serde_json::json!("permuted_block");
    value["block"] = serde_json::json!({ "kind": "fixed", "size": 4 });
    let cfg: StudyConfig =
        serde_json::from_value(value).expect("permuted_block should deserialize");
    match &cfg.method {
        Method::PermutedBlock {
            block: BlockScheme::Fixed { size },
        } => assert_eq!(*size, 4),
        other => panic!("expected permuted_block fixed, got {other:?}"),
    }
    let serialized = serde_json::to_value(&cfg).expect("permuted_block should serialize");
    assert_eq!(serialized["method"], "permuted_block");
    assert_eq!(serialized["block"]["kind"], "fixed");
    let round_tripped: StudyConfig =
        serde_json::from_value(serialized).expect("permuted_block round-trip");
    assert_eq!(cfg, round_tripped);
}

#[test]
fn per_stratum_range_numbering_round_trips() {
    let mut value = base_object();
    value["method"] = serde_json::json!("permuted_block");
    value["block"] = serde_json::json!({ "kind": "fixed", "size": 4 });
    value["numbering"] = serde_json::json!({ "kind": "per_stratum_range", "start": 20001, "block_size": 100, "width": 5 });
    let cfg: StudyConfig =
        serde_json::from_value(value).expect("per_stratum_range should deserialize");
    match cfg.numbering {
        NumberingScheme::PerStratumRange {
            start,
            block_size,
            width,
        } => {
            assert_eq!(start, 20001);
            assert_eq!(block_size, 100);
            assert_eq!(width, 5);
        }
        other => panic!("expected per_stratum_range, got {other:?}"),
    }
    let serialized = serde_json::to_value(&cfg).expect("per_stratum_range should serialize");
    assert_eq!(serialized["numbering"]["kind"], "per_stratum_range");
    let round_tripped: StudyConfig =
        serde_json::from_value(serialized).expect("per_stratum_range round-trip");
    assert_eq!(cfg, round_tripped);
}
