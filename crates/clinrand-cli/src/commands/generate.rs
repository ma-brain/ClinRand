//! `generate` subcommand — draw seed, allocate, and write a package.

use std::path::Path;

use chrono::Utc;
use clinrand_core::{
    generate_with_options, validate_config, ConfigError, ConfigWarning, GenerationError,
    StudyConfig, ValidateOptions,
};
use clinrand_package::{render_list_csv, sha256_hex, write_package, PackageError, PackageMeta};

use crate::exit::ExitCode;
use crate::output::{write_stderr, write_stdout};
use crate::seed::draw_seed;

/// Generate a randomization package from `config_path` into `out_dir`.
pub fn run(
    json: bool,
    config_path: &str,
    out_dir: &str,
    operator: &str,
    allow_large_strata: bool,
) -> ExitCode {
    let cfg = match read_config(config_path) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let options = ValidateOptions { allow_large_strata };
    let warnings = match validate_config(&cfg, &options) {
        Ok(warnings) => warnings,
        Err(errors) => return emit_validation_failure(json, &errors),
    };

    if let Err(code) = ensure_out_dir(out_dir) {
        return code;
    }

    let seed = match draw_seed() {
        Ok(seed) => seed,
        Err(err) => {
            let message = format!("failed to draw seed: {err}\n");
            let _ = write_stderr(&message);
            return ExitCode::IoError;
        }
    };

    let list = match generate_with_options(&cfg, seed, &options) {
        Ok(list) => list,
        Err(GenerationError::InvalidConfig(errors)) => {
            return emit_validation_failure(json, &errors)
        }
        Err(err) => return emit_generation_failure(&err),
    };

    let generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let meta = match PackageMeta::new(operator, &generated_at) {
        Ok(meta) => meta,
        Err(err) => return emit_package_failure(&err),
    };

    let record_count = u64::try_from(list.records.len()).unwrap_or(u64::MAX);
    let list_sha256 = match render_list_csv(&cfg, &list) {
        Ok(csv) => sha256_hex(csv.as_bytes()),
        Err(err) => return emit_package_failure(&err),
    };

    let package_dir = match write_package(Path::new(out_dir), &cfg, &list, &seed, &meta) {
        Ok(path) => path,
        Err(err) => return emit_package_failure(&err),
    };

    if let Err(code) = emit_success(json, &warnings, &package_dir, &list_sha256, record_count) {
        return code;
    }

    ExitCode::Success
}

fn read_config(config_path: &str) -> Result<StudyConfig, ExitCode> {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(err) => {
            let message = format!("failed to read {config_path}: {err}\n");
            let _ = write_stderr(&message);
            return Err(ExitCode::IoError);
        }
    };

    match serde_json::from_str(&text) {
        Ok(cfg) => Ok(cfg),
        Err(err) => {
            let message = format!("failed to parse {config_path}: {err}\n");
            let _ = write_stderr(&message);
            Err(ExitCode::IoError)
        }
    }
}

fn ensure_out_dir(out_dir: &str) -> Result<(), ExitCode> {
    let path = Path::new(out_dir);
    if path.is_dir() {
        return Ok(());
    }
    if path.exists() {
        let message = format!("output path is not a directory: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    }
    let Some(parent) = path.parent() else {
        let message = format!("output directory parent does not exist: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    if parent.is_dir() {
        let message = format!("output directory does not exist: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    }
    let message = format!("output directory parent does not exist: {out_dir}\n");
    let _ = write_stderr(&message);
    Err(ExitCode::IoError)
}

fn emit_validation_failure(json: bool, errors: &[ConfigError]) -> ExitCode {
    if json {
        let error_strings: Vec<String> = errors.iter().map(ToString::to_string).collect();
        let payload = serde_json::json!({
            "ok": false,
            "errors": error_strings,
            "warnings": [],
        });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return ExitCode::IoError;
        }
    } else if write_errors_stderr(errors).is_err() {
        return ExitCode::IoError;
    }
    ExitCode::InvalidConfig
}

fn emit_generation_failure(err: &GenerationError) -> ExitCode {
    let message = format!("generation failed: {err}\n");
    let _ = write_stderr(&message);
    ExitCode::CheckFailure
}

fn emit_package_failure(err: &PackageError) -> ExitCode {
    let message = format!("{err}\n");
    let _ = write_stderr(&message);
    match err {
        PackageError::Io(_) | PackageError::PackageDirExists { .. } => ExitCode::IoError,
        _ => ExitCode::CheckFailure,
    }
}

fn emit_success(
    json: bool,
    warnings: &[ConfigWarning],
    package_dir: &Path,
    list_sha256: &str,
    record_count: u64,
) -> Result<(), ExitCode> {
    if json {
        if write_warnings_stderr(warnings).is_err() {
            return Err(ExitCode::IoError);
        }
        let payload = serde_json::json!({
            "package_dir": package_dir.display().to_string(),
            "list_sha256": list_sha256,
            "record_count": record_count,
        });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return Err(ExitCode::IoError);
        }
    } else {
        if write_stdout(&format!("{}\n", package_dir.display())).is_err() {
            return Err(ExitCode::IoError);
        }
        if write_stdout(&format!("{list_sha256}\n")).is_err() {
            return Err(ExitCode::IoError);
        }
        if write_warnings_stderr(warnings).is_err() {
            return Err(ExitCode::IoError);
        }
    }
    Ok(())
}

fn write_warnings_stderr(warnings: &[ConfigWarning]) -> std::io::Result<()> {
    for warning in warnings {
        write_stderr(&format!("{warning}\n"))?;
    }
    Ok(())
}

fn write_errors_stderr(errors: &[ConfigError]) -> std::io::Result<()> {
    for error in errors {
        write_stderr(&format!("{error}\n"))?;
    }
    Ok(())
}
