//! `decrypt_package` command — decrypt a package's `restricted.age` in place.
//!
//! Thin wrapper over [`clinrand_package::decrypt_package`]. Writes `list.csv`,
//! `list.json`, `manifest.unblinded.json`, `stream.csv`, and
//! `unblinded-report.html` directly into the package directory as plaintext.
//! The passphrase is never returned to the frontend, logged, or included in
//! any error string — same rule as the seed.

use std::path::Path;

use clinrand_package::decrypt_package as decrypt_package_dir;

/// Decrypt `restricted.age` in `package_dir` in place.
///
/// # Errors
///
/// Returns a human-readable error string on a wrong passphrase, a corrupt
/// container, or an I/O failure. Never reveals the passphrase.
#[tauri::command]
pub fn decrypt_package(package_dir: String, passphrase: String) -> Result<(), String> {
    decrypt_package_dir(Path::new(&package_dir), &passphrase)
        .map_err(|err| format!("could not decrypt package: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"{
        "schema_version": "1.0",
        "study_id": "DEMO-780",
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

    #[test]
    fn decrypt_round_trips_and_wrong_passphrase_is_rejected() {
        let dir = std::env::temp_dir().join(format!("clinrand-decrypt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp out dir");

        let generated = crate::commands::generate::generate_package(
            CONFIG.to_string(),
            dir.to_string_lossy().to_string(),
            "tester".to_string(),
            false,
            Some("decrypt command test passphrase".to_string()),
        )
        .expect("generate encrypted");

        let wrong = decrypt_package(generated.package_dir.clone(), "wrong".to_string());
        assert!(wrong.is_err());
        assert!(
            !wrong
                .unwrap_err()
                .contains("decrypt command test passphrase"),
            "error must not leak the passphrase"
        );
        assert!(
            !Path::new(&generated.package_dir).join("list.csv").exists(),
            "a failed decrypt must not leave list.csv on disk"
        );

        decrypt_package(
            generated.package_dir.clone(),
            "decrypt command test passphrase".to_string(),
        )
        .expect("decrypt");
        assert!(Path::new(&generated.package_dir).join("list.csv").is_file());
        assert!(Path::new(&generated.package_dir)
            .join("unblinded-report.html")
            .is_file());

        std::fs::remove_dir_all(&dir).ok();
    }
}
