//! `generate_package` command — draw a seed, allocate, and write a package.
//!
//! Mirrors the CLI `generate` flow (`crates/clinrand-cli/src/commands/generate.rs`
//! plus `seed.rs`). Allocation, hashing, and canonicalization stay in the
//! engine crates; this command only orchestrates them.
//!
//! ## Seed handling (AGENTS.md §3, §4.9)
//!
//! The 256-bit seed is drawn here, at the desktop seed-draw site
//! ([`crate::seed::draw_seed`]), and passed straight into
//! [`clinrand_core::generate_with_options`] and [`clinrand_package::write_package`],
//! which writes it only to `manifest.unblinded.json` on disk. The seed is
//! **never** returned to the frontend, logged, or included in any error string.

use std::path::Path;

use chrono::Utc;
use clinrand_core::{generate_with_options, validate_config, GenerationError, StudyConfig, ValidateOptions};
use clinrand_package::{render_list_csv, sha256_hex, write_package, PackageError, PackageMeta};
use serde::Serialize;

use crate::seed::draw_seed;

/// Result of a successful package generation. Contains no seed material.
#[derive(Clone, Debug, Serialize)]
pub struct GenerateOutcome {
    /// Absolute path of the written package directory.
    pub package_dir: String,
    /// SHA-256 (lowercase hex) of the canonical `list.csv` bytes.
    pub list_sha256: String,
    /// Number of allocation records in the list.
    pub record_count: u64,
    /// Non-fatal validation warnings surfaced to the operator.
    pub warnings: Vec<String>,
}

/// Generate a randomization package into `out_dir`.
///
/// Parses and validates `config_json`, draws a fresh seed (the only entropy
/// draw in this crate), allocates the list, and writes the package. Returns
/// the package path, `list_sha256`, record count, and warnings.
///
/// # Errors
///
/// Returns a human-readable error string on parse, validation, generation, or
/// I/O failure. Error strings never contain the seed (see module docs).
#[tauri::command]
pub fn generate_package(
    config_json: String,
    out_dir: String,
    operator: String,
    allow_large_strata: bool,
) -> Result<GenerateOutcome, String> {
    let cfg: StudyConfig = serde_json::from_str(&config_json)
        .map_err(|err| format!("could not parse config JSON: {err}"))?;

    let options = ValidateOptions { allow_large_strata };
    let warnings = match validate_config(&cfg, &options) {
        Ok(warnings) => warnings,
        Err(errors) => {
            let joined = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("config validation failed: {joined}"));
        }
    };

    ensure_out_dir(&out_dir)?;

    // Desktop seed-draw site. The seed lives only in this function scope until
    // `write_package` records it to `manifest.unblinded.json`.
    let seed = draw_seed().map_err(|err| format!("failed to draw seed: {err}"))?;

    let list = match generate_with_options(&cfg, seed, &options) {
        Ok(list) => list,
        Err(GenerationError::InvalidConfig(errors)) => {
            let joined = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("config validation failed: {joined}"));
        }
        Err(err) => return Err(format!("generation failed: {err}")),
    };

    let generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let meta = PackageMeta::new(operator, &generated_at).map_err(package_error_message)?;

    let record_count = u64::try_from(list.records.len()).unwrap_or(u64::MAX);
    let list_sha256 = render_list_csv(&cfg, &list)
        .map(|csv| sha256_hex(csv.as_bytes()))
        .map_err(package_error_message)?;

    let package_dir = write_package(Path::new(&out_dir), &cfg, &list, &seed, &meta)
        .map_err(package_error_message)?;

    Ok(GenerateOutcome {
        package_dir: package_dir.display().to_string(),
        list_sha256,
        record_count,
        warnings: warnings.iter().map(ToString::to_string).collect(),
    })
}

/// Ensure `out_dir` is (or can be) a package parent directory.
///
/// Mirrors the CLI's `ensure_out_dir`: accepts an existing directory or a
/// not-yet-created directory whose parent exists.
fn ensure_out_dir(out_dir: &str) -> Result<(), String> {
    let path = Path::new(out_dir);
    if path.is_dir() {
        return Ok(());
    }
    if path.exists() {
        return Err(format!("output path is not a directory: {out_dir}"));
    }
    let Some(parent) = path.parent() else {
        return Err(format!("output directory parent does not exist: {out_dir}"));
    };
    if parent.as_os_str().is_empty() || parent.is_dir() {
        return Ok(());
    }
    Err(format!("output directory parent does not exist: {out_dir}"))
}

/// Render a [`PackageError`] as a UI message. `PackageError`'s `Display` never
/// includes the seed (documented on `write_package`).
fn package_error_message(err: PackageError) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"{
        "schema_version": "1.0",
        "study_id": "DEMO-777",
        "protocol_version": "1.0",
        "arms": [
            { "code": "A", "label": "Active", "ratio": 1 },
            { "code": "P", "label": "Placebo", "ratio": 1 }
        ],
        "method": "permuted_block",
        "block": { "kind": "fixed", "size": 4 },
        "strata": [],
        "list_length_per_stratum": 8,
        "numbering": { "kind": "global", "start": 10001, "width": 5 }
    }"#;

    #[test]
    fn generate_writes_package_and_returns_hash_without_seed() {
        let dir = std::env::temp_dir().join(format!("clinrand-gen-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp out dir");

        let outcome = generate_package(
            CONFIG.to_string(),
            dir.to_string_lossy().to_string(),
            "tester".to_string(),
            false,
        )
        .expect("generate");

        assert_eq!(outcome.record_count, 8);
        assert_eq!(outcome.list_sha256.len(), 64);
        assert!(Path::new(&outcome.package_dir).is_dir());
        // The unblinded manifest holds the seed on disk; the command output
        // must not. Serialize the outcome and confirm no `seed` key leaks.
        let json = serde_json::to_string(&outcome).expect("serialize outcome");
        assert!(!json.contains("seed"), "outcome must not mention seed");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn generate_rejects_invalid_config() {
        let dir = std::env::temp_dir();
        let bad = r#"{
            "schema_version": "1.0",
            "study_id": "TEST-1",
            "protocol_version": "1.0",
            "arms": [ { "code": "A", "label": "Active", "ratio": 1 } ],
            "method": "simple",
            "strata": [],
            "list_length_per_stratum": 4,
            "numbering": { "kind": "global", "start": 1, "width": 4 }
        }"#;
        let err = generate_package(
            bad.to_string(),
            dir.to_string_lossy().to_string(),
            "tester".to_string(),
            false,
        )
        .unwrap_err();
        assert!(err.contains("validation failed"), "got: {err}");
    }

    #[test]
    fn generate_rejects_invalid_json() {
        let dir = std::env::temp_dir();
        let err = generate_package(
            "not json".to_string(),
            dir.to_string_lossy().to_string(),
            "tester".to_string(),
            false,
        )
        .unwrap_err();
        assert!(err.contains("could not parse"));
    }
}
