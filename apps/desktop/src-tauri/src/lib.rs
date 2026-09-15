//! ClinRand desktop host.
//!
//! Allocation, hashing, canonicalization, and validation live in
//! `clinrand-core` and `clinrand-package`. This crate only wraps them in Tauri
//! commands (added in later Phase 7 tasks) and must not reimplement them.
//!
//! Only the `dialog` and `fs` plugins are registered. There is deliberately no
//! `http`, `shell`, `process`, or `updater` plugin — the application is
//! provably incapable of transmitting a randomization list.
#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub mod commands;
mod seed;

/// Tauri application entry point, invoked from `main`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::about::get_about,
            commands::config::validate_config_json,
            commands::config::preview_structure,
            commands::generate::generate_package,
            commands::verify::verify_package,
            commands::validation::run_validation_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
