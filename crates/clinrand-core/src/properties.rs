//! Property checks P01–P10 on a generated list (plan §7).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::config::{BlockScheme, Method, NumberingScheme, StudyConfig};
use crate::generate::{AllocationRecord, GeneratedList};
use crate::strata::stratum_combinations;

/// Outcome of [`check_properties`] for one list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyReport {
    /// Results in order P01…P10.
    pub checks: Vec<PropertyCheck>,
}

impl PropertyReport {
    /// True when every non-informational check passed (P01–P09).
    pub fn all_required_passed(&self) -> bool {
        self.checks
            .iter()
            .filter(|c| !c.informational)
            .all(|c| c.passed)
    }
}

/// One property check result, suitable for embedding in HTML reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyCheck {
    /// Check identifier (`"P01"` … `"P10"`).
    pub id: &'static str,
    /// Whether the check passed. P10 is always `true`.
    pub passed: bool,
    /// When true, the check is informational and must not fail a generation.
    pub informational: bool,
    /// Human-readable detail for reports.
    pub detail: String,
}

/// Run property checks P01–P10 against `list` using `cfg`.
///
/// Does not mutate inputs and performs no I/O. P10 is always informational
/// and always reports `passed: true`.
pub fn check_properties(list: &GeneratedList, cfg: &StudyConfig) -> PropertyReport {
    let expected_strata = stratum_combinations(&cfg.strata);
    let n_strata = expected_strata.len();
    let ratio_sum = cfg.arms.iter().map(|a| a.ratio).sum::<u32>();
    let max_block = max_configured_block_size(cfg);

    let checks = vec![
        check_p01(list, cfg, n_strata),
        check_p02(list, cfg),
        check_p03(list, cfg, ratio_sum),
        check_p04(list),
        check_p05(list, cfg, &expected_strata),
        check_p06(list, cfg, &expected_strata),
        check_p07(list, cfg, &expected_strata),
        check_p08(list),
        check_p09(list, cfg, ratio_sum, max_block, &expected_strata),
        check_p10(list, &expected_strata),
    ];
    PropertyReport { checks }
}

fn ok(id: &'static str, detail: impl Into<String>) -> PropertyCheck {
    PropertyCheck {
        id,
        passed: true,
        informational: false,
        detail: detail.into(),
    }
}

fn fail(id: &'static str, detail: impl Into<String>) -> PropertyCheck {
    PropertyCheck {
        id,
        passed: false,
        informational: false,
        detail: detail.into(),
    }
}

fn max_configured_block_size(cfg: &StudyConfig) -> u32 {
    match &cfg.method {
        Method::Simple => 1,
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => match block {
            BlockScheme::Fixed { size } => *size,
            BlockScheme::Variable { sizes } => sizes.iter().copied().max().unwrap_or(0),
        },
    }
}

fn allowed_block_sizes(cfg: &StudyConfig) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    match &cfg.method {
        Method::Simple => {
            set.insert(1);
        }
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => match block {
            BlockScheme::Fixed { size } => {
                set.insert(*size);
            }
            BlockScheme::Variable { sizes } => {
                set.extend(sizes.iter().copied());
            }
        },
    }
    set
}

/// P01: record count == list_length_per_stratum × n_strata.
fn check_p01(list: &GeneratedList, cfg: &StudyConfig, n_strata: usize) -> PropertyCheck {
    let expected = match (cfg.list_length_per_stratum as u64).checked_mul(n_strata as u64) {
        Some(v) => v,
        None => return fail("P01", "overflow computing expected record count"),
    };
    let actual = list.records.len() as u64;
    if actual == expected {
        ok(
            "P01",
            format!("record count {actual} equals list_length_per_stratum × n_strata ({expected})"),
        )
    } else {
        fail(
            "P01",
            format!(
                "record count {actual} != expected {expected} (list_length_per_stratum × n_strata)"
            ),
        )
    }
}

/// P02: every block_size appears in the configured allowed set.
fn check_p02(list: &GeneratedList, cfg: &StudyConfig) -> PropertyCheck {
    let allowed = allowed_block_sizes(cfg);
    for (i, rec) in list.records.iter().enumerate() {
        if !allowed.contains(&rec.block_size) {
            return fail(
                "P02",
                format!(
                    "record {i} has block_size {} not in allowed set {:?}",
                    rec.block_size, allowed
                ),
            );
        }
    }
    ok(
        "P02",
        format!("all block sizes are in the allowed set {:?}", allowed),
    )
}

