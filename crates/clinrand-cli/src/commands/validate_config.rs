//! `validate-config` subcommand — read and validate a study configuration file.

use clinrand_core::{validate_config, ConfigError, ConfigWarning, StudyConfig, ValidateOptions};

use crate::exit::ExitCode;
use crate::output::{write_stderr, write_stdout};

/// Validate `config_path` and print results in human or JSON form.
pub fn run(json: bool, config_path: &str, allow_large_strata: bool) -> ExitCode {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(err) => {
            let message = format!("failed to read {config_path}: {err}\n");
            let _ = write_stderr(&message);
            return ExitCode::IoError;
        }
    };

    let cfg: StudyConfig = match serde_json::from_str(&text) {
        Ok(cfg) => cfg,
        Err(err) => {
            let message = format!("failed to parse {config_path}: {err}\n");
            let _ = write_stderr(&message);
            return ExitCode::IoError;
        }
    };

    let options = ValidateOptions { allow_large_strata };
    match validate_config(&cfg, &options) {
        Ok(warnings) => emit_success(json, &warnings),
        Err(errors) => emit_validation_failure(json, &errors),
    }
}

fn emit_success(json: bool, warnings: &[ConfigWarning]) -> ExitCode {
    if json {
        if write_json(true, &[], warnings).is_err() {
            return ExitCode::IoError;
        }
    } else {
        if write_stdout("config valid\n").is_err() {
            return ExitCode::IoError;
        }
        if write_warnings_stderr(warnings).is_err() {
            return ExitCode::IoError;
        }
    }
    ExitCode::Success
}

fn emit_validation_failure(json: bool, errors: &[ConfigError]) -> ExitCode {
    if json {
        if write_json(false, errors, &[]).is_err() {
            return ExitCode::IoError;
        }
    } else if write_errors_stderr(errors).is_err() {
        return ExitCode::IoError;
    }
    ExitCode::InvalidConfig
}

fn write_json(ok: bool, errors: &[ConfigError], warnings: &[ConfigWarning]) -> std::io::Result<()> {
    let error_strings: Vec<String> = errors.iter().map(ToString::to_string).collect();
    let warning_strings: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    let payload = serde_json::json!({
        "ok": ok,
        "errors": error_strings,
        "warnings": warning_strings,
    });
    write_stdout(&format!("{payload}\n"))
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
