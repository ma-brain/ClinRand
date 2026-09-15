//! `validate_config` rules from plan §5.2 and the §5.6 numbering warning.
//!
//! Each §5.2 reject has a test that names the specific `ConfigError`.
//! Warnings are tested separately; they must not become rejects.

use clinrand_core::{
    validate_config, Arm, BlockScheme, ConfigError, ConfigWarning, Method, NumberingScheme,
    StratificationFactor, StudyConfig, ValidateOptions,
};

fn opts() -> ValidateOptions {
    ValidateOptions {
        allow_large_strata: false,
    }
}

fn validate(cfg: &StudyConfig) -> Result<Vec<ConfigWarning>, Vec<ConfigError>> {
    validate_config(cfg, &opts())
}

fn global_numbering() -> NumberingScheme {
    NumberingScheme::Global {
        start: 10001,
        width: 5,
    }
}

/// Plan §5.1 example shape (`DEMO-201`), known-valid under §5.2.
fn valid_stratified() -> StudyConfig {
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
                levels: vec!["001".into(), "002".into(), "003".into()],
            },
            StratificationFactor {
                name: "agegrp".into(),
                levels: vec!["LT65".into(), "GE65".into()],
            },
        ],
        list_length_per_stratum: 36,
        numbering: global_numbering(),
    }
}

fn valid_simple() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-201".into(),
        protocol_version: "2.1".into(),
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
        list_length_per_stratum: 24,
        numbering: global_numbering(),
    }
}

fn valid_permuted() -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: "DEMO-201".into(),
        protocol_version: "2.1".into(),
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
        method: Method::PermutedBlock {
            block: BlockScheme::Fixed { size: 4 },
        },
        strata: vec![],
        list_length_per_stratum: 24,
        numbering: global_numbering(),
    }
}

fn assert_has_error(cfg: StudyConfig, pred: impl Fn(&ConfigError) -> bool) {
    let errs = validate(&cfg).expect_err("config should be rejected");
    assert!(errs.iter().any(pred), "missing expected error in {errs:?}");
}

fn level_names(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("L{i:03}")).collect()
}

#[test]
fn valid_config_returns_ok_with_no_warnings() {
    let warnings = validate(&valid_stratified()).expect("§5.1-shaped config should be valid");
    assert!(
        warnings.is_empty(),
        "expected no warnings, got {warnings:?}"
    );
}

#[test]
fn rejects_fewer_than_two_arms() {
    let mut cfg = valid_stratified();
    cfg.arms.truncate(1);
    assert_has_error(cfg, |e| matches!(e, ConfigError::TooFewArms));
}

#[test]
fn rejects_simple_with_one_arm() {
    let mut cfg = valid_simple();
    cfg.arms.truncate(1);
    assert_has_error(cfg, |e| matches!(e, ConfigError::TooFewArms));
}

#[test]
fn rejects_duplicate_arm_codes() {
    let mut cfg = valid_stratified();
    cfg.arms[1].code = "A".into();
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::DuplicateArmCode(code) if code == "A"),
    );
}

#[test]
fn rejects_invalid_arm_code() {
    let mut cfg = valid_stratified();
    cfg.arms[0].code = "A.1".into();
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::InvalidArmCode(code) if code == "A.1"),
    );
}

#[test]
fn rejects_zero_ratio() {
    let mut cfg = valid_stratified();
    cfg.arms[1].ratio = 0;
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::ZeroRatio(code) if code == "P"),
    );
}

#[test]
fn rejects_fixed_block_size_not_multiple_of_ratio_sum() {
    let mut cfg = valid_permuted();
    cfg.arms[0].ratio = 2;
    cfg.arms[1].ratio = 1;
    match &mut cfg.method {
        Method::PermutedBlock { block } => *block = BlockScheme::Fixed { size: 4 },
        other => panic!("expected permuted_block, got {other:?}"),
    }
    assert_has_error(cfg, |e| {
        matches!(
            e,
            ConfigError::FixedBlockSizeNotMultiple {
                size: 4,
                ratio_sum: 3
            }
        )
    });
}

#[test]
fn rejects_variable_block_size_not_multiple_of_ratio_sum() {
    let mut cfg = valid_stratified();
    match &mut cfg.method {
        Method::StratifiedBlock { block } => *block = BlockScheme::Variable { sizes: vec![6, 8] },
        other => panic!("expected stratified_block, got {other:?}"),
    }
    assert_has_error(cfg, |e| {
        matches!(
            e,
            ConfigError::VariableBlockSizeNotMultiple {
                size: 8,
                ratio_sum: 3
            }
        )
    });
}

