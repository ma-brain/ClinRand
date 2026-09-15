//! Canonical stratum combination enumeration (plan §5.4).

use std::collections::BTreeMap;

use crate::config::StratificationFactor;

/// Enumerate every stratum combination in canonical order.
///
/// Contract-bound: factors and levels appear in config array order; the
/// Cartesian product is enumerated with the **last factor varying fastest**.
/// An empty `factors` slice yields exactly one empty map. Map keys are
/// factor names; values are level strings. Must not change without an
/// `ALGO_VERSION` bump.
pub fn stratum_combinations(factors: &[StratificationFactor]) -> Vec<BTreeMap<String, String>> {
    if factors.is_empty() {
        return vec![BTreeMap::new()];
    }

    let mut count = 1usize;
    for factor in factors {
        count *= factor.levels.len();
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

    combinations
}
