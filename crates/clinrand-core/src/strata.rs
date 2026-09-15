//! Canonical stratum combination enumeration (plan §5.4).

use std::collections::BTreeMap;

use thiserror::Error;

use crate::config::StratificationFactor;

/// Failure enumerating stratum combinations.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StratumError {
    /// Product of factor level counts overflowed `usize`.
    #[error("stratum combination count overflowed")]
    Overflow,
}

/// Enumerate every stratum combination in canonical order.
///
/// Contract-bound: factors and levels appear in config array order; the
/// Cartesian product is enumerated with the **last factor varying fastest**.
/// An empty `factors` slice yields exactly one empty map. Map keys are
/// factor names; values are level strings. Must not change without an
/// `ALGO_VERSION` bump.
///
/// Combination count uses checked multiplication (same posture as
/// [`crate::validate_config`]). On overflow returns [`StratumError::Overflow`]
/// rather than wrapping. Validated configs keep the product ≤ 200, so the
/// allocation path should not hit this in normal use.
pub fn stratum_combinations(
    factors: &[StratificationFactor],
) -> Result<Vec<BTreeMap<String, String>>, StratumError> {
    if factors.is_empty() {
        return Ok(vec![BTreeMap::new()]);
    }

    let mut count = 1usize;
    for factor in factors {
        count = count
            .checked_mul(factor.levels.len())
            .ok_or(StratumError::Overflow)?;
    }

    let mut combinations = Vec::with_capacity(count);
    for combo_idx in 0..count {
        let mut map = BTreeMap::new();
        let mut remaining = combo_idx;
        for factor in factors.iter().rev() {
            let level_count = factor.levels.len();
            let level_idx = remaining % level_count;
            remaining /= level_count;
            map.insert(factor.name.clone(), factor.levels[level_idx].clone());
        }
        combinations.push(map);
    }

    Ok(combinations)
}
