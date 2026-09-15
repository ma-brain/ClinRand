//! Exact-byte tests for list.csv, list.json, and stream.csv renderers.
//!
//! Fixtures use synthetic DEMO study IDs only. No seed appears in any
//! expected string or helper.

use std::collections::BTreeMap;

use clinrand_core::{
    AllocationRecord, Arm, BlockScheme, DrawPurpose, GeneratedList, Method, NumberingScheme,
    StratificationFactor, StreamDraw, StreamLog, StudyConfig,
};

use clinrand_package::{render_list_csv, render_list_json, render_stream_csv};

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

fn empty_strata_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-401".into(),
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

fn two_factor_cfg() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-402".into(),
        protocol_version: "1.0".into(),
        arms: arms_ap(),
        method: Method::StratifiedBlock {
            block: BlockScheme::Fixed { size: 2 },
        },
        // Config order is site then agegrp — not alphabetical (agegrp < site).
        strata: vec![
            StratificationFactor {
                name: "site".into(),
                levels: vec!["001".into()],
            },
            StratificationFactor {
                name: "agegrp".into(),
                levels: vec!["LT65".into()],
            },
        ],
        list_length_per_stratum: 2,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

fn record(
    randomization_number: &str,
    stratum: BTreeMap<String, String>,
    block_id: u32,
    block_size: u32,
    position_in_block: u32,
    arm_code: &str,
) -> AllocationRecord {
    AllocationRecord {
        randomization_number: randomization_number.into(),
        stratum,
        block_id,
        block_size,
        position_in_block,
        arm_code: arm_code.into(),
    }
}

fn list_with(records: Vec<AllocationRecord>) -> GeneratedList {
    GeneratedList {
        records,
        stream: StreamLog::default(),
    }
}

#[test]
fn list_csv_empty_strata_exact_bytes() {
    let cfg = empty_strata_cfg();
    let list = list_with(vec![
        record("10001", BTreeMap::new(), 1, 2, 1, "A"),
        record("10002", BTreeMap::new(), 1, 2, 2, "P"),
    ]);

    let expected = concat!(
        "randomization_number,block_id,block_size,position_in_block,arm_code\n",
        "10001,1,2,1,A\n",
        "10002,1,2,2,P\n",
    );

    let csv = render_list_csv(&cfg, &list).expect("list.csv");
    assert_eq!(csv, expected);
    assert!(
        csv.ends_with('\n') && !csv.ends_with("\n\n"),
        "exactly one trailing LF after the last row"
    );
    assert!(!csv.starts_with('\u{feff}'), "no BOM");
    assert!(!csv.contains('\r'), "LF only, no CR");
}

#[test]
fn list_csv_two_factors_config_order_not_alpha() {
    let cfg = two_factor_cfg();
    let mut stratum = BTreeMap::new();
    stratum.insert("agegrp".into(), "LT65".into());
    stratum.insert("site".into(), "001".into());
    let list = list_with(vec![
        record("10001", stratum.clone(), 1, 2, 1, "A"),
        record("10002", stratum, 1, 2, 2, "P"),
    ]);

    let expected = concat!(
        "randomization_number,site,agegrp,block_id,block_size,position_in_block,arm_code\n",
        "10001,001,LT65,1,2,1,A\n",
        "10002,001,LT65,1,2,2,P\n",
    );

    let csv = render_list_csv(&cfg, &list).expect("list.csv");
    assert_eq!(csv, expected);
    assert!(
        csv.contains("randomization_number,site,agegrp,block_id"),
        "stratum columns must follow config factor order (site before agegrp)"
    );
}

#[test]
fn list_csv_same_inputs_twice_are_byte_identical() {
    let cfg = two_factor_cfg();
    let mut stratum = BTreeMap::new();
    stratum.insert("site".into(), "001".into());
    stratum.insert("agegrp".into(), "LT65".into());
    let list = list_with(vec![record("10001", stratum, 1, 2, 1, "A")]);

    let a = render_list_csv(&cfg, &list).expect("a");
    let b = render_list_csv(&cfg, &list).expect("b");
    assert_eq!(a.as_bytes(), b.as_bytes());
}

#[test]
fn list_json_matches_csv_columns_and_is_stable() {
    let cfg = two_factor_cfg();
    let mut stratum = BTreeMap::new();
    stratum.insert("site".into(), "001".into());
    stratum.insert("agegrp".into(), "LT65".into());
    let list = list_with(vec![
        record("10001", stratum.clone(), 1, 2, 1, "A"),
        record("10002", stratum, 1, 2, 2, "P"),
    ]);

    // Compact JSON: {"records":[...]} with keys in CSV column order per object.
    // Stratum factors appear as top-level keys in config order (site, agegrp).
    // Trailing LF; no BOM; no insignificant spaces.
    let expected = concat!(
        r#"{"records":["#,
        r#"{"randomization_number":"10001","site":"001","agegrp":"LT65","block_id":1,"block_size":2,"position_in_block":1,"arm_code":"A"},"#,
        r#"{"randomization_number":"10002","site":"001","agegrp":"LT65","block_id":1,"block_size":2,"position_in_block":2,"arm_code":"P"}"#,
        r#"]}"#,
        "\n",
    );

    let json = render_list_json(&cfg, &list).expect("list.json");
    assert_eq!(json, expected);
    assert_eq!(
        render_list_json(&cfg, &list).expect("repeat").as_bytes(),
        json.as_bytes()
    );
    assert!(!json.contains("seed"), "seed must not appear in list.json");
}

#[test]
fn list_json_empty_strata_omits_stratum_keys() {
    let cfg = empty_strata_cfg();
    let list = list_with(vec![record("10001", BTreeMap::new(), 1, 2, 1, "A")]);
    let expected = concat!(
        r#"{"records":[{"randomization_number":"10001","block_id":1,"block_size":2,"position_in_block":1,"arm_code":"A"}]}"#,
        "\n",
    );
    assert_eq!(render_list_json(&cfg, &list).expect("list.json"), expected);
}

#[test]
fn stream_csv_empty_is_header_plus_newline() {
    let stream = StreamLog::default();
    let expected = "index,bound,value,purpose\n";
    let csv = render_stream_csv(&stream).expect("stream.csv");
    assert_eq!(csv, expected);
}

#[test]
fn stream_csv_mixed_purposes_exact_rows() {
    let stream = StreamLog {
        draws: vec![
            StreamDraw {
                index: 0,
                bound: 2,
                value: 1,
                purpose: DrawPurpose::BlockSize,
            },
            StreamDraw {
                index: 1,
                bound: 6,
                value: 3,
                purpose: DrawPurpose::Permutation,
            },
            StreamDraw {
                index: 2,
                bound: 2,
                value: 0,
                purpose: DrawPurpose::SimpleAllocation,
            },
        ],
    };

    let expected = concat!(
        "index,bound,value,purpose\n",
        "0,2,1,block_size\n",
        "1,6,3,permutation\n",
        "2,2,0,simple_allocation\n",
    );

    let csv = render_stream_csv(&stream).expect("stream.csv");
    assert_eq!(csv, expected);
    assert_eq!(
        render_stream_csv(&stream).expect("repeat").as_bytes(),
        csv.as_bytes()
    );
    assert!(!csv.contains("seed"));
}