/// Group key: stratum map + block_id (blocks are numbered per stratum).
fn block_groups(
    records: &[AllocationRecord],
) -> BTreeMap<(BTreeMap<String, String>, u32), Vec<&AllocationRecord>> {
    let mut groups: BTreeMap<(BTreeMap<String, String>, u32), Vec<&AllocationRecord>> =
        BTreeMap::new();
    for rec in records {
        groups
            .entry((rec.stratum.clone(), rec.block_id))
            .or_default()
            .push(rec);
    }
    groups
}

/// P03: every complete (non-truncated) block has exact ratio counts.
/// Truncated final blocks (kept < block_size) are skipped.
/// Simple size-1 blocks are not ratio-bearing units when block_size is not a
/// multiple of the ratio sum — those blocks are skipped.
fn check_p03(list: &GeneratedList, cfg: &StudyConfig, ratio_sum: u32) -> PropertyCheck {
    if ratio_sum == 0 {
        return fail("P03", "ratio sum is zero");
    }
    let groups = block_groups(&list.records);
    let mut checked = 0u32;
    for ((stratum, block_id), members) in &groups {
        let Some(first) = members.first() else {
            continue;
        };
        let block_size = first.block_size;
        // Truncated: fewer kept positions than block_size.
        if (members.len() as u32) < block_size {
            continue;
        }
        if (members.len() as u32) > block_size {
            return fail(
                "P03",
                format!(
                    "stratum {:?}/block {block_id}: kept {} exceeds block_size {block_size}",
                    stratum,
                    members.len()
                ),
            );
        }
        // Not a ratio-complete block size (e.g. simple size-1 with ratio sum > 1).
        if !block_size.is_multiple_of(ratio_sum) {
            continue;
        }
        let unit = block_size / ratio_sum;
        let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
        for rec in members {
            *counts.entry(rec.arm_code.as_str()).or_insert(0) += 1;
        }
        for arm in &cfg.arms {
            let expected = match arm.ratio.checked_mul(unit) {
                Some(v) => v,
                None => return fail("P03", "overflow computing expected arm count in block"),
            };
            let actual = counts.get(arm.code.as_str()).copied().unwrap_or(0);
            if actual != expected {
                return fail(
                    "P03",
                    format!(
                        "stratum {:?}/block {block_id}: arm {} count {actual} != expected {expected}",
                        stratum, arm.code
                    ),
                );
            }
        }
        // No unexpected arm codes inside the block.
        for code in counts.keys() {
            if !cfg.arms.iter().any(|a| a.code == *code) {
                return fail(
                    "P03",
                    format!(
                        "stratum {:?}/block {block_id}: unexpected arm code {code}",
                        stratum
                    ),
                );
            }
        }
        checked = checked.saturating_add(1);
    }
    ok(
        "P03",
        format!("checked {checked} complete ratio-bearing block(s); truncated finals skipped"),
    )
}

/// P04: no duplicate randomization numbers.
fn check_p04(list: &GeneratedList) -> PropertyCheck {
    let mut seen = HashSet::new();
    for rec in &list.records {
        if !seen.insert(rec.randomization_number.as_str()) {
            return fail(
                "P04",
                format!(
                    "duplicate randomization number {}",
                    rec.randomization_number
                ),
            );
        }
    }
    ok(
        "P04",
        format!(
            "no duplicate randomization numbers among {} records",
            list.records.len()
        ),
    )
}

fn parse_number(s: &str) -> Option<u32> {
    s.parse::<u32>().ok()
}

