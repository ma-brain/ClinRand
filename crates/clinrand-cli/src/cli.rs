//! Clap command-line definition (plan §10).

use clap::{Parser, Subcommand};

/// ClinRand randomization-list generator.
#[derive(Debug, Parser)]
#[command(name = "clinrand", version, about)]
pub struct Cli {
    /// Emit machine-readable JSON on stdout.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// List supported randomization methods.
    #[command(name = "list-methods")]
    ListMethods,

    /// Validate a study configuration file.
    #[command(name = "validate-config")]
    ValidateConfig {
        /// Path to the study configuration JSON file.
        #[arg(long)]
        config: String,
        /// Allow more than 200 stratum combinations.
        #[arg(long)]
        allow_large_strata: bool,
    },

    /// Generate a randomization package.
    Generate {
        /// Path to the study configuration JSON file.
        #[arg(long)]
        config: String,
        /// Output directory for the package.
        #[arg(long)]
        out: String,
        /// Operator name recorded in the package manifest.
        #[arg(long)]
        operator: String,
        /// Allow more than 200 stratum combinations.
        #[arg(long)]
        allow_large_strata: bool,
        /// Encrypt restricted files into `restricted.age` (plan §6.6).
        /// Prompts for a passphrase; no plaintext restricted file is written.
        #[arg(long)]
        encrypt: bool,
    },

    /// Reproduce a list from an unblinded manifest.
    Reproduce {
        /// Path to `manifest.unblinded.json`.
        #[arg(long)]
        manifest: String,
        /// Output directory for the reproduced package.
        #[arg(long)]
        out: String,
        /// Encrypt restricted files into `restricted.age` (plan §6.6).
        /// Prompts for a passphrase; no plaintext restricted file is written.
        #[arg(long)]
        encrypt: bool,
    },

    /// Decrypt a package's `restricted.age` container in place.
    ///
    /// Prompts for the passphrase and writes `list.csv`, `list.json`,
    /// `manifest.unblinded.json`, `stream.csv`, and `unblinded-report.html`
    /// directly into `--package`. Refuses to overwrite any that already exist.
    Decrypt {
        /// Path to the package directory containing `restricted.age`.
        #[arg(long)]
        package: String,
    },

    /// Verify package checksums and properties without regenerating.
    Verify {
        /// Path to the package directory.
        #[arg(long)]
        package: String,
    },

    /// Run validation-tier checks and emit a report.
    #[command(name = "validation-report")]
    ValidationReport {
        /// Validation tier to run (`reference`, `properties`, `regression`, or `all`).
        #[arg(long)]
        tier: Option<String>,
        /// Report format (`md` or `html`).
        #[arg(long)]
        format: Option<String>,
    },

    /// Print engine and algorithm version information.
    Version,
}
