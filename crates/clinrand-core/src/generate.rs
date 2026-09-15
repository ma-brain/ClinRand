//! List generation: allocation, numbering, and truncation (plan §5.3–5.6).

use std::collections::BTreeMap;

use crate::config::{Arm, BlockScheme, Method, NumberingScheme, StudyConfig};
use crate::permute::permute;
use crate::rng::Rng;
use crate::strata::stratum_combinations;
use crate::stream::{DrawPurpose, StreamLog};
use crate::uniform::{uniform_below, UniformError};
use crate::validate::{validate_config, ConfigError, ValidateOptions};

/// One generated list plus the stream log for that run (plan §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedList {
    /// Allocations in emission order (canonical strata, then blocks, then positions).
    pub records: Vec<AllocationRecord>,
    /// Accepted `uniform_below` draws for this run.
    pub stream: StreamLog,
}

/// One row of the randomization list (plan §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationRecord {
    /// Formatted randomization number (plan §5.6).
    pub randomization_number: String,
    /// Stratum factor → level; empty when unstratified.
    pub stratum: BTreeMap<String, String>,
    /// Block index within the stratum, starting at 1.
    pub block_id: u32,
    /// Size of the block this position belongs to (1 for `simple`).
    pub block_size: u32,
    /// Position within the block, starting at 1 (only kept positions).
    pub position_in_block: u32,
    /// Assigned treatment arm code.
    pub arm_code: String,
}

/// Why [`generate`] failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GenerationError {
    /// [`validate_config`] rejected the config.
    #[error("config validation failed")]
    InvalidConfig(Vec<ConfigError>),
    /// An allocation-path `uniform_below` draw failed.
    #[error(transparent)]
    Uniform(#[from] UniformError),
    /// Integer arithmetic overflowed while building or numbering the list.
    #[error("allocation-path arithmetic overflow")]
    Overflow,
    /// Block construction parameters violate invariants that
    /// [`validate_config`] should already have enforced (defensive belt).
    #[error("block size {block_size} is not a multiple of ratio sum {ratio_sum}")]
    InconsistentConfig { block_size: u32, ratio_sum: u32 },
}

/// Mutable state shared across strata for one generation run.
struct RunState<'a> {
    cfg: &'a StudyConfig,
    rng: Rng,
    stream: StreamLog,
    records: Vec<AllocationRecord>,
    ratio_sum: u32,
}

/// Generate a randomization list for `(cfg, seed)` with explicit validation options.
///
/// Contract-bound: one [`Rng::from_seed`] per run, one shared [`StreamLog`],
/// stream order per plan §2.4 / [`crate`] determinism docs. Must not change
/// without an `ALGO_VERSION` bump. Pure: no I/O, clock, or environment.
pub fn generate_with_options(
    cfg: &StudyConfig,
    seed: [u8; 32],
    options: &ValidateOptions,
) -> Result<GeneratedList, GenerationError> {
    validate_config(cfg, options).map_err(GenerationError::InvalidConfig)?;

    let mut run = RunState {
        cfg,
        rng: Rng::from_seed(seed),
        stream: StreamLog::default(),
        records: Vec::new(),
        ratio_sum: ratio_sum(&cfg.arms)?,
    };
    let strata = stratum_combinations(&cfg.strata).map_err(|_| GenerationError::Overflow)?;

    match &cfg.method {
        Method::Simple => {
            for (stratum_index, stratum) in strata.iter().enumerate() {
                run.append_simple_stratum(stratum, stratum_index)?;
            }
        }
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => {
            let sorted_sizes = sorted_deduped_sizes(block);
            for (stratum_index, stratum) in strata.iter().enumerate() {
                run.append_block_stratum(block, sorted_sizes.as_deref(), stratum, stratum_index)?;
            }
        }
    }

    Ok(GeneratedList {
        records: run.records,
        stream: run.stream,
    })
}

