//! `validation-report` subcommand — in-process validation tier checks.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use clinrand_core::{
    check_properties, generate, permute, uniform_below, Arm, BlockScheme, DrawPurpose, Method,
    NumberingScheme, Rng, StratificationFactor, StreamDraw, StreamLog, StudyConfig, U64Draw,
    UniformError, ValidateOptions,
};
use clinrand_package::{check_regression_case, load_regression_cases, RegressionOutcome};

use crate::exit::ExitCode;
use crate::output::write_stdout;

const CHACHA20_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/chacha20/cases.json"
));
const UNIFORM_BELOW_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/uniform-below/cases.json"
));
const FISHER_YATES_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/fisher-yates/cases.json"
));
const REGRESSION_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../validation/regression");

const REFERENCE_EVIDENCE: &str = "Correctness against external normative sources (RFC 8439, hand-worked derivations). Expected values are not this engine's own output.";
const PROPERTIES_EVIDENCE: &str = "Invariant evidence only — not external-oracle correctness. A passing sweep means P01–P09 held for sampled configs; it does not prove the algorithm matches an independent reference. Full 1000-case CI suite: `cargo test -p clinrand-core --test properties_proptest`.";
const REGRESSION_EVIDENCE: &str = "Frozen engine output consistency — proves nothing changed since the last approved ALGO_VERSION, not that the algorithm is correct. See validation/regression/README.md.";

const PROPERTIES_SWEEP_CASES: u32 = 100;
const PROPERTIES_RNG_SEED: [u8; 32] = [0x50; 32];

/// Report output format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReportFormat {
    Markdown,
    Html,
}

/// Validation tier filter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TierFilter {
    All,
    Reference,
    Properties,
    Regression,
}

/// Overall tier outcome for exit-code mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TierOutcome {
    Pass,
    Fail,
    Skip,
}

/// One case line in a tier report.
#[derive(Clone, Debug)]
struct CaseLine {
    label: String,
    outcome: TierOutcome,
    detail: Option<String>,
}

/// One tier section in the report.
#[derive(Clone, Debug)]
struct TierSection {
    name: &'static str,
    evidence: &'static str,
    outcome: TierOutcome,
    summary: String,
    groups: Vec<CaseGroup>,
}

/// Primitive or logical grouping within a tier.
#[derive(Clone, Debug)]
struct CaseGroup {
    name: String,
    lines: Vec<CaseLine>,
}

/// Full validation report payload.
#[derive(Clone, Debug)]
struct ValidationReport {
    tiers: Vec<TierSection>,
}

/// Run validation-tier checks and emit a report to stdout.
pub fn run(json: bool, tier: Option<&str>, format: Option<&str>) -> ExitCode {
    let tier_filter = match parse_tier(tier) {
        Ok(filter) => filter,
        Err(message) => {
            let _ = crate::output::write_stderr(&format!("{message}\n"));
            return ExitCode::InvalidConfig;
        }
    };
    let report_format = match parse_format(format) {
        Ok(fmt) => fmt,
        Err(message) => {
            let _ = crate::output::write_stderr(&format!("{message}\n"));
            return ExitCode::InvalidConfig;
        }
    };

    let report = build_report(tier_filter);
    let exit = report_exit_code(&report);

    if json {
        if emit_json(&report).is_err() {
            return ExitCode::IoError;
        }
    } else if emit_human(&report, report_format).is_err() {
        return ExitCode::IoError;
    }

    exit
}

fn parse_tier(tier: Option<&str>) -> Result<TierFilter, String> {
    match tier {
        None | Some("all") => Ok(TierFilter::All),
        Some("reference") => Ok(TierFilter::Reference),
        Some("properties") => Ok(TierFilter::Properties),
        Some("regression") => Ok(TierFilter::Regression),
        Some(other) => Err(format!(
            "unknown validation tier {other:?}; expected reference, properties, regression, or all"
        )),
    }
}

fn parse_format(format: Option<&str>) -> Result<ReportFormat, String> {
    match format.unwrap_or("md") {
        "md" => Ok(ReportFormat::Markdown),
        "html" => Ok(ReportFormat::Html),
        other => Err(format!(
            "unknown report format {other:?}; expected md or html"
        )),
    }
}

