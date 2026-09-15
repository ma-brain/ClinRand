//! Render the study-specific `qc.R` reviewer script (plan §9).
//!
//! The R template lives in `templates/qc.R` and is embedded at compile time.
//! [`render_qc_r`] interpolates a study-specific header (concrete config values
//! for the human reviewer plus a handful of `EXPECTED_*` constants the script
//! cross-checks against the manifest). All reconstruction, hashing, and
//! property logic lives in the R template — this function only fills the header.
//!
//! No allocation logic lives here; the script rebuilds assignments from the
//! recorded `stream.csv`, never reimplementing ChaCha20. The seed is never
//! interpolated and never referenced by the script.

use clinrand_core::{BlockScheme, Method, NumberingScheme, StudyConfig};

use crate::error::PackageError;

/// Marker line in `templates/qc.R` replaced by the interpolated header block.
const HEADER_MARKER: &str = "# @@CLINRAND_HEADER@@";

/// The embedded R QC template.
const TEMPLATE: &str = include_str!("../templates/qc.R");

/// Render `qc.R` for `cfg`.
///
/// The returned string is a complete R script (base R + `jsonlite` + `digest`)
/// that a reviewer runs with `Rscript qc.R` from the package directory. It
/// reconstructs every assignment from `stream.csv`, re-derives `config_sha256`
/// with the same canonical-JSON rules as [`crate::config_sha256`], verifies the
/// `list.csv` / `stream.csv` content hashes, and re-checks P01–P09.
///
/// # Errors
///
/// Currently infallible for any valid [`StudyConfig`]; returns [`Result`] for
/// forward compatibility. [`PackageError`] [`Display`](std::fmt::Display) never
/// includes the seed.
pub fn render_qc_r(cfg: &StudyConfig) -> Result<String, PackageError> {
    let header = build_header(cfg);
    Ok(TEMPLATE.replace(HEADER_MARKER, &header))
}

fn method_name(method: &Method) -> &'static str {
    match method {
        Method::Simple => "simple",
        Method::PermutedBlock { .. } => "permuted_block",
        Method::StratifiedBlock { .. } => "stratified_block",
    }
}

fn build_header(cfg: &StudyConfig) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# Study:            {}\n",
        comment_safe(&cfg.study_id)
    ));
    out.push_str(&format!(
        "# Protocol:         {}\n",
        comment_safe(&cfg.protocol_version)
    ));
    out.push_str(&format!(
        "# Method:           {}\n",
        method_name(&cfg.method)
    ));

    out.push_str("# Arms (config order):\n");
    for arm in &cfg.arms {
        out.push_str(&format!(
            "#   - code={} label={} ratio={}\n",
            comment_safe(&arm.code),
            comment_safe(&arm.label),
            arm.ratio
        ));
    }

    match &cfg.method {
        Method::Simple => {
            out.push_str("# Block scheme:      (none; simple randomization)\n");
        }
        Method::PermutedBlock { block } | Method::StratifiedBlock { block } => match block {
            BlockScheme::Fixed { size } => {
                out.push_str(&format!("# Block scheme:      fixed size {size}\n"));
            }
            BlockScheme::Variable { sizes } => {
                let joined = sizes
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("# Block scheme:      variable sizes [{joined}]\n"));
            }
        },
    }

    if cfg.strata.is_empty() {
        out.push_str("# Strata:            (none)\n");
    } else {
        out.push_str("# Strata (config order, last factor varies fastest):\n");
        for factor in &cfg.strata {
            let levels = factor
                .levels
                .iter()
                .map(|l| comment_safe(l))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "#   - {}: {}\n",
                comment_safe(&factor.name),
                levels
            ));
        }
    }

    out.push_str(&format!(
        "# List length/stratum: {}\n",
        cfg.list_length_per_stratum
    ));
    match &cfg.numbering {
        NumberingScheme::Global { start, width } => {
            out.push_str(&format!(
                "# Numbering:         global start={start} width={width}\n"
            ));
        }
        NumberingScheme::PerStratumRange {
            start,
            block_size,
            width,
        } => {
            out.push_str(&format!(
                "# Numbering:         per_stratum_range start={start} block_size={block_size} width={width}\n"
            ));
        }
    }

    out.push_str("#\n");
    out.push_str("# Cross-checked against manifest.unblinded.json:\n");
    out.push_str(&format!(
        "EXPECTED_STUDY_ID <- {}\n",
        r_string_lit(&cfg.study_id)
    ));
    out.push_str(&format!(
        "EXPECTED_PROTOCOL_VERSION <- {}\n",
        r_string_lit(&cfg.protocol_version)
    ));
    out.push_str(&format!(
        "EXPECTED_METHOD <- {}\n",
        r_string_lit(method_name(&cfg.method))
    ));
    out.push_str(&format!(
        "EXPECTED_LIST_LENGTH_PER_STRATUM <- {}",
        cfg.list_length_per_stratum
    ));

    out
}