#[test]
fn rejects_empty_variable_block_sizes() {
    let mut cfg = valid_stratified();
    match &mut cfg.method {
        Method::StratifiedBlock { block } => *block = BlockScheme::Variable { sizes: vec![] },
        other => panic!("expected stratified_block, got {other:?}"),
    }
    assert_has_error(cfg, |e| matches!(e, ConfigError::EmptyBlockSizes));
}

#[test]
fn rejects_duplicate_variable_block_sizes() {
    let mut cfg = valid_stratified();
    match &mut cfg.method {
        Method::StratifiedBlock { block } => *block = BlockScheme::Variable { sizes: vec![6, 6] },
        other => panic!("expected stratified_block, got {other:?}"),
    }
    assert_has_error(cfg, |e| matches!(e, ConfigError::DuplicateBlockSize(6)));
}

#[test]
fn rejects_variable_block_size_above_24() {
    let mut cfg = valid_stratified();
    match &mut cfg.method {
        Method::StratifiedBlock { block } => *block = BlockScheme::Variable { sizes: vec![6, 27] },
        other => panic!("expected stratified_block, got {other:?}"),
    }
    assert_has_error(cfg, |e| matches!(e, ConfigError::BlockSizeTooLarge(27)));
}

#[test]
fn warns_when_list_length_is_not_a_multiple_of_ratio_sum() {
    let mut cfg = valid_stratified();
    cfg.list_length_per_stratum = 37;
    let warnings = validate(&cfg).expect("non-multiple list length is a warning, not a reject");
    assert!(
        warnings.iter().any(|w| matches!(
            w,
            ConfigWarning::ListLengthNotMultiple {
                length: 37,
                ratio_sum: 3
            }
        )),
        "missing list-length warning in {warnings:?}"
    );
}

#[test]
fn rejects_duplicate_stratum_factor_names() {
    let mut cfg = valid_stratified();
    cfg.strata[1].name = "site".into();
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::DuplicateFactorName(name) if name == "site"),
    );
}

#[test]
fn rejects_duplicate_levels_within_a_factor() {
    let mut cfg = valid_stratified();
    cfg.strata[0].levels = vec!["001".into(), "001".into()];
    assert_has_error(cfg, |e| {
        matches!(
            e,
            ConfigError::DuplicateLevel { factor, level } if factor == "site" && level == "001"
        )
    });
}

#[test]
fn rejects_empty_factor_levels() {
    let mut cfg = valid_stratified();
    cfg.strata[0].levels.clear();
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::EmptyFactorLevels(name) if name == "site"),
    );
}

#[test]
fn rejects_invalid_factor_name() {
    let mut cfg = valid_stratified();
    cfg.strata[0].name = "site id".into();
    assert_has_error(
        cfg,
        |e| matches!(e, ConfigError::InvalidFactorName(name) if name == "site id"),
    );
}

#[test]
fn rejects_invalid_level_name() {
    let mut cfg = valid_stratified();
    cfg.strata[1].levels[0] = "LT 65".into();
    assert_has_error(cfg, |e| {
        matches!(
            e,
            ConfigError::InvalidLevelName { factor, level } if factor == "agegrp" && level == "LT 65"
        )
    });
}

#[test]
fn rejects_more_than_200_stratum_combinations() {
    let mut cfg = valid_stratified();
    cfg.strata = vec![StratificationFactor {
        name: "site".into(),
        levels: level_names(201),
    }];
    assert_has_error(cfg, |e| {
        matches!(e, ConfigError::TooManyStrata { count: 201 })
    });
}

#[test]
fn two_hundred_stratum_combinations_are_accepted() {
    let mut cfg = valid_stratified();
    cfg.strata = vec![StratificationFactor {
        name: "site".into(),
        levels: level_names(200),
    }];
    validate(&cfg).expect("200 combinations is not over the sanity guard");
}

#[test]
fn allow_large_strata_accepts_more_than_200_combinations() {
    let mut cfg = valid_stratified();
    cfg.strata = vec![StratificationFactor {
        name: "site".into(),
        levels: level_names(201),
    }];
    let warnings = validate_config(
        &cfg,
        &ValidateOptions {
            allow_large_strata: true,
        },
    )
    .expect("override should accept 201 combinations");
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, ConfigWarning::ListLengthNotMultiple { .. })),
        "list length 36 is a multiple of 3; got {warnings:?}"
    );
}