/// P05: numbers contiguous and ascending within the numbering scheme.
fn check_p05(
    list: &GeneratedList,
    cfg: &StudyConfig,
    expected_strata: &[BTreeMap<String, String>],
) -> PropertyCheck {
    match cfg.numbering {
        NumberingScheme::Global { start, .. } => {
            for (i, rec) in list.records.iter().enumerate() {
                let Some(n) = parse_number(&rec.randomization_number) else {
                    return fail(
                        "P05",
                        format!(
                            "randomization number {:?} is not a decimal u32",
                            rec.randomization_number
                        ),
                    );
                };
                let expected = match u32::try_from(i).ok().and_then(|i| start.checked_add(i)) {
                    Some(v) => v,
                    None => return fail("P05", "overflow computing expected global number"),
                };
                if n != expected {
                    return fail(
                        "P05",
                        format!(
                            "global numbering: record {i} has {n}, expected {expected} (contiguous ascending from {start})"
                        ),
                    );
                }
            }
            ok(
                "P05",
                format!(
                    "global numbers contiguous and ascending from {start} for {} records",
                    list.records.len()
                ),
            )
        }
        NumberingScheme::PerStratumRange {
            start, block_size, ..
        } => {
            for (stratum_index, stratum) in expected_strata.iter().enumerate() {
                let members: Vec<&AllocationRecord> = list
                    .records
                    .iter()
                    .filter(|r| &r.stratum == stratum)
                    .collect();
                let s = match u32::try_from(stratum_index) {
                    Ok(v) => v,
                    Err(_) => return fail("P05", "stratum index does not fit u32"),
                };
                let base = match s
                    .checked_mul(block_size)
                    .and_then(|off| start.checked_add(off))
                {
                    Some(v) => v,
                    None => return fail("P05", "overflow computing per-stratum range start"),
                };
                for (pos, rec) in members.iter().enumerate() {
                    let Some(n) = parse_number(&rec.randomization_number) else {
                        return fail(
                            "P05",
                            format!(
                                "randomization number {:?} is not a decimal u32",
                                rec.randomization_number
                            ),
                        );
                    };
                    let expected = match u32::try_from(pos).ok().and_then(|p| base.checked_add(p)) {
                        Some(v) => v,
                        None => {
                            return fail("P05", "overflow computing expected per-stratum number")
                        }
                    };
                    if n != expected {
                        return fail(
                            "P05",
                            format!(
                                "per_stratum_range: stratum {:?}/pos {pos} has {n}, expected {expected}",
                                stratum
                            ),
                        );
                    }
                }
            }
            ok(
                "P05",
                "per_stratum_range numbers contiguous and ascending within each stratum",
            )
        }
    }
}

/// P06: every Cartesian stratum combination is present (when length > 0).
fn check_p06(
    list: &GeneratedList,
    cfg: &StudyConfig,
    expected_strata: &[BTreeMap<String, String>],
) -> PropertyCheck {
    if cfg.list_length_per_stratum == 0 {
        return ok(
            "P06",
            "list_length_per_stratum is 0; no stratum presence required",
        );
    }
    let present: HashSet<&BTreeMap<String, String>> =
        list.records.iter().map(|r| &r.stratum).collect();
    for stratum in expected_strata {
        if !present.contains(stratum) {
            return fail("P06", format!("missing stratum combination {:?}", stratum));
        }
    }
    ok(
        "P06",
        format!(
            "all {} stratum combination(s) are present",
            expected_strata.len()
        ),
    )
}

/// P07: records are emitted in canonical stratum order; each stratum's segment
/// of `list_length_per_stratum` records must all carry that stratum's map
/// (no cross-stratum contamination / invented combinations).
fn check_p07(
    list: &GeneratedList,
    cfg: &StudyConfig,
    expected_strata: &[BTreeMap<String, String>],
) -> PropertyCheck {
    let allowed: HashSet<&BTreeMap<String, String>> = expected_strata.iter().collect();
    for (i, rec) in list.records.iter().enumerate() {
        if !allowed.contains(&rec.stratum) {
            return fail(
                "P07",
                format!(
                    "record {i} stratum {:?} is not a configured combination",
                    rec.stratum
                ),
            );
        }
    }

    let length = cfg.list_length_per_stratum as usize;
    if length == 0 {
        return ok("P07", "list_length_per_stratum is 0; no segment check");
    }

    for (stratum_index, expected) in expected_strata.iter().enumerate() {
        let start = match stratum_index.checked_mul(length) {
            Some(v) => v,
            None => return fail("P07", "overflow computing stratum segment start"),
        };
        let end = match start.checked_add(length) {
            Some(v) => v,
            None => return fail("P07", "overflow computing stratum segment end"),
        };
        if list.records.len() < end {
            // P01 covers count; avoid duplicate noise when truncated short.
            break;
        }
        for (offset, rec) in list.records[start..end].iter().enumerate() {
            if &rec.stratum != expected {
                return fail(
                    "P07",
                    format!(
                        "stratum segment {stratum_index} position {offset}: found {:?} expected {:?} (cross-stratum contamination)",
                        rec.stratum, expected
                    ),
                );
            }
        }
    }

    ok(
        "P07",
        "each canonical stratum segment contains only that stratum's records",
    )
}