/// Generate a randomization list for `(cfg, seed)`.
///
/// Uses [`ValidateOptions`] with `allow_large_strata: false`. See
/// [`generate_with_options`] for override behaviour.
pub fn generate(cfg: &StudyConfig, seed: [u8; 32]) -> Result<GeneratedList, GenerationError> {
    generate_with_options(
        cfg,
        seed,
        &ValidateOptions {
            allow_large_strata: false,
        },
    )
}

impl RunState<'_> {
    fn append_simple_stratum(
        &mut self,
        stratum: &BTreeMap<String, String>,
        stratum_index: usize,
    ) -> Result<(), GenerationError> {
        let bound = u64::from(self.ratio_sum);
        for pos in 0..self.cfg.list_length_per_stratum {
            let draw = uniform_below(
                &mut self.rng,
                &mut self.stream,
                bound,
                DrawPurpose::SimpleAllocation,
            )?;
            let arm_code = arm_for_simple_draw(&self.cfg.arms, draw)?;
            let block_id = pos.checked_add(1).ok_or(GenerationError::Overflow)?;
            let number = randomization_number(self.cfg, stratum_index, self.records.len(), pos)?;
            self.records.push(AllocationRecord {
                randomization_number: number,
                stratum: stratum.clone(),
                block_id,
                block_size: 1,
                position_in_block: 1,
                arm_code,
            });
        }
        Ok(())
    }

    fn append_block_stratum(
        &mut self,
        block: &BlockScheme,
        sorted_sizes: Option<&[u32]>,
        stratum: &BTreeMap<String, String>,
        stratum_index: usize,
    ) -> Result<(), GenerationError> {
        let mut remaining = self.cfg.list_length_per_stratum;
        let mut block_id = 1u32;
        let mut pos_in_stratum = 0u32;

        while remaining > 0 {
            let block_size =
                resolve_block_size(block, sorted_sizes, &mut self.rng, &mut self.stream)?;
            let mut multiset = build_arm_multiset(&self.cfg.arms, block_size, self.ratio_sum)?;
            permute(&mut self.rng, &mut self.stream, &mut multiset)?;

            let keep = remaining.min(block_size);
            for position in 1..=keep {
                let idx =
                    usize::try_from(position.checked_sub(1).ok_or(GenerationError::Overflow)?)
                        .map_err(|_| GenerationError::Overflow)?;
                let arm_code = multiset.get(idx).ok_or(GenerationError::Overflow)?.clone();
                let number = randomization_number(
                    self.cfg,
                    stratum_index,
                    self.records.len(),
                    pos_in_stratum,
                )?;
                self.records.push(AllocationRecord {
                    randomization_number: number,
                    stratum: stratum.clone(),
                    block_id,
                    block_size,
                    position_in_block: position,
                    arm_code,
                });
                pos_in_stratum = pos_in_stratum
                    .checked_add(1)
                    .ok_or(GenerationError::Overflow)?;
            }

            remaining = remaining
                .checked_sub(keep)
                .ok_or(GenerationError::Overflow)?;
            block_id = block_id.checked_add(1).ok_or(GenerationError::Overflow)?;
        }

        Ok(())
    }
}

fn ratio_sum(arms: &[Arm]) -> Result<u32, GenerationError> {
    let mut sum = 0u32;
    for arm in arms {
        sum = sum
            .checked_add(arm.ratio)
            .ok_or(GenerationError::Overflow)?;
    }
    if sum == 0 {
        return Err(GenerationError::Overflow);
    }
    Ok(sum)
}

fn sorted_deduped_sizes(block: &BlockScheme) -> Option<Vec<u32>> {
    match block {
        BlockScheme::Fixed { .. } => None,
        BlockScheme::Variable { sizes } => {
            let mut sorted = sizes.clone();
            sorted.sort_unstable();
            sorted.dedup();
            Some(sorted)
        }
    }
}

fn arm_for_simple_draw(arms: &[Arm], draw: u64) -> Result<String, GenerationError> {
    let mut cumulative = 0u64;
    for arm in arms {
        let next = cumulative
            .checked_add(u64::from(arm.ratio))
            .ok_or(GenerationError::Overflow)?;
        if draw < next {
            return Ok(arm.code.clone());
        }
        cumulative = next;
    }
    Err(GenerationError::Overflow)
}

