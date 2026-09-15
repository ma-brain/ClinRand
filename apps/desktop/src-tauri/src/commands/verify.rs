//! `verify_package` command — checksum and property verification.
//!
//! Thin wrapper over [`clinrand_package::verify_package`]. Reads a package
//! directory, re-hashes its files against `checksums.txt`, and re-runs the
//! property checks (P01–P09) against the stored list without regenerating.
//! The seed is not read or used.

use std::path::Path;

use clinrand_package::verify_package as verify_package_dir;
use serde::Serialize;

/// Serializable verification report for the UI.
#[derive(Clone, Debug, Serialize)]
pub struct VerifyOutcome {
    /// True when both checksums and required properties pass.
    pub ok: bool,
    /// True when every file matches its `checksums.txt` digest.
    pub checksums_ok: bool,
    /// True when all required property checks pass.
    pub properties_ok: bool,
    /// Human-readable checksum failures.
    pub checksum_failures: Vec<String>,
    /// Human-readable required property failures (`Pxx: detail`).
    pub property_failures: Vec<String>,
}

/// Verify a package directory's checksums and list properties.
///
/// # Errors
///
/// Returns a human-readable error string when the package cannot be read or
/// parsed. Checksum and property failures are reported in the outcome, not as
/// errors.
#[tauri::command]
pub fn verify_package(package_dir: String) -> Result<VerifyOutcome, String> {
    let report = verify_package_dir(Path::new(&package_dir))
        .map_err(|err| format!("could not verify package: {err}"))?;

    Ok(VerifyOutcome {
        ok: report.ok(),
        checksums_ok: report.checksums_ok,
        properties_ok: report.properties_ok,
        checksum_failures: report.checksum_failures,
        property_failures: report.property_failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_reports_error_for_missing_package() {
        let missing = std::env::temp_dir().join("clinrand-does-not-exist-xyz");
        let err = verify_package(missing.to_string_lossy().to_string()).unwrap_err();
        assert!(err.contains("could not verify package"));
    }

    #[test]
    fn verify_accepts_a_freshly_generated_package() {
        // Generate a package, then verify it round-trips clean.
        let dir = std::env::temp_dir().join(format!("clinrand-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp out dir");
        let config = r#"{
            "schema_version": "1.0",
            "study_id": "DEMO-778",
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
        let generated = crate::commands::generate::generate_package(
            config.to_string(),
            dir.to_string_lossy().to_string(),
            "tester".to_string(),
            false,
        )
        .expect("generate");

        let outcome = verify_package(generated.package_dir.clone()).expect("verify");
        assert!(outcome.ok, "outcome: {outcome:?}");
        assert!(outcome.checksums_ok);
        assert!(outcome.properties_ok);

        std::fs::remove_dir_all(&dir).ok();
    }
}
