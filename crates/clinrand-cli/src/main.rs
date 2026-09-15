//! Command-line interface for ClinRand.
//!
//! Allocation and hashing logic live in `clinrand-core` and
//! `clinrand-package`. This binary must not reimplement them.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

mod cli;
mod exit;

use std::io::{self, Write};

use clap::Parser;
use cli::{Cli, Command};
use clinrand_core::{ALGO_VERSION, ENGINE_VERSION};
use exit::ExitCode;

fn main() {
    let cli = Cli::parse();
    let code = dispatch(&cli);
    std::process::exit(code.code());
}

fn dispatch(cli: &Cli) -> ExitCode {
    match &cli.command {
        Command::Version => run_version(cli.json),
        Command::ListMethods => run_list_methods(cli.json),
        Command::ValidateConfig { .. }
        | Command::Generate { .. }
        | Command::Reproduce { .. }
        | Command::Verify { .. }
        | Command::ValidationReport { .. } => stub_not_implemented(&cli.command),
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

fn write_stdout(text: &str) -> io::Result<()> {
    io::stdout().write_all(text.as_bytes())
}

fn write_stderr(text: &str) -> io::Result<()> {
    io::stderr().write_all(text.as_bytes())
}
