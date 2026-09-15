//! `run_validation_report` command — in-process validation-tier checks.
//!
//! Adapted from the CLI `validation-report` subcommand
//! (`crates/clinrand-cli/src/commands/validation_report.rs`) per the Phase 7
//! ruling: copy/adapt in-process rather than extract a shared crate. The only
//! substantive differences from the CLI are:
//!
//! * fixture paths resolve relative to this crate's `CARGO_MANIFEST_DIR`
//!   (`apps/desktop/src-tauri`), so `validation/` is `../../../validation`
//!   (three levels up: `src-tauri` -> `desktop` -> `apps` -> repo root);
//! * output is returned as a [`ValidationReportOutcome`] (report text plus a
//!   pass/fail/skip outcome) instead of being written to stdout with an exit
//!   code.
//!
//! The three tiers carry different evidential weight (correctness vs.
//! invariants vs. frozen-output consistency); the evidence strings below must
//! not be conflated (AGENTS.md §7).

use std::fmt::Write as _;
use std::path::Path;

use clinrand_core::{
    check_properties, generate, permute, uniform_below, Arm, BlockScheme, DrawPurpose, Method,
    NumberingScheme, Rng, StratificationFactor, StreamDraw, StreamLog, StudyConfig, U64Draw,
    UniformError, ValidateOptions,
};
use serde::Serialize;

const CHACHA20_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../validation/reference/chacha20/cases.json"
));
const UNIFORM_BELOW_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../validation/reference/uniform-below/cases.json"
));
const FISHER_YATES_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../validation/reference/fisher-yates/cases.json"
));
const REGRESSION_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../validation/regression"
);

const REFERENCE_EVIDENCE: &str = "Correctness against external normative sources (RFC 8439, hand-worked derivations). Expected values are not this engine's own output.";
const PROPERTIES_EVIDENCE: &str = "Invariant evidence only — not external-oracle correctness. A passing sweep means P01–P09 held for sampled configs; it does not prove the algorithm matches an independent reference. Full 1000-case CI suite: `cargo test -p clinrand-core --test properties_proptest`.";
const REGRESSION_EVIDENCE: &str = "Frozen engine output consistency — proves nothing changed since the last approved ALGO_VERSION, not that the algorithm is correct. Regression fixtures arrive in Phase 9.";

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

/// Overall tier outcome.
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

/// Serializable command result: report text plus pass/fail/skip outcome.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReportOutcome {
    /// True when no tier failed.
    pub ok: bool,
    /// `"pass"`, `"fail"`, or `"skip"` for the whole report.
    pub outcome: String,
    /// Rendered report text in the requested format.
    pub report: String,
    /// Echoed report format (`"md"` or `"html"`).
    pub format: String,
}

/// Run validation-tier checks in-process and return a rendered report.
///
/// `tier` selects `reference` / `properties` / `regression` / `all` (default
/// `all`). `format` selects `md` (default) or `html`.
///
/// # Errors
///
/// Returns an error string for an unknown tier or format.
#[tauri::command]
pub fn run_validation_report(
    tier: Option<String>,
    format: Option<String>,
) -> Result<ValidationReportOutcome, String> {
    let tier_filter = parse_tier(tier.as_deref())?;
    let report_format = parse_format(format.as_deref())?;

    let report = build_report(tier_filter);
    let (ok, outcome) = overall_outcome(&report);
    let text = match report_format {
        ReportFormat::Markdown => render_markdown(&report),
        ReportFormat::Html => render_html(&report),
    };

    Ok(ValidationReportOutcome {
        ok,
        outcome: outcome.to_string(),
        report: text,
        format: match report_format {
            ReportFormat::Markdown => "md".to_string(),
            ReportFormat::Html => "html".to_string(),
        },
    })
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

/// Overall pass/fail/skip: any failure fails; else all-skip is skip; else pass.
fn overall_outcome(report: &ValidationReport) -> (bool, &'static str) {
    if report.tiers.iter().any(|t| t.outcome == TierOutcome::Fail) {
        (false, "fail")
    } else if !report.tiers.is_empty()
        && report.tiers.iter().all(|t| t.outcome == TierOutcome::Skip)
    {
        (true, "skip")
    } else {
        (true, "pass")
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
    let fixture_count = count_regression_fixtures(Path::new(REGRESSION_DIR));
    if fixture_count == 0 {
        return TierSection {
            name: "Regression",
            evidence: REGRESSION_EVIDENCE,
            outcome: TierOutcome::Skip,
            summary: "SKIP — 0 fixtures (regression fixtures arrive in Phase 9)".into(),
            groups: vec![CaseGroup {
                name: "regression-fixtures".into(),
                lines: vec![CaseLine {
                    label: "fixtures".into(),
                    outcome: TierOutcome::Skip,
                    detail: Some(
                        "validation/regression/ is missing or empty; frozen output checks deferred to Phase 9"
                            .into(),
                    ),
                }],
            }],
        };
    }

    TierSection {
        name: "Regression",
        evidence: REGRESSION_EVIDENCE,
        outcome: TierOutcome::Skip,
        summary: format!("SKIP — {fixture_count} fixture dirs present; execution deferred to Phase 9"),
        groups: vec![CaseGroup {
            name: "regression-fixtures".into(),
            lines: vec![CaseLine {
                label: "fixtures".into(),
                outcome: TierOutcome::Skip,
                detail: Some(format!(
                    "{fixture_count} fixture director(ies) found; list_sha256 checks deferred to Phase 9"
                )),
            }],
        }],
    }
}

fn count_regression_fixtures(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("algo-v"))
        .count()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_tier_passes_against_bundled_vectors() {
        let outcome = run_validation_report(Some("reference".to_string()), None).expect("report");
        assert!(outcome.ok, "report: {}", outcome.report);
        assert_eq!(outcome.outcome, "pass");
        assert_eq!(outcome.format, "md");
        assert!(outcome.report.contains("Reference tier"));
    }

    #[test]
    fn regression_tier_skips_when_no_fixtures() {
        let outcome = run_validation_report(Some("regression".to_string()), None).expect("report");
        // No fixtures exist yet (Phase 9); the whole (single-tier) report skips.
        assert_eq!(outcome.outcome, "skip");
        assert!(outcome.ok);
    }

    #[test]
    fn properties_tier_passes_bounded_sweep() {
        let outcome = run_validation_report(Some("properties".to_string()), None).expect("report");
        assert!(outcome.ok, "report: {}", outcome.report);
        assert_eq!(outcome.outcome, "pass");
    }

    #[test]
    fn html_format_is_honored() {
        let outcome =
            run_validation_report(Some("reference".to_string()), Some("html".to_string()))
                .expect("report");
        assert_eq!(outcome.format, "html");
        assert!(outcome.report.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn unknown_tier_is_rejected() {
        let err = run_validation_report(Some("bogus".to_string()), None).unwrap_err();
        assert!(err.contains("unknown validation tier"));
    }
}