fn build_report(filter: TierFilter) -> ValidationReport {
    let mut tiers = Vec::new();
    if matches!(filter, TierFilter::All | TierFilter::Reference) {
        tiers.push(run_reference_tier());
    }
    if matches!(filter, TierFilter::All | TierFilter::Properties) {
        tiers.push(run_properties_tier());
    }
    if matches!(filter, TierFilter::All | TierFilter::Regression) {
        tiers.push(run_regression_tier());
    }
    ValidationReport { tiers }
}

fn report_exit_code(report: &ValidationReport) -> ExitCode {
    let any_fail = report
        .tiers
        .iter()
        .any(|tier| tier.outcome == TierOutcome::Fail);
    if any_fail {
        ExitCode::CheckFailure
    } else {
        ExitCode::Success
    }
}

fn run_reference_tier() -> TierSection {
    let chacha = reference_group("chacha20", CHACHA20_CASES, run_chacha20_case);
    let uniform = reference_group("uniform-below", UNIFORM_BELOW_CASES, run_uniform_below_case);
    let fisher = reference_group("fisher-yates", FISHER_YATES_CASES, run_fisher_yates_case);
    let groups = vec![chacha, uniform, fisher];
    let outcome = tier_outcome_from_groups(&groups);
    let passed = count_passed(&groups);
    let total = count_total(&groups);
    TierSection {
        name: "Reference",
        evidence: REFERENCE_EVIDENCE,
        outcome,
        summary: format!("{passed}/{total} reference cases passed"),
        groups,
    }
}

fn reference_group(
    primitive: &str,
    cases_json: &str,
    runner: fn(&str, &str) -> Result<(), String>,
) -> CaseGroup {
    let mut lines = Vec::new();
    for case_id in case_ids(cases_json) {
        let result = runner(cases_json, &case_id);
        lines.push(CaseLine {
            label: case_id,
            outcome: if result.is_ok() {
                TierOutcome::Pass
            } else {
                TierOutcome::Fail
            },
            detail: result.err(),
        });
    }
    CaseGroup {
        name: primitive.to_string(),
        lines,
    }
}

fn run_chacha20_case(cases_json: &str, case_id: &str) -> Result<(), String> {
    let object = case_object(cases_json, case_id)?;
    let key = hex_array::<32>(&json_str(object, "key")?)?;
    let nonce = hex_array::<12>(&json_str(object, "nonce")?)?;
    let block_counter = json_u32(object, "blockCounter")?;
    let expected = decode_hex(&json_str(object, "keystream")?)?;

    let mut rng = Rng::from_ietf(key, nonce, block_counter);
    let mut got = vec![0u8; expected.len()];
    rng.fill_bytes(&mut got);
    if got == expected {
        Ok(())
    } else {
        Err("keystream mismatch".into())
    }
}

fn run_uniform_below_case(cases_json: &str, case_id: &str) -> Result<(), String> {
    let object = case_object(cases_json, case_id)?;
    let input = json_object(object, "input")?;
    let expect = json_object(object, "expect")?;
    let n = json_u64(input, "n")?;
    let purpose = parse_purpose(&json_ident(input, "purpose")?)?;
    let words = json_u64_strings(input, "keystream")?;
    let mut stream = DocumentedStream { words, pos: 0 };
    let mut log = StreamLog::default();
    let result = uniform_below(&mut stream, &mut log, n, purpose);

    let consumed = json_u64(expect, "consumed")?;
    if u64::try_from(stream.pos).map_err(|_| "pos overflow".to_string())? != consumed {
        return Err(format!(
            "consumed {case_id}: expected {consumed}, got {}",
            stream.pos
        ));
    }
    let expected_stream = json_stream(expect)?;
    if log.draws != expected_stream {
        return Err(format!("stream log mismatch for {case_id}"));
    }

    if let Some(code) = json_optional_str(expect, "error")? {
        if code != "zero_bound" {
            return Err(format!("unknown error code {code}"));
        }
        return match result {
            Err(UniformError::ZeroBound) => Ok(()),
            Err(other) => Err(format!("unexpected error: {other}")),
            Ok(value) => Err(format!("expected error, got value {value}")),
        };
    }

    let value = result.map_err(|e| format!("expected value: {e}"))?;
    if value != json_u64(expect, "value")? {
        return Err(format!("value mismatch for {case_id}"));
    }
    Ok(())
}

