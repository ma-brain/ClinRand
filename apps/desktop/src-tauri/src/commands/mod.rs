//! Tauri command handlers wrapping `clinrand-core` and `clinrand-package`.
//!
//! Every command is a thin orchestration layer: allocation, hashing,
//! canonicalization, and validation live in the engine crates and must not be
//! reimplemented here (AGENTS.md §5, §7). No command returns or logs the seed
//! (AGENTS.md §4.9), and no blinded output includes an arm assignment for a
//! randomization number (AGENTS.md §4.8).

pub mod about;
pub mod config;
pub mod decrypt;
pub mod generate;
pub mod validation;
pub mod verify;

pub use about::get_about;
pub use config::{preview_structure, validate_config_json};
pub use decrypt::decrypt_package;
pub use generate::generate_package;
pub use validation::run_validation_report;
pub use verify::verify_package;