#[test]
fn rejects_stratified_block_with_empty_strata() {
    let mut cfg = valid_stratified();
    cfg.strata.clear();
    assert_has_error(cfg, |e| {
        matches!(e, ConfigError::StratifiedBlockEmptyStrata)
    });
}

#[test]
fn rejects_permuted_block_with_non_empty_strata() {
    let mut cfg = valid_permuted();
    cfg.strata = vec![StratificationFactor {
        name: "site".into(),
        levels: vec!["001".into(), "002".into()],
    }];
    assert_has_error(cfg, |e| {
        matches!(e, ConfigError::PermutedBlockNonEmptyStrata)
    });
}

#[test]
fn simple_with_non_empty_strata_is_accepted() {
    let mut cfg = valid_simple();
    cfg.strata = vec![StratificationFactor {
        name: "site".into(),
        levels: vec!["001".into(), "002".into()],
    }];
    validate(&cfg).expect("simple with strata is allowed");
}

#[test]
fn per_stratum_range_is_accepted_with_disclosure_warning() {
    let mut cfg = valid_stratified();
    cfg.numbering = NumberingScheme::PerStratumRange {
        start: 20001,
        block_size: 100,
        width: 5,
    };
    let warnings = validate(&cfg).expect("per_stratum_range is accepted");
    assert_eq!(warnings, vec![ConfigWarning::PerStratumRangeDisclosure]);
}

#[test]
fn collects_all_errors_rather_than_stopping_at_the_first() {
    let mut cfg = valid_stratified();
    cfg.arms = vec![Arm {
        code: "A!".into(),
        label: "Only".into(),
        ratio: 0,
    }];
    cfg.strata.clear();
    let errs = validate(&cfg).expect_err("several independent rules are broken");
    assert!(
        errs.iter().any(|e| matches!(e, ConfigError::TooFewArms)),
        "missing TooFewArms in {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| matches!(e, ConfigError::InvalidArmCode(code) if code == "A!")),
        "missing InvalidArmCode in {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| matches!(e, ConfigError::ZeroRatio(code) if code == "A!")),
        "missing ZeroRatio in {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| matches!(e, ConfigError::StratifiedBlockEmptyStrata)),
        "missing StratifiedBlockEmptyStrata in {errs:?}"
    );
    assert!(
        errs.len() >= 4,
        "should collect every independent error, got {errs:?}"
    );
}

#[test]
fn rejects_ratio_sum_overflow() {
    let mut cfg = valid_simple();
    cfg.arms[0].ratio = u32::MAX;
    cfg.arms[1].ratio = 1;
    assert_has_error(cfg, |e| matches!(e, ConfigError::RatioSumOverflow));
}

#[test]
fn rejects_stratum_combination_count_overflow() {
    // 16^8 = 2^32, which does not fit in u32.
    let mut cfg = valid_stratified();
    cfg.strata = (0..8)
        .map(|i| StratificationFactor {
            name: format!("F{i:02}"),
            levels: (0..16).map(|j| format!("L{j:02}")).collect(),
        })
        .collect();
    assert_has_error(cfg, |e| {
        matches!(e, ConfigError::StratumCombinationOverflow)
    });
}

#[test]
fn combination_overflow_is_still_an_error_when_large_strata_are_allowed() {
    let mut cfg = valid_stratified();
    cfg.strata = (0..8)
        .map(|i| StratificationFactor {
            name: format!("F{i:02}"),
            levels: (0..16).map(|j| format!("L{j:02}")).collect(),
        })
        .collect();
    let err = validate_config(
        &cfg,
        &ValidateOptions {
            allow_large_strata: true,
        },
    )
    .expect_err("overflow cannot be overridden");
    assert!(err
        .iter()
        .any(|e| matches!(e, ConfigError::StratumCombinationOverflow)));
}

#[test]
fn fixed_block_size_above_24_is_accepted_when_a_multiple_of_ratios() {
    let mut cfg = valid_permuted();
    match &mut cfg.method {
        Method::PermutedBlock { block } => *block = BlockScheme::Fixed { size: 30 },
        other => panic!("expected permuted_block, got {other:?}"),
    }
    cfg.arms[0].ratio = 2;
    cfg.arms[1].ratio = 1;
    cfg.list_length_per_stratum = 30;
    validate(&cfg).expect("§5.2 caps variable sizes at 24, not fixed size");
}