/// P08: position_in_block is 1..kept complete without gaps for every block.
fn check_p08(list: &GeneratedList) -> PropertyCheck {
    let groups = block_groups(&list.records);
    for ((stratum, block_id), members) in &groups {
        let kept = members.len() as u32;
        let mut positions: Vec<u32> = members.iter().map(|r| r.position_in_block).collect();
        positions.sort_unstable();
        let expected: Vec<u32> = (1..=kept).collect();
        if positions != expected {
            return fail(
                "P08",
                format!(
                    "stratum {:?}/block {block_id}: positions {:?} != expected 1..{kept}",
                    stratum, positions
                ),
            );
        }
        // Also: no position exceeds the declared block_size.
        if let Some(first) = members.first() {
            if kept > first.block_size {
                return fail(
                    "P08",
                    format!(
                        "stratum {:?}/block {block_id}: kept {kept} > block_size {}",
                        stratum, first.block_size
                    ),
                );
            }
            for rec in members {
                if rec.position_in_block == 0 || rec.position_in_block > first.block_size {
                    return fail(
                        "P08",
                        format!(
                            "stratum {:?}/block {block_id}: position_in_block {} outside 1..{}",
                            stratum, rec.position_in_block, first.block_size
                        ),
                    );
                }
            }
        }
    }
    ok(
        "P08",
        "position_in_block is 1..kept without gaps for every block",
    )
}

/// P09: overall (per-stratum) arm counts match ratio within one block's tolerance.
///
/// Integer form: for each arm i in each stratum,
/// `|count_i * R - n * ratio_i| <= max_block * R`.
///
/// For `Method::Simple`, size-1 "blocks" are not ratio-bearing; the check
/// always passes with an explanatory detail (no hard balance guarantee).
fn check_p09(
    list: &GeneratedList,
    cfg: &StudyConfig,
    ratio_sum: u32,
    max_block: u32,
    expected_strata: &[BTreeMap<String, String>],
) -> PropertyCheck {
    if matches!(cfg.method, Method::Simple) {
        return ok(
            "P09",
            "simple randomization has no block-balance guarantee; P09 not applied",
        );
    }
    if ratio_sum == 0 {
        return fail("P09", "ratio sum is zero");
    }
    // Tolerance width in count-space: max_block.
    // |c*R - n*r| <= max_block * R
    for stratum in expected_strata {
        let members: Vec<&AllocationRecord> = list
            .records
            .iter()
            .filter(|r| &r.stratum == stratum)
            .collect();
        let n = members.len() as u32;
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for rec in &members {
            *counts.entry(rec.arm_code.as_str()).or_insert(0) += 1;
        }
        for arm in &cfg.arms {
            let count = counts.get(arm.code.as_str()).copied().unwrap_or(0);
            let left = match count
                .checked_mul(ratio_sum)
                .and_then(|c| n.checked_mul(arm.ratio).map(|nr| c.abs_diff(nr)))
            {
                Some(v) => v,
                None => return fail("P09", "overflow computing arm-count deviation"),
            };
            let limit = match max_block.checked_mul(ratio_sum) {
                Some(v) => v,
                None => return fail("P09", "overflow computing P09 tolerance"),
            };
            if left > limit {
                return fail(
                    "P09",
                    format!(
                        "stratum {:?}: arm {} count {count} of {n} exceeds one-block tolerance (max_block={max_block}, ratio_sum={ratio_sum})",
                        stratum, arm.code
                    ),
                );
            }
        }
    }
    ok(
        "P09",
        format!("per-stratum arm counts within one-block tolerance (max_block={max_block})"),
    )
}

/// P10: max run length of identical consecutive arms per stratum — informational only.
fn check_p10(list: &GeneratedList, expected_strata: &[BTreeMap<String, String>]) -> PropertyCheck {
    let mut parts = Vec::new();
    let mut global_max = 0u32;
    for stratum in expected_strata {
        let members: Vec<&AllocationRecord> = list
            .records
            .iter()
            .filter(|r| &r.stratum == stratum)
            .collect();
        let mut max_run = 0u32;
        let mut cur_run = 0u32;
        let mut prev: Option<&str> = None;
        for rec in members {
            match prev {
                Some(p) if p == rec.arm_code.as_str() => {
                    cur_run = cur_run.saturating_add(1);
                }
                _ => {
                    cur_run = 1;
                    prev = Some(rec.arm_code.as_str());
                }
            }
            if cur_run > max_run {
                max_run = cur_run;
            }
        }
        if max_run > global_max {
            global_max = max_run;
        }
        parts.push(format!("stratum {:?}: max_run={max_run}", stratum));
    }
    PropertyCheck {
        id: "P10",
        passed: true,
        informational: true,
        detail: format!(
            "informational only (not a failure): maximum identical consecutive arm run per stratum — {}; global_max={global_max}. Long runs can prompt methodological discussion but are legitimate outcomes of correct randomization.",
            parts.join("; ")
        ),
    }
}
