//! Canonical stratum Cartesian product (plan §5.4 / determinism.md §6).
//!
//! Expected sequences are hand-worked from the contract: config order,
//! last factor varies fastest. They are not this engine's own output.

use std::collections::BTreeMap;

use clinrand_core::{stratum_combinations, StratificationFactor};

fn combo(factors: &[(&str, &[&str])]) -> Vec<BTreeMap<String, String>> {
    let strata: Vec<StratificationFactor> = factors
        .iter()
        .map(|(name, levels)| StratificationFactor {
            name: (*name).into(),
            levels: levels.iter().map(|l| (*l).into()).collect(),
        })
        .collect();
    stratum_combinations(&strata)
}

fn as_pairs(map: &BTreeMap<String, String>) -> Vec<(&str, &str)> {
    map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

#[test]
fn empty_factors_yield_one_empty_map() {
    let combos = combo(&[]);
    assert_eq!(combos.len(), 1);
    assert!(combos[0].is_empty());
}

#[test]
fn one_factor_levels_in_config_order() {
    let combos = combo(&[("arm", &["A", "B"])]);
    assert_eq!(combos.len(), 2);
    assert_eq!(as_pairs(&combos[0]), vec![("arm", "A")]);
    assert_eq!(as_pairs(&combos[1]), vec![("arm", "B")]);
}

#[test]
fn demo_201_site_by_agegrp_last_factor_fastest() {
    let combos = combo(&[
        ("site", &["001", "002", "003"]),
        ("agegrp", &["LT65", "GE65"]),
    ]);
    assert_eq!(combos.len(), 6);

    let expected = [
        [("agegrp", "LT65"), ("site", "001")],
        [("agegrp", "GE65"), ("site", "001")],
        [("agegrp", "LT65"), ("site", "002")],
        [("agegrp", "GE65"), ("site", "002")],
        [("agegrp", "LT65"), ("site", "003")],
        [("agegrp", "GE65"), ("site", "003")],
    ];

    for (combo, want) in combos.iter().zip(expected) {
        assert_eq!(as_pairs(combo), want);
    }
}

#[test]
fn three_factors_last_factor_varies_fastest() {
    let combos = combo(&[
        ("a", &["X", "Y"]),
        ("b", &["1", "2"]),
        ("c", &["p", "q", "r"]),
    ]);
    assert_eq!(combos.len(), 12);

    let expected = [
        [("a", "X"), ("b", "1"), ("c", "p")],
        [("a", "X"), ("b", "1"), ("c", "q")],
        [("a", "X"), ("b", "1"), ("c", "r")],
        [("a", "X"), ("b", "2"), ("c", "p")],
        [("a", "X"), ("b", "2"), ("c", "q")],
        [("a", "X"), ("b", "2"), ("c", "r")],
        [("a", "Y"), ("b", "1"), ("c", "p")],
        [("a", "Y"), ("b", "1"), ("c", "q")],
        [("a", "Y"), ("b", "1"), ("c", "r")],
        [("a", "Y"), ("b", "2"), ("c", "p")],
        [("a", "Y"), ("b", "2"), ("c", "q")],
        [("a", "Y"), ("b", "2"), ("c", "r")],
    ];

    for (combo, want) in combos.iter().zip(expected) {
        assert_eq!(as_pairs(combo), want);
    }
}