/// Sanitize a value for a single-line `#` comment (strip CR/LF; keep it inert).
fn comment_safe(s: &str) -> String {
    s.chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect()
}

/// Emit an R double-quoted string literal with backslash/quote/newline escaped.
fn r_string_lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinrand_core::{Arm, StratificationFactor};

    fn demo_201() -> StudyConfig {
        StudyConfig {
            schema_version: "1.0".into(),
            study_id: "DEMO-201".into(),
            protocol_version: "2.1".into(),
            arms: vec![
                Arm {
                    code: "A".into(),
                    label: "Investigational product 50 mg".into(),
                    ratio: 2,
                },
                Arm {
                    code: "P".into(),
                    label: "Placebo".into(),
                    ratio: 1,
                },
            ],
            method: Method::StratifiedBlock {
                block: BlockScheme::Variable { sizes: vec![6, 9] },
            },
            strata: vec![
                StratificationFactor {
                    name: "site".into(),
                    levels: vec!["001".into(), "002".into(), "003".into()],
                },
                StratificationFactor {
                    name: "agegrp".into(),
                    levels: vec!["LT65".into(), "GE65".into()],
                },
            ],
            list_length_per_stratum: 36,
            numbering: NumberingScheme::Global {
                start: 10001,
                width: 5,
            },
        }
    }

    #[test]
    fn render_embeds_study_fields_and_constants() {
        let script = render_qc_r(&demo_201()).expect("render");
        assert!(script.contains("EXPECTED_STUDY_ID <- \"DEMO-201\""));
        assert!(script.contains("EXPECTED_PROTOCOL_VERSION <- \"2.1\""));
        assert!(script.contains("EXPECTED_METHOD <- \"stratified_block\""));
        assert!(script.contains("EXPECTED_LIST_LENGTH_PER_STRATUM <- 36"));
        // arm codes and labels appear in the header comment
        assert!(script.contains("code=A"));
        assert!(script.contains("code=P"));
        assert!(script.contains("Investigational product 50 mg"));
        // strata factors appear
        assert!(script.contains("site: 001, 002, 003"));
        assert!(script.contains("agegrp: LT65, GE65"));
        // variable block sizes appear
        assert!(script.contains("variable sizes [6, 9]"));
        // marker fully consumed
        assert!(!script.contains(HEADER_MARKER));
    }

    #[test]
    fn render_marks_simple_without_block() {
        let mut cfg = demo_201();
        cfg.method = Method::Simple;
        cfg.strata = vec![];
        let script = render_qc_r(&cfg).expect("render");
        assert!(script.contains("EXPECTED_METHOD <- \"simple\""));
        assert!(script.contains("(none; simple randomization)"));
        assert!(script.contains("# Strata:            (none)"));
    }

    #[test]
    fn r_string_lit_escapes_quotes_and_backslashes() {
        assert_eq!(r_string_lit("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
}
