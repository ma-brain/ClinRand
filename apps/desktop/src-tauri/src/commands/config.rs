//! Config-facing commands: `validate_config_json` and `preview_structure`.
//!
//! Both parse a `StudyConfig` from JSON and derive information from the config
//! alone. Neither draws entropy, calls `generate`, or produces any arm
//! assignment for a randomization number (AGENTS.md §4.8, plan Global
//! Constraints). Allocation, hashing, and canonicalization stay in the engine
//! crates — this module only calls into `clinrand_core`.

use clinrand_core::{
    stratum_combinations, validate_config, Arm, BlockScheme, ConfigError, Method, StudyConfig,
    ValidateOptions,
};
use serde::Serialize;

/// One config error rendered for the UI: a stable machine code plus message.
#[derive(Clone, Debug, Serialize)]
pub struct ConfigIssue {
    /// Stable snake_case identifier for the failure kind.
    pub code: String,
    /// Human-readable message (the error's `Display`).
    pub message: String,
}

/// Structured result of validating a config JSON string.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationOutcome {
    /// True when the JSON parsed and `validate_config` returned no errors.
    pub ok: bool,
    /// Every independent validation error (or a single `invalid_json` entry).
    pub errors: Vec<ConfigIssue>,
    /// Non-fatal warnings (disclosure / truncation notes).
    pub warnings: Vec<String>,
}

/// Parse `json` into a [`StudyConfig`] and run [`validate_config`].
///
/// Invalid JSON is reported as `ok: false` with a single `invalid_json` error
/// rather than a command failure, so the UI can render it uniformly. The seed
/// is never involved here.
#[tauri::command]
pub fn validate_config_json(json: String, allow_large_strata: bool) -> ValidationOutcome {
    let cfg: StudyConfig = match serde_json::from_str(&json) {
        Ok(cfg) => cfg,
        Err(err) => {
            return ValidationOutcome {
                ok: false,
                errors: vec![ConfigIssue {
                    code: "invalid_json".to_string(),
                    message: format!("could not parse config JSON: {err}"),
                }],
                warnings: Vec::new(),
            };
        }
    };

    let options = ValidateOptions { allow_large_strata };
    match validate_config(&cfg, &options) {
        Ok(warnings) => ValidationOutcome {
            ok: true,
            errors: Vec::new(),
            warnings: warnings.iter().map(ToString::to_string).collect(),
        },
        Err(errors) => ValidationOutcome {
            ok: false,
            errors: errors.iter().map(config_issue).collect(),
            warnings: Vec::new(),
        },
    }
}

/// A single stratum combination in canonical (config) order.
#[derive(Clone, Debug, Serialize)]
pub struct StratumCombination {
    /// `factor -> level` pairs in config factor order (last factor fastest).
    pub levels: Vec<StratumLevel>,
}

/// One `factor = level` pair within a stratum combination.
#[derive(Clone, Debug, Serialize)]
pub struct StratumLevel {
    /// Factor name.
    pub factor: String,
    /// Level value.
    pub level: String,
}

/// Expected per-arm total derived from ratios (integer division).
#[derive(Clone, Debug, Serialize)]
pub struct ArmTotal {
    /// Arm code.
    pub code: String,
    /// Arm label.
    pub label: String,
    /// Allocation ratio.
    pub ratio: u32,
    /// `total_records * ratio / ratio_sum` (floored; see `remainder`).
    pub total: u64,
}

/// Blinded block structure. Never contains arm assignments.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockPreview {
    /// `simple` randomization: no blocking.
    None,
    /// Fixed-size permuted blocks.
    Fixed {
        /// Block size.
        block_size: u32,
        /// Whole blocks of `block_size` per stratum.
        full_blocks_per_stratum: u64,
        /// Size of the trailing truncated block (0 when it divides evenly).
        final_block_size: u32,
        /// Total blocks per stratum (full plus any truncated block).
        blocks_per_stratum: u64,
    },
    /// Variable-size permuted blocks; concrete sizes are drawn at generation.
    Variable {
        /// Allowed block sizes from the config.
        allowed_sizes: Vec<u32>,
        /// Note that concrete draws happen during generation, not preview.
        note: String,
    },
}