fn run_fisher_yates_case(cases_json: &str, case_id: &str) -> Result<(), String> {
    let object = case_object(cases_json, case_id)?;
    let input = json_object(object, "input")?;
    let expect = json_object(object, "expect")?;
    let mut items = json_i64_array(input, "items")?;
    let words = json_u64_strings(input, "keystream")?;
    let mut stream = DocumentedStream { words, pos: 0 };
    let mut log = StreamLog::default();

    permute(&mut stream, &mut log, &mut items).map_err(|e| format!("permute failed: {e}"))?;

    let consumed = json_u64(expect, "consumed")?;
    if u64::try_from(stream.pos).map_err(|_| "pos overflow".to_string())? != consumed {
        return Err(format!("consumed mismatch for {case_id}"));
    }
    if items != json_i64_array(expect, "items")? {
        return Err(format!("item order mismatch for {case_id}"));
    }
    if log.draws != json_stream(expect)? {
        return Err(format!("stream log mismatch for {case_id}"));
    }
    Ok(())
}

fn run_properties_tier() -> TierSection {
    let mut rng = Rng::from_seed(PROPERTIES_RNG_SEED);
    let mut lines = Vec::with_capacity(PROPERTIES_SWEEP_CASES as usize);

    for index in 0..PROPERTIES_SWEEP_CASES {
        let label = format!("property-case-{index}");
        let cfg = property_config(index);
        if clinrand_core::validate_config(&cfg, &ValidateOptions::default()).is_err() {
            lines.push(CaseLine {
                label,
                outcome: TierOutcome::Fail,
                detail: Some("generated config failed validate_config".into()),
            });
            continue;
        }

        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        match generate(&cfg, seed) {
            Ok(list) => {
                let report = check_properties(&list, &cfg);
                if report.all_required_passed() {
                    lines.push(CaseLine {
                        label,
                        outcome: TierOutcome::Pass,
                        detail: None,
                    });
                } else {
                    let detail = report
                        .checks
                        .iter()
                        .filter(|c| !c.informational && !c.passed)
                        .map(|c| format!("{}: {}", c.id, c.detail))
                        .collect::<Vec<_>>()
                        .join("; ");
                    lines.push(CaseLine {
                        label,
                        outcome: TierOutcome::Fail,
                        detail: Some(detail),
                    });
                }
            }
            Err(err) => lines.push(CaseLine {
                label,
                outcome: TierOutcome::Fail,
                detail: Some(format!("generate failed: {err}")),
            }),
        }
    }

    let passed = lines
        .iter()
        .filter(|l| l.outcome == TierOutcome::Pass)
        .count();
    let outcome = if passed == lines.len() {
        TierOutcome::Pass
    } else {
        TierOutcome::Fail
    };

    TierSection {
        name: "Properties",
        evidence: PROPERTIES_EVIDENCE,
        outcome,
        summary: format!(
            "{passed}/{PROPERTIES_SWEEP_CASES} bounded property cases passed (full CI suite: 1000 cases)"
        ),
        groups: vec![CaseGroup {
            name: "invariant-sweep".into(),
            lines,
        }],
    }
}

fn property_config(index: u32) -> StudyConfig {
    let kind = index % 3;
    let list_length = (index % 20).saturating_add(1);
    let arm_count = 2usize.saturating_add((index as usize) % 3);

    let arms: Vec<Arm> = (0..arm_count)
        .map(|i| {
            let ratio = 1u32.saturating_add((index as usize).saturating_add(i) as u32 % 3);
            let code = ['A', 'B', 'C', 'D']
                .get(i)
                .copied()
                .unwrap_or('A')
                .to_string();
            Arm {
                code: code.clone(),
                label: format!("Arm{code}"),
                ratio,
            }
        })
        .collect();

    let ratio_sum: u32 = arms.iter().map(|a| a.ratio).sum();
    let block_sizes: Vec<u32> = (1..=24).filter(|size| size % ratio_sum == 0).collect();
    let block_size = block_sizes
        .get((index as usize) % block_sizes.len().max(1))
        .copied()
        .unwrap_or(ratio_sum);

    let numbering = NumberingScheme::Global {
        start: 1_000u32.saturating_add(index),
        width: 5,
    };

    match kind {
        0 => StudyConfig {
            schema_version: "1.0".into(),
            study_id: "DEMO-501".into(),
            protocol_version: "1.0".into(),
            arms,
            method: Method::Simple,
            strata: vec![],
            list_length_per_stratum: list_length,
            numbering,
        },
        1 => StudyConfig {
            schema_version: "1.0".into(),
            study_id: "TEST-501".into(),
            protocol_version: "1.0".into(),
            arms,
            method: Method::PermutedBlock {
                block: BlockScheme::Fixed { size: block_size },
            },
            strata: vec![],
            list_length_per_stratum: list_length,
            numbering,
        },
        _ => {
            let n_factors = 1usize.saturating_add((index as usize) % 2);
            let strata: Vec<StratificationFactor> = (0..n_factors)
                .map(|f| StratificationFactor {
                    name: format!("f{f}"),
                    levels: vec![
                        "L0".into(),
                        "L1".into(),
                        format!("L{}", (index % 3).saturating_add(2)),
                    ],
                })
                .collect();
            StudyConfig {
                schema_version: "1.0".into(),
                study_id: "EXAMPLE-501".into(),
                protocol_version: "1.0".into(),
                arms,
                method: Method::StratifiedBlock {
                    block: BlockScheme::Fixed { size: block_size },
                },
                strata,
                list_length_per_stratum: list_length,
                numbering,
            }
        }
    }
}

