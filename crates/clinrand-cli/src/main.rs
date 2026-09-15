//! Command-line interface for ClinRand.
//!
//! Allocation and hashing logic live in `clinrand-core` and
//! `clinrand-package`. This binary must not reimplement them.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

mod cli;
mod commands;
mod exit;
mod output;
mod seed;

use clap::Parser;
use cli::{Cli, Command};
use clinrand_core::{ALGO_VERSION, ENGINE_VERSION};
use exit::ExitCode;
use output::{write_stderr, write_stdout};

fn main() {
    assert_eq!(
        clinrand_core::ALGO_VERSION,
        clinrand_package::ALGO_VERSION,
        "clinrand-core and clinrand-package ALGO_VERSION must match"
    );

    let cli = Cli::parse();
    let code = dispatch(&cli);
    std::process::exit(code.code());
}

fn dispatch(cli: &Cli) -> ExitCode {
    match &cli.command {
        Command::Version => run_version(cli.json),
        Command::ListMethods => run_list_methods(cli.json),
        Command::ValidateConfig {
            config,
            allow_large_strata,
        } => commands::validate_config::run(cli.json, config, *allow_large_strata),
        Command::Generate {
            config,
            out,
            operator,
            allow_large_strata,
        } => commands::generate::run(cli.json, config, out, operator, *allow_large_strata),
        Command::Reproduce { manifest, out } => commands::reproduce::run(cli.json, manifest, out),
        Command::Verify { package } => commands::verify::run(cli.json, package),
        Command::ValidationReport { .. } => stub_not_implemented(&cli.command),
    }
}

fn run_version(json: bool) -> ExitCode {
    if json {
        let payload = serde_json::json!({
            "engine_version": ENGINE_VERSION,
            "algo_version": ALGO_VERSION,
        });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return ExitCode::IoError;
        }
    } else {
        let line = format!("clinrand {ENGINE_VERSION} (algo_version {ALGO_VERSION})\n");
        if write_stdout(&line).is_err() {
            return ExitCode::IoError;
        }
    }
    ExitCode::Success
}

fn run_list_methods(json: bool) -> ExitCode {
    const METHODS: [&str; 3] = ["simple", "permuted_block", "stratified_block"];
    if json {
        let payload = serde_json::json!(METHODS);
        if write_stdout(&format!("{payload}\n")).is_err() {
            return ExitCode::IoError;
        }
    } else {
        for method in METHODS {
            if write_stdout(&format!("{method}\n")).is_err() {
                return ExitCode::IoError;
            }
        }
    }
    ExitCode::Success
}

fn stub_not_implemented(command: &Command) -> ExitCode {
    let name = command_name(command);
    let message = format!("clinrand {name}: not yet implemented\n");
    let _ = write_stderr(&message);
    ExitCode::CheckFailure
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::ListMethods => "list-methods",
        Command::ValidateConfig { .. } => "validate-config",
        Command::Generate { .. } => "generate",
        Command::Reproduce { .. } => "reproduce",
        Command::Verify { .. } => "verify",
        Command::ValidationReport { .. } => "validation-report",
        Command::Version => "version",
    }
}