/// Blinded, config-derived structure preview. No allocations, no RNG.
#[derive(Clone, Debug, Serialize)]
pub struct StructurePreview {
    /// Study identifier from the config.
    pub study_id: String,
    /// Method name (`simple` / `permuted_block` / `stratified_block`).
    pub method: String,
    /// Number of stratum combinations (1 when unstratified).
    pub n_strata: u64,
    /// Stratum combinations in canonical order.
    pub strata_combinations: Vec<StratumCombination>,
    /// Records allocated per stratum combination.
    pub list_length_per_stratum: u32,
    /// `list_length_per_stratum * n_strata` (checked).
    pub total_records: u64,
    /// Sum of arm ratios.
    pub ratio_sum: u32,
    /// Expected per-arm totals from ratios (integer division).
    pub arms: Vec<ArmTotal>,
    /// `total_records - sum(arm totals)`; positions ratios cannot split evenly.
    pub ratio_remainder: u64,
    /// Blinded block structure.
    pub block: BlockPreview,
    /// Non-fatal validation warnings, if the config validates.
    pub warnings: Vec<String>,
}

/// Derive a blinded structure preview from `json` (config-derived only).
///
/// Returns stratum combinations, total record counts, per-arm ratio totals,
/// and block structure. It never calls `generate`, never draws entropy, and
/// never emits an arm assignment for any randomization number. Errors are
/// returned as strings and contain no seed material (no seed exists here).
#[tauri::command]
pub fn preview_structure(json: String) -> Result<StructurePreview, String> {
    let cfg: StudyConfig =
        serde_json::from_str(&json).map_err(|err| format!("could not parse config JSON: {err}"))?;

    let ratio_sum = checked_ratio_sum(&cfg.arms)?;
    if ratio_sum == 0 {
        return Err("cannot preview structure: arm ratio sum is zero".to_string());
    }

    let combinations = stratum_combinations(&cfg.strata)
        .map_err(|err| format!("cannot enumerate stratum combinations: {err}"))?;
    let n_strata = u64::try_from(combinations.len())
        .map_err(|_| "stratum combination count does not fit u64".to_string())?;

    let total_records = u64::from(cfg.list_length_per_stratum)
        .checked_mul(n_strata)
        .ok_or_else(|| "total record count overflowed u64".to_string())?;

    // Present each combination in config factor order (last factor fastest),
    // reading levels from the canonical map returned by the engine.
    let strata_combinations = combinations
        .iter()
        .map(|combo| StratumCombination {
            levels: cfg
                .strata
                .iter()
                .filter_map(|factor| {
                    combo.get(&factor.name).map(|level| StratumLevel {
                        factor: factor.name.clone(),
                        level: level.clone(),
                    })
                })
                .collect(),
        })
        .collect();

    let arms = arm_totals(&cfg.arms, total_records, ratio_sum)?;
    let allocated: u64 = arms.iter().map(|arm| arm.total).sum();
    let ratio_remainder = total_records.saturating_sub(allocated);

    let block = block_preview(&cfg.method, cfg.list_length_per_stratum)?;

    // Surface warnings when the config validates; do not fail preview on
    // validation errors — preview is intentionally config-derived and lenient
    // so operators can inspect structure while still editing.
    let warnings = match validate_config(
        &cfg,
        &ValidateOptions {
            allow_large_strata: true,
        },
    ) {
        Ok(warnings) => warnings.iter().map(ToString::to_string).collect(),
        Err(_) => Vec::new(),
    };

    Ok(StructurePreview {
        study_id: cfg.study_id,
        method: method_name(&cfg.method).to_string(),
        n_strata,
        strata_combinations,
        list_length_per_stratum: cfg.list_length_per_stratum,
        total_records,
        ratio_sum,
        arms,
        ratio_remainder,
        block,
        warnings,
    })
}

fn checked_ratio_sum(arms: &[Arm]) -> Result<u32, String> {
    let mut sum = 0u32;
    for arm in arms {
        sum = sum
            .checked_add(arm.ratio)
            .ok_or_else(|| "arm ratio sum overflowed u32".to_string())?;
    }
    Ok(sum)
}

