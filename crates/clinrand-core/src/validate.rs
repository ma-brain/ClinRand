//! Config validation (plan §5.2) and the §5.6 numbering disclosure warning.
//!
//! Does not consult JSON Schema (that is a later task). Does not implement
//! allocation. Integer arithmetic is checked; overflow is a [`ConfigError`].

use std::collections::HashSet;

use crate::config::{Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig};

/// Options that change which configs [`validate_config`] accepts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidateOptions {
    /// When true, Cartesian stratum combinations may exceed 200.
    ///
    /// Overflow of the combination count is still an error: the count
    /// cannot be represented, so it cannot be permitted.
    pub allow_large_strata: bool,
}

/// Why [`validate_config`] rejected a config (plan §5.2).
///
/// Every independent failure is collected; the first error is not fatal
/// to the rest of the checks.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// Fewer than two treatment arms.
    #[error("at least two arms are required")]
    TooFewArms,
    /// The same arm code appears more than once.
    #[error("duplicate arm code {0}")]
    DuplicateArmCode(String),
    /// Arm code is empty, longer than 12 characters, or contains a character
    /// outside `A-Za-z0-9_-`.
    #[error("arm code {0} is invalid")]
    InvalidArmCode(String),
    /// An arm has `ratio == 0`.
    #[error("arm {0} has ratio 0")]
    ZeroRatio(String),
    /// Sum of arm ratios does not fit in `u32`.
    #[error("arm ratio sum overflowed u32")]
    RatioSumOverflow,
    /// Fixed `size` is 0, or a variable `sizes` entry is 0.
    #[error("block size 0 is not allowed")]
    BlockSizeZero,
    /// Fixed block size is not a multiple of the ratio sum.
    #[error("fixed block size {size} is not a multiple of ratio sum {ratio_sum}")]
    FixedBlockSizeNotMultiple { size: u32, ratio_sum: u32 },
    /// A variable block size is not a multiple of the ratio sum.
    #[error("variable block size {size} is not a multiple of ratio sum {ratio_sum}")]
    VariableBlockSizeNotMultiple { size: u32, ratio_sum: u32 },
    /// `block.sizes` is empty.
    #[error("variable block sizes must not be empty")]
    EmptyBlockSizes,
    /// The same value appears more than once in `block.sizes`.
    #[error("duplicate variable block size {0}")]
    DuplicateBlockSize(u32),
    /// A variable block size is greater than 24.
    #[error("variable block size {0} exceeds 24")]
    BlockSizeTooLarge(u32),
    /// Two factors share a name.
    #[error("duplicate stratum factor name {0}")]
    DuplicateFactorName(String),
    /// A factor lists the same level more than once.
    #[error("duplicate level {level} in factor {factor}")]
    DuplicateLevel { factor: String, level: String },
    /// A factor has no levels, so no Cartesian combinations exist.
    #[error("factor {0} has no levels")]
    EmptyFactorLevels(String),
    /// Factor name is empty, longer than 32 characters, or contains a
    /// character outside `A-Za-z0-9_.-`.
    #[error("factor name {0} is invalid")]
    InvalidFactorName(String),
    /// Level name is empty, longer than 32 characters, or contains a
    /// character outside `A-Za-z0-9_.-`.
    #[error("level {level} of factor {factor} is invalid")]
    InvalidLevelName { factor: String, level: String },
    /// Cartesian product of factor levels exceeds 200 and was not overridden.
    #[error("stratum combinations {count} exceed 200")]
    TooManyStrata { count: u32 },
    /// Cartesian product of factor levels does not fit in `u32`.
    #[error("stratum combination count overflowed u32")]
    StratumCombinationOverflow,
    /// `stratified_block` requires at least one factor.
    #[error("stratified_block requires a non-empty strata array")]
    StratifiedBlockEmptyStrata,
    /// `permuted_block` must not be stratified.
    #[error("permuted_block must not include strata")]
    PermutedBlockNonEmptyStrata,
}

/// Non-fatal disclosure or truncation notes from [`validate_config`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigWarning {
    /// `list_length_per_stratum` is not a multiple of the ratio sum.
    /// The final block in each stratum may be truncated (plan §5.5).
    ListLengthNotMultiple { length: u32, ratio_sum: u32 },
    /// `per_stratum_range` numbering discloses stratum membership in the
    /// randomization number (plan §5.6).
    PerStratumRangeDisclosure,
}

impl std::fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListLengthNotMultiple { length, ratio_sum } => {
                write!(
                    f,
                    "list_length_per_stratum {length} is not a multiple of ratio sum {ratio_sum}; the final block may be truncated"
                )
            }
            Self::PerStratumRangeDisclosure => write!(
                f,
                "per_stratum_range numbering discloses stratum membership in the randomization number"
            ),
        }
    }
}

