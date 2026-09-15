//! Property suite (plan §8.2): arbitrary valid configs × generate × P01–P09.
//!
//! Minimum 1000 cases in CI. Asserts invariants via `check_properties`, not
//! correctness against an external oracle (see `validation/properties/README.md`).

use clinrand_core::{
    check_properties, generate, validate_config, Arm, BlockScheme, Method, NumberingScheme,
    StratificationFactor, StudyConfig, ValidateOptions, ALGO_VERSION,
};
use proptest::prelude::*;

/// Multiples of `ratio_sum` in `1..=24` (strategy bound; §5.2 caps variable at 24).
fn block_size_multiples(ratio_sum: u32) -> Vec<u32> {
    debug_assert!(ratio_sum > 0);
    let mut out = Vec::new();
    let mut k = 1u32;
    while let Some(size) = k.checked_mul(ratio_sum) {
        if size > 24 {
            break;
        }
        out.push(size);
        k = k.saturating_add(1);
        if k == 0 {
            break;
        }
    }
    out
}

fn arb_arms() -> impl Strategy<Value = Vec<Arm>> {
    (2usize..=4).prop_flat_map(|n| {
        prop::collection::vec(1u32..=3, n).prop_map(move |ratios| {
            ratios
                .into_iter()
                .enumerate()
                .map(|(i, ratio)| {
                    let code = char::from(b'A' + u8::try_from(i).expect("n<=4"));
                    Arm {
                        code: code.to_string(),
                        label: format!("Arm{code}"),
                        ratio,
                    }
                })
                .collect()
        })
    })
}

fn arb_block(ratio_sum: u32) -> impl Strategy<Value = BlockScheme> {
    let multiples = block_size_multiples(ratio_sum);
    assert!(
        !multiples.is_empty(),
        "ratio_sum {ratio_sum} must admit a block size ≤24"
    );
    let fixed =
        proptest::sample::select(multiples.clone()).prop_map(|size| BlockScheme::Fixed { size });
    let variable = (1usize..=multiples.len().min(4)).prop_flat_map(move |n| {
        Just(multiples.clone())
            .prop_shuffle()
            .prop_map(move |shuffled| BlockScheme::Variable {
                sizes: shuffled.into_iter().take(n).collect(),
            })
    });
    prop_oneof![fixed, variable]
}

fn arb_strata(
    min_factors: usize,
    max_factors: usize,
) -> impl Strategy<Value = Vec<StratificationFactor>> {
    (min_factors..=max_factors).prop_flat_map(|n_factors| {
        if n_factors == 0 {
            return Just(Vec::new()).boxed();
        }
        // 2–5 levels each; with ≤3 factors max product is 5³ = 125 ≤ 200.
        prop::collection::vec(2usize..=5, n_factors)
            .prop_map(|level_counts| {
                level_counts
                    .into_iter()
                    .enumerate()
                    .map(|(i, n_levels)| StratificationFactor {
                        name: format!("f{i}"),
                        levels: (0..n_levels).map(|j| format!("L{j}")).collect(),
                    })
                    .collect()
            })
            .boxed()
    })
}

fn arb_list_length() -> impl Strategy<Value = u32> {
    // Keep cases fast under 1000 runs; zero is covered but rare (~5%).
    prop_oneof![1 => Just(0u32), 19 => 1u32..=24]
}

fn arb_numbering(list_length: u32) -> impl Strategy<Value = NumberingScheme> {
    let global =
        (1u32..=1000, 1u8..=8).prop_map(|(start, width)| NumberingScheme::Global { start, width });
    // Range size must cover the per-stratum list so numbers never collide (P04).
    let range_block = list_length.max(1);
    let per_stratum =
        (1u32..=1000, Just(range_block), 1u8..=8).prop_map(|(start, block_size, width)| {
            NumberingScheme::PerStratumRange {
                start,
                block_size,
                width,
            }
        });
    prop_oneof![global, per_stratum]
}

fn arb_valid_config() -> impl Strategy<Value = StudyConfig> {
    (
        arb_arms(),
        prop_oneof![Just(0u8), Just(1u8), Just(2u8)], // method kind
        arb_list_length(),
    )
        .prop_flat_map(|(arms, method_kind, list_length)| {
            let ratio_sum: u32 = arms.iter().map(|a| a.ratio).sum();
            let numbering = arb_numbering(list_length);
            match method_kind {
                0 => (
                    Just(arms),
                    Just(Method::Simple),
                    arb_strata(0, 3),
                    Just(list_length),
                    numbering,
                )
                    .prop_map(
                        |(arms, method, strata, list_length, numbering)| StudyConfig {
                            schema_version: "1.0".into(),
                            study_id: "DEMO-501".into(),
                            protocol_version: "1.0".into(),
                            arms,
                            method,
                            strata,
                            list_length_per_stratum: list_length,
                            numbering,
                        },
                    )
                    .boxed(),
                1 => (
                    Just(arms),
                    arb_block(ratio_sum),
                    Just(list_length),
                    numbering,
                )
                    .prop_map(|(arms, block, list_length, numbering)| StudyConfig {
                        schema_version: "1.0".into(),
                        study_id: "TEST-501".into(),
                        protocol_version: "1.0".into(),
                        arms,
                        method: Method::PermutedBlock { block },
                        strata: vec![],
                        list_length_per_stratum: list_length,
                        numbering,
                    })
                    .boxed(),
                _ => (
                    Just(arms),
                    arb_block(ratio_sum),
                    arb_strata(1, 3),
                    Just(list_length),
                    numbering,
                )
                    .prop_map(
                        |(arms, block, strata, list_length, numbering)| StudyConfig {
                            schema_version: "1.0".into(),
                            study_id: "EXAMPLE-501".into(),
                            protocol_version: "1.0".into(),
                            arms,
                            method: Method::StratifiedBlock { block },
                            strata,
                            list_length_per_stratum: list_length,
                            numbering,
                        },
                    )
                    .boxed(),
            }
        })
        .prop_filter("validate_config accepts", |cfg| {
            validate_config(cfg, &ValidateOptions::default()).is_ok()
        })
}

fn arb_seed() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        // Integration tests have no crate root for SourceParallel persistence.
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::Off,
        )),
        ..ProptestConfig::default()
    })]

    /// P01–P09 hold for every generated list from an arbitrary valid config.
    ///
    /// Uses [`PropertyReport::all_required_passed`]: informational P10 is ignored,
    /// and P09 for `Method::Simple` is treated as pass (controller ruling).
    #[test]
    fn generated_lists_satisfy_required_properties(
        cfg in arb_valid_config(),
        seed in arb_seed(),
    ) {
        prop_assert_eq!(ALGO_VERSION, 1);

        let list = generate(&cfg, seed).expect("strategy must only yield configs generate accepts");
        let report = check_properties(&list, &cfg);
        prop_assert!(
            report.all_required_passed(),
            "required property check(s) failed for {:?}: {:?}",
            cfg.method,
            report
                .checks
                .iter()
                .filter(|c| !c.informational && !c.passed)
                .map(|c| format!("{}: {}", c.id, c.detail))
                .collect::<Vec<_>>()
        );
    }

    /// `generate(cfg, seed)` is a pure function of its inputs.
    #[test]
    fn generate_twice_identical_records_and_stream(
        cfg in arb_valid_config(),
        seed in arb_seed(),
    ) {
        let a = generate(&cfg, seed).expect("generate a");
        let b = generate(&cfg, seed).expect("generate b");
        prop_assert_eq!(&a.records, &b.records);
        prop_assert_eq!(&a.stream, &b.stream);
    }
}