fn arm_totals(arms: &[Arm], total_records: u64, ratio_sum: u32) -> Result<Vec<ArmTotal>, String> {
    let ratio_sum = u64::from(ratio_sum);
    arms.iter()
        .map(|arm| {
            let total = u64::from(arm.ratio)
                .checked_mul(total_records)
                .ok_or_else(|| "per-arm total overflowed u64".to_string())?
                .checked_div(ratio_sum)
                .ok_or_else(|| "ratio sum is zero".to_string())?;
            Ok(ArmTotal {
                code: arm.code.clone(),
                label: arm.label.clone(),
                ratio: arm.ratio,
                total,
            })
        })
        .collect()
}

fn block_preview(method: &Method, list_length: u32) -> Result<BlockPreview, String> {
    match method {
        Method::Simple => Ok(BlockPreview::None),
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => match block {
            BlockScheme::Fixed { size } => {
                if *size == 0 {
                    return Err("cannot preview structure: block size is zero".to_string());
                }
                let full_blocks_per_stratum = u64::from(list_length / size);
                let final_block_size = list_length % size;
                let blocks_per_stratum = full_blocks_per_stratum
                    .checked_add(u64::from(final_block_size > 0))
                    .ok_or_else(|| "block count overflowed u64".to_string())?;
                Ok(BlockPreview::Fixed {
                    block_size: *size,
                    full_blocks_per_stratum,
                    final_block_size,
                    blocks_per_stratum,
                })
            }
            BlockScheme::Variable { sizes } => Ok(BlockPreview::Variable {
                allowed_sizes: sizes.clone(),
                note: "concrete block sizes are drawn from these allowed sizes at generation time"
                    .to_string(),
            }),
        },
    }
}

fn method_name(method: &Method) -> &'static str {
    match method {
        Method::Simple => "simple",
        Method::PermutedBlock { .. } => "permuted_block",
        Method::StratifiedBlock { .. } => "stratified_block",
    }
}