/// Validate `cfg` against plan §5.2.
///
/// Returns `Ok(warnings)` when every reject-rule passes. Warnings do not
/// fail validation. Returns `Err` with **every** independent reject-rule
/// failure; checks do not stop at the first error.
///
/// JSON Schema is not applied here.
pub fn validate_config(
    cfg: &StudyConfig,
    opts: &ValidateOptions,
) -> Result<Vec<ConfigWarning>, Vec<ConfigError>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    check_arms(&cfg.arms, &mut errors);
    let ratio_sum = checked_ratio_sum(&cfg.arms, &mut errors);

    match &cfg.method {
        Method::Simple => {}
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => {
            check_block(block, ratio_sum, &mut errors);
        }
    }

    match &cfg.method {
        Method::StratifiedBlock { .. } if cfg.strata.is_empty() => {
            errors.push(ConfigError::StratifiedBlockEmptyStrata);
        }
        Method::PermutedBlock { .. } if !cfg.strata.is_empty() => {
            errors.push(ConfigError::PermutedBlockNonEmptyStrata);
        }
        _ => {}
    }

    check_strata(&cfg.strata, &mut errors);

    match combination_count(&cfg.strata) {
        Ok(count) => {
            if count > 200 && !opts.allow_large_strata {
                errors.push(ConfigError::TooManyStrata { count });
            }
        }
        Err(err) => errors.push(err),
    }

    if let Some(sum) = ratio_sum {
        if sum > 0 && !cfg.list_length_per_stratum.is_multiple_of(sum) {
            warnings.push(ConfigWarning::ListLengthNotMultiple {
                length: cfg.list_length_per_stratum,
                ratio_sum: sum,
            });
        }
    }

    if matches!(cfg.numbering, NumberingScheme::PerStratumRange { .. }) {
        warnings.push(ConfigWarning::PerStratumRangeDisclosure);
    }

    if errors.is_empty() {
        Ok(warnings)
    } else {
        Err(errors)
    }
}

fn check_arms(arms: &[Arm], errors: &mut Vec<ConfigError>) {
    if arms.len() < 2 {
        errors.push(ConfigError::TooFewArms);
    }

    let mut seen = HashSet::new();
    let mut reported_dup = HashSet::new();
    for arm in arms {
        if !is_arm_code(&arm.code) {
            errors.push(ConfigError::InvalidArmCode(arm.code.clone()));
        }
        if !seen.insert(arm.code.as_str()) && reported_dup.insert(arm.code.as_str()) {
            errors.push(ConfigError::DuplicateArmCode(arm.code.clone()));
        }
        if arm.ratio == 0 {
            errors.push(ConfigError::ZeroRatio(arm.code.clone()));
        }
    }
}

fn checked_ratio_sum(arms: &[Arm], errors: &mut Vec<ConfigError>) -> Option<u32> {
    let mut sum = 0u32;
    for arm in arms {
        match sum.checked_add(arm.ratio) {
            Some(next) => sum = next,
            None => {
                errors.push(ConfigError::RatioSumOverflow);
                return None;
            }
        }
    }
    Some(sum)
}

fn check_block(block: &BlockScheme, ratio_sum: Option<u32>, errors: &mut Vec<ConfigError>) {
    match block {
        BlockScheme::Fixed { size } => {
            if *size == 0 {
                errors.push(ConfigError::BlockSizeZero);
            }
            if let Some(sum) = ratio_sum {
                if sum > 0 && !size.is_multiple_of(sum) {
                    errors.push(ConfigError::FixedBlockSizeNotMultiple {
                        size: *size,
                        ratio_sum: sum,
                    });
                }
            }
        }
        BlockScheme::Variable { sizes } => {
            if sizes.is_empty() {
                errors.push(ConfigError::EmptyBlockSizes);
                return;
            }
            let mut seen = HashSet::new();
            let mut reported_dup = HashSet::new();
            for &size in sizes {
                if size == 0 {
                    errors.push(ConfigError::BlockSizeZero);
                }
                if !seen.insert(size) && reported_dup.insert(size) {
                    errors.push(ConfigError::DuplicateBlockSize(size));
                }
                if size > 24 {
                    errors.push(ConfigError::BlockSizeTooLarge(size));
                }
                if let Some(sum) = ratio_sum {
                    if sum > 0 && !size.is_multiple_of(sum) {
                        errors.push(ConfigError::VariableBlockSizeNotMultiple {
                            size,
                            ratio_sum: sum,
                        });
                    }
                }
            }
        }
    }
}

fn check_strata(strata: &[StratificationFactor], errors: &mut Vec<ConfigError>) {
    let mut seen_names = HashSet::new();
    let mut reported_dup_names = HashSet::new();
    for factor in strata {
        if !is_factor_or_level_name(&factor.name) {
            errors.push(ConfigError::InvalidFactorName(factor.name.clone()));
        }
        if !seen_names.insert(factor.name.as_str())
            && reported_dup_names.insert(factor.name.as_str())
        {
            errors.push(ConfigError::DuplicateFactorName(factor.name.clone()));
        }
        if factor.levels.is_empty() {
            errors.push(ConfigError::EmptyFactorLevels(factor.name.clone()));
            continue;
        }
        let mut seen_levels = HashSet::new();
        let mut reported_dup_levels = HashSet::new();
        for level in &factor.levels {
            if !is_factor_or_level_name(level) {
                errors.push(ConfigError::InvalidLevelName {
                    factor: factor.name.clone(),
                    level: level.clone(),
                });
            }
            if !seen_levels.insert(level.as_str()) && reported_dup_levels.insert(level.as_str()) {
                errors.push(ConfigError::DuplicateLevel {
                    factor: factor.name.clone(),
                    level: level.clone(),
                });
            }
        }
    }
}

fn combination_count(strata: &[StratificationFactor]) -> Result<u32, ConfigError> {
    let mut count = 1u32;
    for factor in strata {
        let n = u32::try_from(factor.levels.len())
            .map_err(|_| ConfigError::StratumCombinationOverflow)?;
        count = count
            .checked_mul(n)
            .ok_or(ConfigError::StratumCombinationOverflow)?;
    }
    Ok(count)
}

fn is_arm_code(s: &str) -> bool {
    let len = s.len();
    (1..=12).contains(&len)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn is_factor_or_level_name(s: &str) -> bool {
    let len = s.len();
    (1..=32).contains(&len)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
