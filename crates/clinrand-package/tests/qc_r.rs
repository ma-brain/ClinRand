//! `render_qc_r` embeds study fields, and a rendered `qc.R` QCs a real package.
//!
//! `write_package` emits `qc.R`; these tests run `Rscript qc.R` from the package
//! directory to assert PASS/FAIL behaviour.
//!
//! Synthetic DEMO study IDs only. The seed appears on disk only in
//! `manifest.unblinded.json`; `qc.R` never prints it.

use std::path::Path;
use std::process::{Command, Output};

use clinrand_core::{generate, StudyConfig};
use clinrand_package::{config_sha256, render_qc_r, write_package, PackageMeta};

const SIMPLE_JSON: &str = include_str!("../../../examples/simple.json");
const PB_FIXED_JSON: &str = include_str!("../../../examples/permuted-block-fixed.json");
const STRAT_VARIABLE_JSON: &str = include_str!("../../../examples/stratified-block-variable.json");
const PER_STRATUM_RANGE_JSON: &str = include_str!("../../../examples/per-stratum-range.json");

/// DEMO-201 canonical `config_sha256` (docs/output-package.md §5).
const DEMO_201_CONFIG_SHA256: &str =
    "1364cea5ed27222f7d53130d10dbe7d94eb70b79e5fe0e2d007d2aa9979f01be";

fn seed() -> [u8; 32] {
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    s
}

fn meta() -> PackageMeta {
    PackageMeta::new("QC Test Operator", "2026-09-15T14:42:10Z").expect("meta")
}

fn parse(json: &str) -> StudyConfig {
    serde_json::from_str(json).expect("example config parses")
}

/// Run `Rscript qc.R` in `package_dir`. Panics with an install hint if Rscript
/// is unavailable (CI and the owner have R + jsonlite + digest; not skippable).
fn run_qc(package_dir: &Path) -> Output {
    match Command::new("Rscript")
        .arg("qc.R")
        .current_dir(package_dir)
        .output()
    {
        Ok(out) => out,
        Err(err) => panic!(
            "failed to run `Rscript qc.R` ({err}). Install R plus the jsonlite and digest \
             packages: install.packages(c(\"jsonlite\", \"digest\"))"
        ),
    }
}

/// Write a package for `cfg`/`seed` (includes `qc.R` from `write_package`).
fn write_package_with_qc(cfg: &StudyConfig, dir: &Path) -> std::path::PathBuf {
    let list = generate(cfg, seed()).expect("generate");
    write_package(dir, cfg, &list, &seed(), &meta()).expect("write_package")
}

fn assert_pass(cfg: &StudyConfig) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_package_with_qc(cfg, tmp.path());

    let out = run_qc(&package_dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "qc.R must exit 0 for {}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        cfg.study_id
    );
    assert!(
        stdout.contains("OVERALL: PASS"),
        "qc.R must report OVERALL: PASS for {}\n{stdout}",
        cfg.study_id
    );
    // The seed must never be printed.
    let seed_hex: String = seed().iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        !stdout.contains(&seed_hex) && !stderr.contains(&seed_hex),
        "qc.R output must not contain the seed"
    );
}

#[test]
fn render_embeds_study_fields() {
    let cfg = parse(STRAT_VARIABLE_JSON);
    let script = render_qc_r(&cfg).expect("render");
    assert!(script.contains("EXPECTED_STUDY_ID <- \"DEMO-201\""));
    assert!(script.contains("EXPECTED_METHOD <- \"stratified_block\""));
    assert!(script.contains("code=A"));
    assert!(script.contains("variable sizes [6, 9]"));
    assert!(script.contains("site: 001, 002, 003"));
}

#[test]
fn simple_package_passes_qc() {
    assert_pass(&parse(SIMPLE_JSON));
}

#[test]
fn permuted_block_fixed_package_passes_qc() {
    assert_pass(&parse(PB_FIXED_JSON));
}

#[test]
fn stratified_block_variable_package_passes_qc() {
    let cfg = parse(STRAT_VARIABLE_JSON);
    // Cross-check: this fixture's config hash is the documented DEMO-201 value,
    // so a PASS on config_sha256 in R proves R canonical JSON matches Rust.
    assert_eq!(
        config_sha256(&cfg).expect("config_sha256"),
        DEMO_201_CONFIG_SHA256
    );
    assert_pass(&cfg);
}

#[test]
fn per_stratum_range_package_passes_qc() {
    assert_pass(&parse(PER_STRATUM_RANGE_JSON));
}

#[test]
fn corrupted_list_csv_fails_qc() {
    let cfg = parse(PB_FIXED_JSON);
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = write_package_with_qc(&cfg, tmp.path());

    // Flip the arm code on the first data row without touching stream.csv, so
    // reconstruction (and the list.csv hash) must disagree.
    let list_path = package_dir.join("list.csv");
    let original = std::fs::read_to_string(&list_path).expect("read list.csv");
    let mut lines: Vec<String> = original.split_inclusive('\n').map(str::to_owned).collect();
    let row = 1; // line 0 is the header
    let flipped = {
        let line = lines[row].trim_end_matches('\n');
        let mut fields: Vec<&str> = line.split(',').collect();
        let last = fields.len() - 1;
        let new_arm = if fields[last] == "A" { "P" } else { "A" };
        fields[last] = new_arm;
        format!("{}\n", fields.join(","))
    };
    lines[row] = flipped;
    std::fs::write(&list_path, lines.concat()).expect("write corrupted list.csv");

    let out = run_qc(&package_dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "qc.R must exit non-zero on a corrupted list.csv\n{stdout}"
    );
    assert!(
        stdout.contains("OVERALL: FAIL"),
        "qc.R must report OVERALL: FAIL on a corrupted list.csv\n{stdout}"
    );
}