fn run_regression_tier() -> TierSection {
    let algo_dirs = find_algo_version_dirs(Path::new(REGRESSION_DIR));
    if algo_dirs.is_empty() {
        return TierSection {
            name: "Regression",
            evidence: REGRESSION_EVIDENCE,
            outcome: TierOutcome::Skip,
            summary: "SKIP — no validation/regression/algo-v* fixture directories found".into(),
            groups: vec![CaseGroup {
                name: "regression-fixtures".into(),
                lines: vec![CaseLine {
                    label: "fixtures".into(),
                    outcome: TierOutcome::Skip,
                    detail: Some("validation/regression/ has no algo-v* directory".into()),
                }],
            }],
        };
    }

    let groups: Vec<CaseGroup> = algo_dirs.iter().map(|dir| regression_group(dir)).collect();
    let outcome = tier_outcome_from_groups(&groups);
    let passed = count_passed(&groups);
    let total = count_total(&groups);

    TierSection {
        name: "Regression",
        evidence: REGRESSION_EVIDENCE,
        outcome,
        summary: format!("{passed}/{total} regression fixtures matched frozen expectations"),
        groups,
    }
}

/// Load and check every fixture in one `algo-vN/` directory.
fn regression_group(dir: &Path) -> CaseGroup {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("algo-v?")
        .to_string();

    let lines = match load_regression_cases(dir) {
        Ok(cases) => cases
            .iter()
            .map(|case| {
                let outcome = match generate(&case.config, case.seed) {
                    Ok(list) => check_regression_case(case, &list),
                    Err(err) => RegressionOutcome {
                        case_id: case.case_id.clone(),
                        passed: false,
                        failures: vec![format!("generate failed: {err}")],
                    },
                };
                CaseLine {
                    label: outcome.case_id,
                    outcome: if outcome.passed {
                        TierOutcome::Pass
                    } else {
                        TierOutcome::Fail
                    },
                    detail: (!outcome.failures.is_empty()).then(|| outcome.failures.join("; ")),
                }
            })
            .collect(),
        Err(err) => vec![CaseLine {
            label: name.clone(),
            outcome: TierOutcome::Fail,
            detail: Some(format!("failed to load fixtures: {err}")),
        }],
    };

    CaseGroup { name, lines }
}

/// `algo-v*` subdirectories directly inside `dir`, sorted by name.
fn find_algo_version_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("algo-v"))
        })
        .collect();
    dirs.sort();
    dirs
}

fn tier_outcome_from_groups(groups: &[CaseGroup]) -> TierOutcome {
    let total = count_total(groups);
    let passed = count_passed(groups);
    if total == 0 {
        TierOutcome::Skip
    } else if passed == total {
        TierOutcome::Pass
    } else {
        TierOutcome::Fail
    }
}

fn count_passed(groups: &[CaseGroup]) -> usize {
    groups
        .iter()
        .flat_map(|g| g.lines.iter())
        .filter(|l| l.outcome == TierOutcome::Pass)
        .count()
}

fn count_total(groups: &[CaseGroup]) -> usize {
    groups.iter().map(|g| g.lines.len()).sum()
}

fn emit_human(report: &ValidationReport, format: ReportFormat) -> std::io::Result<()> {
    let text = match format {
        ReportFormat::Markdown => render_markdown(report),
        ReportFormat::Html => render_html(report),
    };
    write_stdout(&text)
}