fn resolve_block_size(
    block: &BlockScheme,
    sorted_sizes: Option<&[u32]>,
    rng: &mut Rng,
    stream: &mut StreamLog,
) -> Result<u32, GenerationError> {
    match block {
        BlockScheme::Fixed { size } => Ok(*size),
        BlockScheme::Variable { .. } => {
            let sizes = sorted_sizes.ok_or(GenerationError::Overflow)?;
            let bound = u64::try_from(sizes.len()).map_err(|_| GenerationError::Overflow)?;
            let index = uniform_below(rng, stream, bound, DrawPurpose::BlockSize)?;
            let idx = usize::try_from(index).map_err(|_| GenerationError::Overflow)?;
            sizes.get(idx).copied().ok_or(GenerationError::Overflow)
        }
    }
}

/// Build the arm multiset for one block: for each arm in config order, append
/// `code` exactly `ratio * (block_size / ratio_sum)` times.
///
/// Rejects a non-multiple `block_size` / `ratio_sum` rather than silently
/// dropping a remainder. `validate_config` already rejects this for normal
/// configs; this is a defensive belt on the allocation path.
pub(crate) fn build_arm_multiset(
    arms: &[Arm],
    block_size: u32,
    ratio_sum: u32,
) -> Result<Vec<String>, GenerationError> {
    if ratio_sum == 0 {
        return Err(GenerationError::Overflow);
    }
    if !block_size.is_multiple_of(ratio_sum) {
        return Err(GenerationError::InconsistentConfig {
            block_size,
            ratio_sum,
        });
    }
    let unit = block_size / ratio_sum;
    let mut items = Vec::new();
    for arm in arms {
        let count = arm
            .ratio
            .checked_mul(unit)
            .ok_or(GenerationError::Overflow)?;
        for _ in 0..count {
            items.push(arm.code.clone());
        }
    }
    Ok(items)
}

fn randomization_number(
    cfg: &StudyConfig,
    stratum_index: usize,
    global_index: usize,
    pos_in_stratum: u32,
) -> Result<String, GenerationError> {
    let value = match cfg.numbering {
        NumberingScheme::Global { start, .. } => {
            let n = u32::try_from(global_index).map_err(|_| GenerationError::Overflow)?;
            start.checked_add(n).ok_or(GenerationError::Overflow)?
        }
        NumberingScheme::PerStratumRange {
            start, block_size, ..
        } => {
            let s = u32::try_from(stratum_index).map_err(|_| GenerationError::Overflow)?;
            let offset = s.checked_mul(block_size).ok_or(GenerationError::Overflow)?;
            let base = start.checked_add(offset).ok_or(GenerationError::Overflow)?;
            base.checked_add(pos_in_stratum)
                .ok_or(GenerationError::Overflow)?
        }
    };
    let width = match cfg.numbering {
        NumberingScheme::Global { width, .. } | NumberingScheme::PerStratumRange { width, .. } => {
            width
        }
    };
    Ok(format_number(value, width))
}

/// Zero-pad to `width` when the decimal fits; otherwise emit the full decimal
/// without truncating digits (decision 0005).
fn format_number(value: u32, width: u8) -> String {
    let raw = value.to_string();
    let w = usize::from(width);
    if raw.len() >= w {
        raw
    } else {
        format!("{value:0>width$}", width = w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn build_arm_multiset_rejects_non_multiple_block_size() {
        let err = build_arm_multiset(&arms_2_1(), 5, 3).expect_err("5 % 3 != 0");
        assert_eq!(
            err,
            GenerationError::InconsistentConfig {
                block_size: 5,
                ratio_sum: 3,
            }
        );
    }

    #[test]
    fn build_arm_multiset_config_order_contiguous_counts() {
        let items = build_arm_multiset(&arms_2_1(), 6, 3).expect("6 is multiple of 3");
        assert_eq!(items, vec!["A", "A", "A", "A", "P", "P"]);
    }
}
