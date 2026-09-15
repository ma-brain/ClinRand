//! Package verification: checksums and property checks without regenerating.

use std::fs;
use std::path::Path;

use clinrand_core::{check_properties, GeneratedList, StreamLog, StudyConfig};

use crate::canonical::sha256_hex;
use crate::error::PackageError;
use crate::read::{
    parse_blinded_manifest, parse_checksums_txt, parse_list_csv, parse_unblinded_manifest,
};

/// Outcome of [`verify_package`]: checksum and property validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyReport {
    /// True when every file listed in `checksums.txt` matches its digest.
    pub checksums_ok: bool,
    /// True when all required property checks (P01–P09) pass, or trivially
    /// true when [`Self::properties_checked`] is `false`.
    pub properties_ok: bool,
    /// False when `list.csv` is absent because the package's restricted
    /// files are still encrypted in `restricted.age` — property checks need
    /// the plaintext list and are skipped rather than reported as a failure.
    /// Run [`crate::decrypt_package`] first to enable them.
    pub properties_checked: bool,
    /// Human-readable checksum failures (missing file or digest mismatch).
    pub checksum_failures: Vec<String>,
    /// Human-readable required property failures (`Pxx: detail`).
    pub property_failures: Vec<String>,
}

impl VerifyReport {
    /// True when both checksums and required properties pass.
    pub fn ok(&self) -> bool {
        self.checksums_ok && self.properties_ok
    }
}

impl std::fmt::Display for VerifyReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "checksums_ok={} properties_ok={} checksum_failures={} property_failures={}",
            self.checksums_ok,
            self.properties_ok,
            self.checksum_failures.len(),
            self.property_failures.len()
        )
    }
}

/// Verify package file checksums and list properties without regenerating.
///
/// Loads study configuration from `manifest.blinded.json` when present, otherwise
/// from `manifest.unblinded.json` (config field only — seed is not read or used).
/// Parses `list.csv` and runs [`check_properties`] with an empty stream log.
///
/// If `list.csv` is absent because the package was written with `--encrypt`
/// and not yet decrypted (`restricted.age` present instead), property checks
/// are skipped rather than treated as a failure — see
/// [`VerifyReport::properties_checked`].
///
/// # Errors
///
/// I/O failures and parse errors for `checksums.txt`, manifests, or `list.csv`
/// propagate as [`PackageError`]. Checksum and property failures are recorded
/// in the returned report instead.
pub fn verify_package(package_dir: &Path) -> Result<VerifyReport, PackageError> {
    let checksum_failures = verify_checksums(package_dir)?;
    let cfg = load_config(package_dir)?;

    let list_csv_path = package_dir.join("list.csv");
    if !list_csv_path.is_file() && package_dir.join("restricted.age").is_file() {
        return Ok(VerifyReport {
            checksums_ok: checksum_failures.is_empty(),
            properties_ok: true,
            properties_checked: false,
            checksum_failures,
            property_failures: Vec::new(),
        });
    }

    let list_csv = fs::read_to_string(&list_csv_path)?;
    let records = parse_list_csv(&cfg, &list_csv)?;
    let list = GeneratedList {
        records,
        stream: StreamLog::default(),
    };

    let property_failures = collect_property_failures(&list, &cfg);

    Ok(VerifyReport {
        checksums_ok: checksum_failures.is_empty(),
        properties_ok: property_failures.is_empty(),
        properties_checked: true,
        checksum_failures,
        property_failures,
    })
}

fn verify_checksums(package_dir: &Path) -> Result<Vec<String>, PackageError> {
    let checksums_path = package_dir.join("checksums.txt");
    let checksums_text = fs::read_to_string(&checksums_path)?;
    let entries = parse_checksums_txt(&checksums_text)?;

    let mut failures = Vec::new();
    for (relative_path, expected_hex) in entries {
        let file_path = package_dir.join(&relative_path);
        let bytes = match fs::read(&file_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                failures.push(format!("{relative_path}: file missing"));
                continue;
            }
            Err(err) => return Err(PackageError::Io(err)),
        };
        let actual_hex = sha256_hex(&bytes);
        if actual_hex != expected_hex {
            failures.push(format!(
                "{relative_path}: digest mismatch (expected {expected_hex}, got {actual_hex})"
            ));
        }
    }
    Ok(failures)
}

fn load_config(package_dir: &Path) -> Result<StudyConfig, PackageError> {
    let blinded_path = package_dir.join("manifest.blinded.json");
    if blinded_path.is_file() {
        let text = fs::read_to_string(&blinded_path)?;
        return parse_blinded_manifest(&text);
    }

    let unblinded_path = package_dir.join("manifest.unblinded.json");
    let text = fs::read_to_string(&unblinded_path)?;
    parse_unblinded_manifest(&text).map(|manifest| manifest.config)
}

fn collect_property_failures(list: &GeneratedList, cfg: &StudyConfig) -> Vec<String> {
    let report = check_properties(list, cfg);
    report
        .checks
        .into_iter()
        .filter(|check| !check.informational && !check.passed)
        .map(|check| format!("{}: {}", check.id, check.detail))
        .collect()
}