fn render_markdown(report: &ValidationReport) -> String {
    let mut out = String::from("# ClinRand Validation Report\n\n");
    for tier in &report.tiers {
        let _ = writeln!(out, "## {} tier\n", tier.name);
        let _ = writeln!(out, "**Evidential status:** {}\n", tier.evidence);
        let _ = writeln!(out, "**Summary:** {}\n", tier.summary);
        for group in &tier.groups {
            let _ = writeln!(out, "### {}\n", group.name);
            for line in &group.lines {
                let status = outcome_label(line.outcome);
                if let Some(detail) = &line.detail {
                    let _ = writeln!(out, "- {}: {} ({})", line.label, status, detail);
                } else {
                    let _ = writeln!(out, "- {}: {}", line.label, status);
                }
            }
            out.push('\n');
        }
    }
    out
}

fn render_html(report: &ValidationReport) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>ClinRand Validation Report</title>\n</head>\n<body>\n\
         <h1>ClinRand Validation Report</h1>\n",
    );
    for tier in &report.tiers {
        let _ = write!(
            out,
            "<h2>{} tier</h2>\n<p><strong>Evidential status:</strong> {}</p>\n\
             <p><strong>Summary:</strong> {}</p>\n",
            html_escape(tier.name),
            html_escape(tier.evidence),
            html_escape(&tier.summary)
        );
        for group in &tier.groups {
            let _ = write!(out, "<h3>{}</h3>\n<ul>\n", html_escape(&group.name));
            for line in &group.lines {
                let status = outcome_label(line.outcome);
                let detail = line
                    .detail
                    .as_ref()
                    .map(|d| format!(" ({})", html_escape(d)))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "<li>{}: {}{}</li>",
                    html_escape(&line.label),
                    html_escape(status),
                    detail
                );
            }
            out.push_str("</ul>\n");
        }
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn emit_json(report: &ValidationReport) -> std::io::Result<()> {
    let tiers: Vec<serde_json::Value> = report
        .tiers
        .iter()
        .map(|tier| {
            let groups: Vec<serde_json::Value> = tier
                .groups
                .iter()
                .map(|group| {
                    let cases: Vec<serde_json::Value> = group
                        .lines
                        .iter()
                        .map(|line| {
                            serde_json::json!({
                                "label": line.label,
                                "status": outcome_label(line.outcome),
                                "detail": line.detail,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "name": group.name,
                        "cases": cases,
                    })
                })
                .collect();
            serde_json::json!({
                "name": tier.name,
                "evidence": tier.evidence,
                "summary": tier.summary,
                "status": outcome_label(tier.outcome),
                "groups": groups,
            })
        })
        .collect();

    let ok = !report.tiers.iter().any(|t| t.outcome == TierOutcome::Fail);
    let payload = serde_json::json!({ "ok": ok, "tiers": tiers });
    write_stdout(&format!("{payload}\n"))
}

fn outcome_label(outcome: TierOutcome) -> &'static str {
    match outcome {
        TierOutcome::Pass => "PASS",
        TierOutcome::Fail => "FAIL",
        TierOutcome::Skip => "SKIP",
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

struct DocumentedStream {
    words: Vec<u64>,
    pos: usize,
}

impl U64Draw for DocumentedStream {
    fn next_u64(&mut self) -> u64 {
        let word = self.words[self.pos];
        self.pos = self.pos.saturating_add(1);
        word
    }
}

fn case_ids(json: &str) -> Vec<String> {
    json.match_indices("\"caseId\": \"")
        .map(|(pos, _)| {
            let start = pos.saturating_add("\"caseId\": \"".len());
            let rest = &json[start..];
            let end = rest.find('"').unwrap_or(rest.len());
            rest[..end].to_string()
        })
        .collect()
}

fn case_object<'a>(json: &'a str, case_id: &str) -> Result<&'a str, String> {
    let needle = format!("\"caseId\": \"{case_id}\"");
    let id_pos = json
        .find(&needle)
        .ok_or_else(|| format!("missing case {case_id}"))?;
    let obj_start = json[..id_pos]
        .rfind('{')
        .ok_or_else(|| format!("unterminated case {case_id}"))?;
    Ok(json_bracketed(&json[obj_start..], '{', '}'))
}

fn json_str(object: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{key}\": \"");
    let start = object
        .find(&needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| format!("missing string field {key}"))?;
    let end = object[start..]
        .find('"')
        .ok_or_else(|| format!("unterminated string field {key}"))?;
    Ok(object[start..start.saturating_add(end)].to_string())
}

