//! `verify` subcommand — checksum and property validation without regenerating.

use std::path::Path;

use clinrand_package::{verify_package, PackageError, VerifyReport};

use crate::exit::ExitCode;
use crate::output::{write_stderr, write_stdout};

/// Verify a package directory's checksums and list properties.
pub fn run(json: bool, package_path: &str) -> ExitCode {
    let package_dir = Path::new(package_path);
    if !package_dir.is_dir() {
        let message = format!("package path is not a directory: {package_path}\n");
        let _ = write_stderr(&message);
        return ExitCode::IoError;
    }

    match verify_package(package_dir) {
        Ok(report) => emit_report(json, &report),
        Err(err) => emit_verify_error(&err),
    }
}

fn emit_report(json: bool, report: &VerifyReport) -> ExitCode {
    if json {
        let payload = serde_json::json!({
            "ok": report.ok(),
            "checksums_ok": report.checksums_ok,
            "properties_ok": report.properties_ok,
            "properties_checked": report.properties_checked,
            "checksum_failures": report.checksum_failures,
            "property_failures": report.property_failures,
        });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return ExitCode::IoError;
        }
    } else if report.ok() {
        let line = if report.properties_checked {
            "verify ok\n"
        } else {
            "verify ok (properties not checked: package is still encrypted; run `decrypt` first)\n"
        };
        if write_stdout(line).is_err() {
            return ExitCode::IoError;
        }
    } else {
        for failure in &report.checksum_failures {
            if write_stderr(&format!("checksum: {failure}\n")).is_err() {
                return ExitCode::IoError;
            }
        }
        for failure in &report.property_failures {
            if write_stderr(&format!("property: {failure}\n")).is_err() {
                return ExitCode::IoError;
            }
        }
    }

    if report.ok() {
        ExitCode::Success
    } else {
        ExitCode::CheckFailure
    }
}

fn emit_verify_error(err: &PackageError) -> ExitCode {
    let message = format!("{err}\n");
    let _ = write_stderr(&message);
    match err {
        PackageError::Io(_) => ExitCode::IoError,
        PackageError::ListCsvParse { .. }
        | PackageError::ListCsvHeaderMismatch
        | PackageError::ListCsvNoDataRows
        | PackageError::ListCsvColumnCount { .. }
        | PackageError::ListCsvInvalidInteger { .. }
        | PackageError::ChecksumsParse { .. }
        | PackageError::ChecksumsInvalidDigest { .. }
        | PackageError::ChecksumsDuplicatePath { .. }
        | PackageError::ManifestJsonParse
        | PackageError::BlindedManifestJsonParse => ExitCode::IoError,
        _ => ExitCode::CheckFailure,
    }
}