/// Map a [`ConfigError`] to a UI-facing issue (stable code + message).
fn config_issue(error: &ConfigError) -> ConfigIssue {
    let code = match error {
        ConfigError::TooFewArms => "too_few_arms",
        ConfigError::DuplicateArmCode(_) => "duplicate_arm_code",
        ConfigError::InvalidArmCode(_) => "invalid_arm_code",
        ConfigError::ZeroRatio(_) => "zero_ratio",
        ConfigError::RatioSumOverflow => "ratio_sum_overflow",
        ConfigError::BlockSizeZero => "block_size_zero",
        ConfigError::FixedBlockSizeNotMultiple { .. } => "fixed_block_size_not_multiple",
        ConfigError::VariableBlockSizeNotMultiple { .. } => "variable_block_size_not_multiple",
        ConfigError::EmptyBlockSizes => "empty_block_sizes",
        ConfigError::DuplicateBlockSize(_) => "duplicate_block_size",
        ConfigError::BlockSizeTooLarge(_) => "block_size_too_large",
        ConfigError::DuplicateFactorName(_) => "duplicate_factor_name",
        ConfigError::DuplicateLevel { .. } => "duplicate_level",
        ConfigError::EmptyFactorLevels(_) => "empty_factor_levels",
        ConfigError::InvalidFactorName(_) => "invalid_factor_name",
        ConfigError::InvalidLevelName { .. } => "invalid_level_name",
        ConfigError::TooManyStrata { .. } => "too_many_strata",
        ConfigError::StratumCombinationOverflow => "stratum_combination_overflow",
        ConfigError::StratifiedBlockEmptyStrata => "stratified_block_empty_strata",
        ConfigError::PermutedBlockNonEmptyStrata => "permuted_block_non_empty_strata",
        ConfigError::PerStratumRangeTooSmall { .. } => "per_stratum_range_too_small",
    };
    ConfigIssue {
        code: code.to_string(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_CONFIG: &str = r#"{
        "schema_version": "1.0",
        "study_id": "DEMO-201",
        "protocol_version": "1.0",
        "arms": [
            { "code": "A", "label": "Active", "ratio": 1 },
            { "code": "P", "label": "Placebo", "ratio": 1 }
        ],
        "method": "stratified_block",
        "block": { "kind": "fixed", "size": 4 },
        "strata": [
            { "name": "site", "levels": ["001", "002"] },
            { "name": "agegrp", "levels": ["LT65", "GE65"] }
        ],
        "list_length_per_stratum": 8,
        "numbering": { "kind": "global", "start": 10001, "width": 5 }
    }"#;

    #[test]
    fn validate_accepts_known_good_config() {
        let outcome = validate_config_json(GOOD_CONFIG.to_string(), false);
        assert!(outcome.ok, "errors: {:?}", outcome.errors);
        assert!(outcome.errors.is_empty());
    }

    #[test]
    fn validate_rejects_invalid_json() {
        let outcome = validate_config_json("{ not valid json".to_string(), false);
        assert!(!outcome.ok);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(outcome.errors[0].code, "invalid_json");
    }

    #[test]
    fn validate_rejects_invalid_config() {
        // One arm -> TooFewArms; ratio 0 -> ZeroRatio.
        let json = r#"{
            "schema_version": "1.0",
            "study_id": "TEST-1",
            "protocol_version": "1.0",
            "arms": [ { "code": "A", "label": "Active", "ratio": 0 } ],
            "method": "simple",
            "strata": [],
            "list_length_per_stratum": 4,
            "numbering": { "kind": "global", "start": 1, "width": 4 }
        }"#;
        let outcome = validate_config_json(json.to_string(), false);
        assert!(!outcome.ok);
        let codes: Vec<&str> = outcome.errors.iter().map(|e| e.code.as_str()).collect();
        assert!(codes.contains(&"too_few_arms"), "codes: {codes:?}");
    }

    #[test]
    fn preview_computes_strata_totals_and_blocks() {
        let preview = preview_structure(GOOD_CONFIG.to_string()).expect("preview");
        assert_eq!(preview.n_strata, 4);
        assert_eq!(preview.total_records, 32);
        assert_eq!(preview.ratio_sum, 2);
        assert_eq!(preview.arms.len(), 2);
        assert_eq!(preview.arms[0].total, 16);
        assert_eq!(preview.arms[1].total, 16);
        assert_eq!(preview.ratio_remainder, 0);
        match preview.block {
            BlockPreview::Fixed {
                block_size,
                full_blocks_per_stratum,
                final_block_size,
                blocks_per_stratum,
            } => {
                assert_eq!(block_size, 4);
                assert_eq!(full_blocks_per_stratum, 2);
                assert_eq!(final_block_size, 0);
                assert_eq!(blocks_per_stratum, 2);
            }
            other => panic!("expected fixed block, got {other:?}"),
        }
        // Canonical order: last factor (agegrp) varies fastest.
        assert_eq!(preview.strata_combinations[0].levels[0].level, "001");
        assert_eq!(preview.strata_combinations[0].levels[1].level, "LT65");
        assert_eq!(preview.strata_combinations[1].levels[1].level, "GE65");
    }

    #[test]
    fn preview_reports_ratio_remainder() {
        // 2:1 ratio over list_length 10 (unstratified) -> 6 + 3 = 9, remainder 1.
        let json = r#"{
            "schema_version": "1.0",
            "study_id": "EXAMPLE-9",
            "protocol_version": "1.0",
            "arms": [
                { "code": "A", "label": "Active", "ratio": 2 },
                { "code": "P", "label": "Placebo", "ratio": 1 }
            ],
            "method": "simple",
            "strata": [],
            "list_length_per_stratum": 10,
            "numbering": { "kind": "global", "start": 1, "width": 4 }
        }"#;
        let preview = preview_structure(json.to_string()).expect("preview");
        assert_eq!(preview.total_records, 10);
        assert_eq!(preview.arms[0].total, 6);
        assert_eq!(preview.arms[1].total, 3);
        assert_eq!(preview.ratio_remainder, 1);
        assert!(matches!(preview.block, BlockPreview::None));
    }

    #[test]
    fn preview_rejects_invalid_json() {
        let err = preview_structure("nonsense".to_string()).unwrap_err();
        assert!(err.contains("could not parse"));
    }
}