fn json_optional_str(object: &str, key: &str) -> Result<Option<String>, String> {
    let needle = format!("\"{key}\": \"");
    let Some(start) = object.find(&needle) else {
        return Ok(None);
    };
    let value_start = start.saturating_add(needle.len());
    let end = object[value_start..]
        .find('"')
        .ok_or_else(|| format!("unterminated string field {key}"))?;
    Ok(Some(
        object[value_start..value_start.saturating_add(end)].to_string(),
    ))
}

fn json_u32(object: &str, key: &str) -> Result<u32, String> {
    let value = json_u64(object, key)?;
    u32::try_from(value).map_err(|_| format!("field {key} does not fit u32"))
}

fn json_u64(object: &str, key: &str) -> Result<u64, String> {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| format!("missing integer field {key}"))?;
    object[start..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .ok_or_else(|| format!("empty integer field {key}"))?
        .parse()
        .map_err(|err| format!("invalid integer field {key}: {err}"))
}

fn json_object<'a>(parent: &'a str, key: &str) -> Result<&'a str, String> {
    let needle = format!("\"{key}\": ");
    let start = parent
        .find(&needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| format!("missing object field {key}"))?;
    Ok(json_bracketed(parent[start..].trim_start(), '{', '}'))
}

fn json_bracketed(rest: &str, open: char, close: char) -> &str {
    let bytes = rest.as_bytes();
    let mut depth = 0usize;
    for (offset, ch) in rest.char_indices() {
        if ch == open {
            depth = depth.saturating_add(1);
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let end = offset.saturating_add(close.len_utf8());
                return std::str::from_utf8(&bytes[..end]).expect("json slice is utf-8");
            }
        }
    }
    panic!("unterminated bracketed value");
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("hex length must be even".into());
    }
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair).map_err(|_| "hex is not utf-8".to_string())?;
            u8::from_str_radix(s, 16).map_err(|err| format!("invalid hex {s}: {err}"))
        })
        .collect()
}

fn hex_array<const N: usize>(hex: &str) -> Result<[u8; N], String> {
    let bytes = decode_hex(hex)?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected {N} bytes, got {}", v.len()))
}

fn parse_purpose(name: &str) -> Result<DrawPurpose, String> {
    match name {
        "BlockSize" => Ok(DrawPurpose::BlockSize),
        "Permutation" => Ok(DrawPurpose::Permutation),
        "SimpleAllocation" => Ok(DrawPurpose::SimpleAllocation),
        other => Err(format!("unknown purpose {other}")),
    }
}

fn json_ident(object: &str, key: &str) -> Result<String, String> {
    json_optional_str(object, key)?.ok_or_else(|| format!("missing string field {key}"))
}

fn json_stream(expect: &str) -> Result<Vec<StreamDraw>, String> {
    let needle = "\"stream\": ";
    let start = expect
        .find(needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| "missing stream array".to_string())?;
    let rest = expect[start..].trim_start();
    if rest.starts_with("[]") {
        return Ok(Vec::new());
    }
    let array = json_bracketed(rest, '[', ']');
    let mut draws = Vec::new();
    let mut search = array;
    while let Some(rel) = search.find('{') {
        let obj = json_bracketed(&search[rel..], '{', '}');
        draws.push(StreamDraw {
            index: json_u64(obj, "index")?,
            bound: json_u64(obj, "bound")?,
            value: json_u64(obj, "value")?,
            purpose: parse_purpose(&json_ident(obj, "purpose")?)?,
        });
        search = &search[rel.saturating_add(obj.len())..];
    }
    Ok(draws)
}

fn json_u64_strings(object: &str, key: &str) -> Result<Vec<u64>, String> {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| format!("missing array field {key}"))?;
    let array = json_bracketed(object[start..].trim_start(), '[', ']');
    array
        .split(',')
        .filter_map(|part| {
            let part = part.trim().trim_matches(|c| c == '[' || c == ']');
            let part = part.trim().trim_matches('"').trim();
            if part.is_empty() {
                None
            } else {
                Some(
                    part.parse::<u64>()
                        .map_err(|err| format!("invalid u64 {part}: {err}")),
                )
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

fn json_i64_array(object: &str, key: &str) -> Result<Vec<i64>, String> {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i.saturating_add(needle.len()))
        .ok_or_else(|| format!("missing array field {key}"))?;
    let array = json_bracketed(object[start..].trim_start(), '[', ']');
    let inner = array.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<i64>()
                .map_err(|err| format!("invalid i64 {}: {err}", part.trim()))
        })
        .collect()
}
